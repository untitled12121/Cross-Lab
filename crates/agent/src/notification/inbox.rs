use std::{collections::VecDeque, time::Duration};

use tokio::time::Instant;

use crosslab_policy::SessionId;
use crosslab_protocol::{Event, NotificationPayload, NotificationProfileError, RequestId};

use super::parse_notification_event;

pub const MAX_VISIBLE_NOTIFICATIONS: usize = 64;
const SUBSCRIPTION_RESPONSE_TIMEOUT: Duration = Duration::from_secs(10);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NotificationInboxStatus {
    Idle,
    AwaitingApproval,
    Active,
    Denied,
    TimedOut,
}

#[derive(Clone, PartialEq, Eq)]
pub struct NotificationInboxSnapshot {
    phase: NotificationInboxStatus,
    entries: Vec<NotificationPayload>,
    skipped: u64,
}

impl Default for NotificationInboxSnapshot {
    fn default() -> Self {
        Self {
            phase: NotificationInboxStatus::Idle,
            entries: Vec::new(),
            skipped: 0,
        }
    }
}

impl core::fmt::Debug for NotificationInboxSnapshot {
    fn fmt(&self, formatter: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        formatter
            .debug_struct("NotificationInboxSnapshot")
            .field("phase", &self.phase)
            .field("entry_count", &self.entries.len())
            .field("skipped", &self.skipped)
            .finish()
    }
}

impl NotificationInboxSnapshot {
    pub const fn phase(&self) -> NotificationInboxStatus {
        self.phase
    }

    pub fn entries(&self) -> &[NotificationPayload] {
        &self.entries
    }

    pub const fn skipped_count(&self) -> u64 {
        self.skipped
    }
}

pub struct NotificationInbox {
    status: NotificationInboxStatus,
    pending: Option<(RequestId, SessionId, Instant)>,
    active: Option<SessionId>,
    events: VecDeque<NotificationPayload>,
    skipped: u64,
}

impl Default for NotificationInbox {
    fn default() -> Self {
        Self {
            status: NotificationInboxStatus::Idle,
            pending: None,
            active: None,
            events: VecDeque::new(),
            skipped: 0,
        }
    }
}

impl NotificationInbox {
    pub fn snapshot(&self) -> NotificationInboxSnapshot {
        NotificationInboxSnapshot {
            phase: self.status,
            entries: self.events.iter().cloned().collect(),
            skipped: self.skipped,
        }
    }

    pub const fn status(&self) -> NotificationInboxStatus {
        self.status
    }

    pub fn begin(&mut self, request_id: RequestId, session_id: SessionId) {
        self.reset();
        self.pending = Some((
            request_id,
            session_id,
            Instant::now() + SUBSCRIPTION_RESPONSE_TIMEOUT,
        ));
        self.status = NotificationInboxStatus::AwaitingApproval;
    }

    pub fn complete(
        &mut self,
        request_id: RequestId,
        session_id: SessionId,
        approved: bool,
    ) -> bool {
        if self.pending.map(|(id, session, _)| (id, session))
            != Some((request_id, session_id))
        {
            return false;
        }
        self.pending = None;
        if approved {
            self.active = Some(session_id);
            self.status = NotificationInboxStatus::Active;
        } else {
            self.status = NotificationInboxStatus::Denied;
        }
        true
    }

    pub fn next_deadline(&self) -> Option<Instant> {
        self.pending.map(|(_, _, deadline)| deadline)
    }

    pub fn expire(&mut self, now: Instant) -> Option<RequestId> {
        let (request_id, _, deadline) = self.pending?;
        if now < deadline {
            return None;
        }
        self.pending = None;
        self.status = NotificationInboxStatus::TimedOut;
        Some(request_id)
    }

    pub fn receive(
        &mut self,
        event: &Event,
        session: SessionId,
    ) -> Result<bool, NotificationProfileError> {
        if self.active != Some(session) {
            return Ok(false);
        }
        let Some(payload) = parse_notification_event(event)? else {
            return Ok(false);
        };
        let id = match &payload {
            NotificationPayload::Posted(posted) => posted.id(),
            NotificationPayload::Removed(id) => *id,
        };
        self.events.retain(|old| match old {
            NotificationPayload::Posted(posted) => posted.id() != id,
            NotificationPayload::Removed(old_id) => *old_id != id,
        });
        match payload {
            NotificationPayload::Removed(_) => {}
            other => {
                if self.events.len() == MAX_VISIBLE_NOTIFICATIONS {
                    let _ = self.events.pop_front();
                    self.skipped = self.skipped.saturating_add(1);
                }
                self.events.push_back(other);
            }
        }
        Ok(true)
    }

