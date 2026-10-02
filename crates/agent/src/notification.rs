use std::collections::VecDeque;

use crosslab_identity::DeviceId;
use crosslab_policy::{
    AuthorizationContext, CapabilityVersion, DecisionEffect, PolicyState, SessionId, TrustState,
};
use crosslab_protocol::{
    NOTIFICATION_CAPABILITY_ID, NOTIFICATION_SUBSCRIBE_OPERATION, NotificationId,
    NotificationPayload, NotificationPosted, RequestId,
};

#[path = "notification/inbox.rs"]
mod inbox;

pub use inbox::{NotificationInbox, NotificationInboxSnapshot, NotificationInboxStatus};

#[path = "notification/wire.rs"]
mod wire;

pub(crate) use wire::{
    notification_advertisement, notification_capabilities, notification_event,
    notification_event_subscriptions, notification_subscribe_request, parse_notification_event,
    validate_notification_request, validate_notification_response,
};

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum NotificationRole {
    #[default]
    Disabled,
    Source {
        content_enabled: bool,
    },
    Receiver,
}

impl NotificationRole {
    pub const fn enabled(self) -> bool {
        !matches!(self, Self::Disabled)
    }

    pub const fn source_consent(self) -> Option<NotificationConsent> {
        match self {
            Self::Source { content_enabled } => Some(NotificationConsent {
                os_access: true,
                owner_enabled: true,
                content_enabled,
                runtime_active: true,
            }),
            _ => None,
        }
    }
}

pub const NOTIFICATION_QUEUE_CAPACITY: usize = 64;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct NotificationConsent {
    pub os_access: bool,
    pub owner_enabled: bool,
    pub content_enabled: bool,
    pub runtime_active: bool,
}

impl NotificationConsent {
    pub const fn available(self) -> bool {
        self.os_access && self.owner_enabled && self.runtime_active
    }
}

