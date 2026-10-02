use std::collections::VecDeque;

use crosslab_identity::DeviceId;
use crosslab_policy::{
    AuthorizationContext, CapabilityVersion, DecisionEffect, PolicyState, SessionId, TrustState,
};
use crosslab_protocol::{
    NOTIFICATION_CAPABILITY_ID, NOTIFICATION_SUBSCRIBE_OPERATION, NotificationPayload,
    NotificationPosted,
};

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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NotificationMirrorError {
    Unavailable,
    NotAuthorized,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Subscription {
    session_id: SessionId,
    source_device_id: DeviceId,
    trust_revision: u64,
    policy_revision: u64,
}

#[derive(Default)]
pub struct NotificationMirror {
    subscription: Option<Subscription>,
    pending: VecDeque<NotificationPayload>,
    skipped: u64,
}

impl NotificationMirror {
    /// The caller must supply the verified active session's authorization context.
    pub fn subscribe(
        &mut self,
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
            session_id: context.session_id(),
            source_device_id: context.source_device_id(),
            trust_revision: context.trust_revision(),
            policy_revision: policy.revision(),
        });
        Ok(())
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
    fn no_rule_ask_and_explicit_deny_cannot_subscribe() {
        for effect in [None, Some(RuleEffect::Ask), Some(RuleEffect::Deny)] {
            let mut mirror = NotificationMirror::default();
            assert_eq!(
                mirror.subscribe(&context(1), &policy(effect), consent()),
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
                mirror.subscribe(&context(1), &policy, consent),
                Err(NotificationMirrorError::Unavailable)
            );
        }
    }

    #[test]
    fn content_stays_redacted_and_queue_is_bounded() {
        let mut mirror = NotificationMirror::default();
        let policy = policy(Some(RuleEffect::Allow));
        mirror.subscribe(&context(1), &policy, consent()).unwrap();
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
            .subscribe(&context(1), &old_policy, consent())
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
            .subscribe(&context(1), &old_policy, consent())
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
        mirror.subscribe(&context(1), &policy, consent()).unwrap();
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