    pub fn ensure_session(&mut self, session: Option<SessionId>) {
        if self.active.is_some_and(|active| Some(active) != session)
            || self
                .pending
                .is_some_and(|(_, pending, _)| Some(pending) != session)
        {
            self.reset();
        }
    }

    pub fn reset(&mut self) {
        self.pending = None;
        self.active = None;
        self.events.clear();
        self.skipped = 0;
        self.status = NotificationInboxStatus::Idle;
    }

    pub fn entries(&self) -> &VecDeque<NotificationPayload> {
        &self.events
    }

    pub const fn skipped_count(&self) -> u64 {
        self.skipped
    }

    pub const fn is_active(&self) -> bool {
        self.active.is_some()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crosslab_protocol::{NotificationId, NotificationPosted};

    fn posted(id: u8) -> Event {
        let entry = NotificationPayload::Posted(
            NotificationPosted::new(
                NotificationId::from_bytes([id; 16]),
                "App".to_owned(),
                None,
                None,
                true,
            )
            .unwrap(),
        );
        super::super::notification_event(&entry).unwrap()
    }

    #[test]
    fn unsolicited_early_and_replayed_events_cannot_enter_inbox() {
        let mut inbox = NotificationInbox::default();
        let session = SessionId::from_bytes([1; 32]);
        let request = RequestId::from_bytes([2; 16]);
        assert!(!inbox.receive(&posted(3), session).unwrap());
        inbox.begin(request, session);
        assert!(!inbox.receive(&posted(3), session).unwrap());
        assert!(!inbox.complete(request, SessionId::from_bytes([9; 32]), true));
        assert!(inbox.complete(request, session, true));
        assert!(inbox.receive(&posted(3), session).unwrap());
        assert!(
            !inbox
                .receive(&posted(4), SessionId::from_bytes([9; 32]))
                .unwrap()
        );
        inbox.ensure_session(None);
        assert!(inbox.entries().is_empty());
        assert!(!inbox.receive(&posted(3), session).unwrap());
    }

    #[test]
    fn posted_coalesce_removed_and_bounded_capacity() {
        let mut inbox = NotificationInbox::default();
        let session = SessionId::from_bytes([1; 32]);
        let req = RequestId::from_bytes([2; 16]);
        inbox.begin(req, session);
        assert!(inbox.complete(req, session, true));
        for id in 0..70 {
            assert!(inbox.receive(&posted(id), session).unwrap());
        }
        assert_eq!(inbox.entries().len(), MAX_VISIBLE_NOTIFICATIONS);
        assert_eq!(inbox.skipped_count(), 6);
        assert!(inbox.receive(&posted(69), session).unwrap());
        assert_eq!(inbox.entries().len(), MAX_VISIBLE_NOTIFICATIONS);
        let removed = NotificationPayload::Removed(NotificationId::from_bytes([69; 16]));
        assert!(
            inbox
                .receive(
                    &super::super::notification_event(&removed).unwrap(),
                    session
                )
                .unwrap()
        );
        assert_eq!(inbox.entries().len(), MAX_VISIBLE_NOTIFICATIONS - 1);
    }

    #[test]
    fn late_subscription_response_cannot_activate_expired_session() {
        let mut inbox = NotificationInbox::default();
        let session = SessionId::from_bytes([3; 32]);
        let request_id = RequestId::from_bytes([5; 16]);
        inbox.begin(request_id, session);
        let deadline = inbox.next_deadline().unwrap();

        assert_eq!(inbox.expire(deadline - Duration::from_nanos(1)), None);
        assert_eq!(inbox.expire(deadline), Some(request_id));
        assert_eq!(inbox.status(), NotificationInboxStatus::TimedOut);
        assert!(inbox.next_deadline().is_none());
        assert!(!inbox.complete(request_id, session, true));
        assert!(!inbox.receive(&posted(1), session).unwrap());
        assert!(inbox.entries().is_empty());
    }

    #[test]
    fn denied_subscription_displays_no_private_events() {
        let mut inbox = NotificationInbox::default();
        let session = SessionId::from_bytes([1; 32]);
        let request = RequestId::from_bytes([2; 16]);
        inbox.begin(request, session);
        assert!(inbox.complete(request, session, false));
        assert_eq!(inbox.status(), NotificationInboxStatus::Denied);
        assert!(!inbox.receive(&posted(2), session).unwrap());
        assert!(inbox.entries().is_empty());
    }
}