/// Android platform keys are retained only in short-lived process memory.
pub enum PlatformNotification {
    Posted {
        key: String,
        app_label: String,
        title: Option<String>,
        preview: Option<String>,
        protected: bool,
    },
    Removed {
        key: String,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NotificationMirrorError {
    Unavailable,
    NotAuthorized,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Subscription {
    request_id: RequestId,
    session_id: SessionId,
    source_device_id: DeviceId,
    trust_revision: u64,
    policy_revision: u64,
}

#[derive(Default)]
pub struct NotificationMirror {
    subscription: Option<Subscription>,
    pending: VecDeque<NotificationPayload>,
    source_ids: VecDeque<(String, NotificationId)>,
    skipped: u64,
}

impl NotificationMirror {
    /// The caller must supply the verified active session's authorization context.
    pub fn subscribe(
        &mut self,
        request_id: RequestId,
        context: &AuthorizationContext,
        policy: &PolicyState,
        consent: NotificationConsent,
    ) -> Result<(), NotificationMirrorError> {
        self.close();
        if !consent.available() {
            return Err(NotificationMirrorError::Unavailable);
        }
        if !authorized(context, policy) {
            return Err(NotificationMirrorError::NotAuthorized);
        }
        self.subscription = Some(Subscription {
            request_id,
            session_id: context.session_id(),
            source_device_id: context.source_device_id(),
            trust_revision: context.trust_revision(),
            policy_revision: policy.revision(),
        });
        Ok(())
    }

    pub fn cancel_request(&mut self, request_id: RequestId) -> bool {
        let matching = self
            .subscription
            .is_some_and(|active| active.request_id == request_id);
        if matching {
            self.close();
        }
        matching
    }

    /// A policy/session/OS permission change discards all pending private payloads.
    pub fn revalidate(
        &mut self,
        context: &AuthorizationContext,
        policy: &PolicyState,
        consent: NotificationConsent,
    ) -> bool {
        let valid = consent.available()
            && self.subscription.is_some_and(|active| {
                active.session_id == context.session_id()
                    && active.source_device_id == context.source_device_id()
                    && active.trust_revision == context.trust_revision()
                    && active.policy_revision == policy.revision()
            })
            && authorized(context, policy);
        if !valid {
            self.close();
        }
        valid
    }

    pub const fn is_subscribed(&self) -> bool {
        self.subscription.is_some()
    }

    /// Exclude protected/system-private OS notifications before invoking this.
    pub fn submit_platform(
        &mut self,
        event: PlatformNotification,
        context: &AuthorizationContext,
        policy: &PolicyState,
        consent: NotificationConsent,
    ) -> bool {
        if !self.revalidate(context, policy, consent) {
            return false;
        }
        match event {
            PlatformNotification::Posted {
                key,
                app_label,
                title,
                preview,
                protected,
            } => {
                if key.is_empty() || key.len() > 256 {
                    return false;
                }
                let existing = self
                    .source_ids
                    .iter()
                    .find(|(old, _)| old == &key)
                    .map(|(_, id)| *id);
                let id = match existing {
                    Some(id) => id,
                    None => match NotificationId::generate() {
                        Ok(id) => id,
                        Err(_) => return false,
                    },
                };
                let redacted = protected || !consent.content_enabled;
                let (title, preview) = if redacted {
                    (None, None)
                } else {
                    (title, preview)
                };
                let Ok(posted) = NotificationPosted::new(id, app_label, title, preview, redacted)
                else {
                    return false;
                };
                if !self.enqueue(
                    NotificationPayload::Posted(posted),
                    context,
                    policy,
                    consent,
                ) {
                    return false;
                }
                if existing.is_none() {
                    if self.source_ids.len() == NOTIFICATION_QUEUE_CAPACITY {
                        let _ = self.source_ids.pop_front();
                    }
                    self.source_ids.push_back((key, id));
                }
                true
            }
            PlatformNotification::Removed { key } => {
                let Some(position) = self.source_ids.iter().position(|(old, _)| old == &key) else {
                    return false;
                };
                let (_, id) = self
                    .source_ids
                    .remove(position)
                    .expect("bounded position found");
                self.enqueue(NotificationPayload::Removed(id), context, policy, consent)
            }
        }
    }

    /// The platform adapter has already excluded protected/system-private notifications.
    pub fn enqueue(
        &mut self,
        event: NotificationPayload,
        context: &AuthorizationContext,
        policy: &PolicyState,
        consent: NotificationConsent,
    ) -> bool {
        if !self.revalidate(context, policy, consent) {
            return false;
        }
        let event = match event {
            NotificationPayload::Posted(posted) if !consent.content_enabled => {
                let redacted = NotificationPosted::new(
                    posted.id(),
                    posted.app_label().to_owned(),
                    None,
                    None,
                    true,
                )
                .expect("verified label remains bounded after removal of private content");
                NotificationPayload::Posted(redacted)
            }
            other => other,
        };
        let id = match &event {
            NotificationPayload::Posted(posted) => posted.id(),
            NotificationPayload::Removed(id) => *id,
        };
        self.pending.retain(|existing| match existing {
            NotificationPayload::Posted(posted) => posted.id() != id,
            NotificationPayload::Removed(old_id) => *old_id != id,
        });
        if self.pending.len() == NOTIFICATION_QUEUE_CAPACITY {
            let _ = self.pending.pop_front();
            self.skipped = self.skipped.saturating_add(1);
        }
        self.pending.push_back(event);
        true
    }

    pub fn take_next(
        &mut self,
        context: &AuthorizationContext,
        policy: &PolicyState,
        consent: NotificationConsent,
    ) -> Option<NotificationPayload> {
        self.revalidate(context, policy, consent)
            .then(|| self.pending.pop_front())
            .flatten()
    }

    pub const fn skipped_count(&self) -> u64 {
        self.skipped
    }

    pub fn close(&mut self) {
        self.subscription = None;
        self.pending.clear();
        self.source_ids.clear();
        self.skipped = 0;
    }
}

fn authorized(context: &AuthorizationContext, policy: &PolicyState) -> bool {
    context.trust_state() == TrustState::Trusted
        && context.capability_id().as_str() == NOTIFICATION_CAPABILITY_ID
        && context.negotiated_version() == CapabilityVersion::new(3, 0)
        && context.operation().as_str() == NOTIFICATION_SUBSCRIBE_OPERATION
        && policy.evaluate(context).effect() == DecisionEffect::Allow
}

#[cfg(test)]
mod tests {
    use crosslab_identity::DeviceId;
    use crosslab_policy::{
        CapabilityId, CapabilityVersionRange, LocalCapability, NetworkClass, OperationName,
        RuleEffect,
    };
    use crosslab_protocol::NotificationId;

    use super::*;

    fn context(session: u8) -> AuthorizationContext {
        let capability = CapabilityId::parse(NOTIFICATION_CAPABILITY_ID).unwrap();
        AuthorizationContext::new(
            DeviceId::from_bytes([1; 32]),
            DeviceId::from_bytes([2; 32]),
            SessionId::from_bytes([session; 32]),
            capability.clone(),
            CapabilityVersion::new(3, 0),
            OperationName::parse(NOTIFICATION_SUBSCRIBE_OPERATION).unwrap(),
            TrustState::Trusted,
            0,
            LocalCapability::new(
                capability,
                CapabilityVersionRange::new(3, 0, 0).unwrap(),
                true,
            ),
            NetworkClass::Local,
        )
    }

    fn policy(effect: Option<RuleEffect>) -> PolicyState {
        let mut policy = PolicyState::new();
        if let Some(effect) = effect {
            policy
                .set_rule_effect(
                    DeviceId::from_bytes([1; 32]),
                    CapabilityId::parse(NOTIFICATION_CAPABILITY_ID).unwrap(),
                    OperationName::parse(NOTIFICATION_SUBSCRIBE_OPERATION).unwrap(),
                    effect,
                )
                .unwrap();
        }
        policy
    }

    fn consent() -> NotificationConsent {
        NotificationConsent {
            os_access: true,
            owner_enabled: true,
            content_enabled: false,
            runtime_active: true,
        }
    }

    fn request_id() -> RequestId {
        RequestId::from_bytes([0x44; 16])
    }

    fn posted(id: u8) -> NotificationPayload {
        NotificationPayload::Posted(
            NotificationPosted::new(
                NotificationId::from_bytes([id; 16]),
                "Messenger".to_owned(),
                Some("Private subject".to_owned()),
                Some("Private text".to_owned()),
                false,
            )
            .unwrap(),
        )
    }

    #[test]
    fn platform_keys_never_leave_as_identifiers_and_clear_on_close() {
        let mut mirror = NotificationMirror::default();
        let policy = policy(Some(RuleEffect::Allow));
        mirror
            .subscribe(request_id(), &context(1), &policy, consent())
            .unwrap();
        assert!(mirror.submit_platform(
            PlatformNotification::Posted {
                key: "private.platform.notification.id".to_owned(),
                app_label: "Messages".to_owned(),
                title: Some("Secret".to_owned()),
                preview: Some("Preview".to_owned()),
                protected: true,
            },
            &context(1),
            &policy,
            consent(),
        ));
        let first = mirror.take_next(&context(1), &policy, consent()).unwrap();
        assert!(!first.encode().windows(7).any(|part| part == b"private"));
        let NotificationPayload::Posted(posted) = first else {
            panic!("expected posted");
        };
        assert!(posted.redacted());
        assert!(posted.title().is_none());
        assert!(mirror.submit_platform(
            PlatformNotification::Removed {
                key: "private.platform.notification.id".to_owned(),
            },
            &context(1),
            &policy,
            consent(),
        ));
        assert_eq!(
            mirror.take_next(&context(1), &policy, consent()),
            Some(NotificationPayload::Removed(posted.id())),
        );
        mirror.close();
        assert!(mirror.source_ids.is_empty());
        assert!(!mirror.is_subscribed());
    }

    #[test]
    fn cancellation_only_revokes_matching_subscription() {
        let mut mirror = NotificationMirror::default();
        let policy = policy(Some(RuleEffect::Allow));
        let approved = RequestId::from_bytes([0x44; 16]);
        mirror
            .subscribe(approved, &context(1), &policy, consent())
            .unwrap();

        assert!(!mirror.cancel_request(RequestId::from_bytes([0x45; 16])));
        assert!(mirror.is_subscribed());
        assert!(mirror.cancel_request(approved));
        assert!(!mirror.is_subscribed());
        assert!(!mirror.cancel_request(approved));
    }

    #[test]
    fn platform_private_flag_overrides_optional_text_consent() {
        let mut mirror = NotificationMirror::default();
        let policy = policy(Some(RuleEffect::Allow));
        let consent = NotificationConsent {
            content_enabled: true,
            ..consent()
        };
        mirror
            .subscribe(request_id(), &context(1), &policy, consent)
            .unwrap();
        assert!(mirror.submit_platform(
            PlatformNotification::Posted {
                key: "private-message-key".to_owned(),
                app_label: "Messages".to_owned(),
                title: Some("Secret".to_owned()),
                preview: Some("Sensitive".to_owned()),
                protected: true,
            },
            &context(1),
            &policy,
            consent,
        ));
        let NotificationPayload::Posted(posted) =
            mirror.take_next(&context(1), &policy, consent).unwrap()
        else {
            panic!("expected posted notification");
        };
        assert!(posted.redacted());
        assert!(posted.title().is_none());
        assert!(posted.preview().is_none());
        assert!(!format!("{posted:?}").contains("Secret"));
    }

    #[test]
    fn no_rule_ask_and_explicit_deny_cannot_subscribe() {
        for effect in [None, Some(RuleEffect::Ask), Some(RuleEffect::Deny)] {
            let mut mirror = NotificationMirror::default();
            assert_eq!(
                mirror.subscribe(request_id(), &context(1), &policy(effect), consent()),
                Err(NotificationMirrorError::NotAuthorized)
            );
        }
    }

    #[test]
    fn os_or_owner_permission_missing_blocks_subscription() {
        let policy = policy(Some(RuleEffect::Allow));
        for consent in [
            NotificationConsent {
                os_access: false,
                ..consent()
            },
            NotificationConsent {
                owner_enabled: false,
                ..consent()
            },
            NotificationConsent {
                runtime_active: false,
                ..consent()
            },
        ] {
            let mut mirror = NotificationMirror::default();
            assert_eq!(
                mirror.subscribe(request_id(), &context(1), &policy, consent),
                Err(NotificationMirrorError::Unavailable)
            );
        }
    }

    #[test]
    fn content_stays_redacted_and_queue_is_bounded() {
        let mut mirror = NotificationMirror::default();
        let policy = policy(Some(RuleEffect::Allow));
        mirror
            .subscribe(request_id(), &context(1), &policy, consent())
            .unwrap();
        for id in 0..66 {
            assert!(mirror.enqueue(posted(id), &context(1), &policy, consent()));
        }
        assert_eq!(mirror.skipped_count(), 2);
        let mut seen = 0;
        while let Some(event) = mirror.take_next(&context(1), &policy, consent()) {
            let NotificationPayload::Posted(posted) = event else {
                panic!("expected posted notification");
            };
            assert!(posted.redacted());
            assert!(posted.title().is_none());
            assert!(posted.preview().is_none());
            seen += 1;
        }
        assert_eq!(seen, NOTIFICATION_QUEUE_CAPACITY);
    }

    #[test]
    fn session_replacement_and_policy_revision_drop_pending() {
        let mut mirror = NotificationMirror::default();
        let old_policy = policy(Some(RuleEffect::Allow));
        mirror
            .subscribe(request_id(), &context(1), &old_policy, consent())
            .unwrap();
        mirror.enqueue(posted(4), &context(1), &old_policy, consent());
        assert!(
            mirror
                .take_next(&context(2), &old_policy, consent())
                .is_none()
        );
        assert!(
            mirror
                .take_next(&context(1), &old_policy, consent())
                .is_none()
        );
        mirror
            .subscribe(request_id(), &context(1), &old_policy, consent())
            .unwrap();
        mirror.enqueue(posted(4), &context(1), &old_policy, consent());
        let mut new_policy = old_policy.clone();
        new_policy
            .set_rule_effect(
                DeviceId::from_bytes([1; 32]),
                CapabilityId::parse(NOTIFICATION_CAPABILITY_ID).unwrap(),
                OperationName::parse(NOTIFICATION_SUBSCRIBE_OPERATION).unwrap(),
                RuleEffect::Deny,
            )
            .unwrap();
        assert!(
            mirror
                .take_next(&context(1), &new_policy, consent())
                .is_none()
        );
        assert!(
            mirror
                .take_next(&context(1), &old_policy, consent())
                .is_none()
        );
    }

    #[test]
    fn duplicate_identifier_is_coalesced_and_optout_drops_private_queue() {
        let mut mirror = NotificationMirror::default();
        let policy = policy(Some(RuleEffect::Allow));
        mirror
            .subscribe(request_id(), &context(1), &policy, consent())
            .unwrap();
        mirror.enqueue(posted(2), &context(1), &policy, consent());
        mirror.enqueue(posted(2), &context(1), &policy, consent());
        assert_eq!(mirror.pending.len(), 1);
        assert!(!mirror.enqueue(
            posted(3),
            &context(1),
            &policy,
            NotificationConsent {
                owner_enabled: false,
                ..consent()
            }
        ));
        assert!(mirror.pending.is_empty());
    }
}
