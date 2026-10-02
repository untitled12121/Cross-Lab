use crosslab_agent::NotificationInboxStatus;
use crosslab_core::{AuditAction, AuditOutcome};

use crate::features::{
    file_transfer::{FileTransferFailure, FileTransferStage},
    pairing::DesktopPairingStage,
};

/// Only bounded, non-identifying audit metadata may reach the protected writer.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct AuditIntent {
    pub(crate) action: AuditAction,
    pub(crate) outcome: AuditOutcome,
    pub(crate) revision: u64,
}

impl AuditIntent {
    pub(crate) const fn new(action: AuditAction, outcome: AuditOutcome, revision: u64) -> Self {
        Self { action, outcome, revision }
    }
}

pub(crate) struct AuditLifecycle {
    session_active: bool,
    policy_revision: Option<u64>,
    subscription: NotificationInboxStatus,
    send_stage: Option<FileTransferStage>,
    receive_stage: Option<FileTransferStage>,
    pairing_stage: Option<DesktopPairingStage>,
}

impl Default for AuditLifecycle {
    fn default() -> Self {
        Self {
            session_active: false,
            policy_revision: None,
            subscription: NotificationInboxStatus::Idle,
            send_stage: None,
            receive_stage: None,
            pairing_stage: None,
        }
    }
}

impl AuditLifecycle {
    pub(crate) fn pairing_started(&mut self) -> AuditIntent {
        self.pairing_stage = None;
        AuditIntent::new(AuditAction::PairingStarted, AuditOutcome::Succeeded, 0)
    }

    pub(crate) fn pairing(&mut self, stage: DesktopPairingStage) -> [Option<AuditIntent>; 2] {
        if self.pairing_stage == Some(stage) {
            return [None, None];
        }
        self.pairing_stage = Some(stage);
        let event = |action, outcome| Some(AuditIntent::new(action, outcome, 0));
        match stage {
            DesktopPairingStage::Paired => [
                event(AuditAction::PairingCompleted, AuditOutcome::Succeeded),
                event(AuditAction::PeerTrusted, AuditOutcome::Succeeded),
            ],
            DesktopPairingStage::Failed | DesktopPairingStage::Expired => {
                [event(AuditAction::PairingFailed, AuditOutcome::Failed), None]
            }
            DesktopPairingStage::Cancelled => {
                [event(AuditAction::PairingCancelled, AuditOutcome::Cancelled), None]
            }
            _ => [None, None],
        }
    }

    pub(crate) fn session(&mut self, active: bool) -> Option<AuditIntent> {
        if active == self.session_active {
            return None;
        }
        self.session_active = active;
        Some(AuditIntent::new(
            if active {
                AuditAction::SessionAuthenticated
            } else {
                AuditAction::SessionDisconnected
            },
            AuditOutcome::Succeeded,
            0,
        ))
    }

    pub(crate) fn permission(&mut self, revision: u64) -> Option<AuditIntent> {
        let previous = self.policy_revision.replace(revision)?;
        (revision > previous).then(|| {
            AuditIntent::new(AuditAction::PermissionChanged, AuditOutcome::Succeeded, revision)
        })
    }

    pub(crate) fn notification(
        &mut self,
        next: NotificationInboxStatus,
    ) -> Option<AuditIntent> {
        let previous = self.subscription;
        if previous == next {
            return None;
        }
        self.subscription = next;
        let result = if next == NotificationInboxStatus::Active {
            Some((
                AuditAction::NotificationSubscriptionEnabled,
                AuditOutcome::Succeeded,
            ))
        } else if previous == NotificationInboxStatus::Active {
            Some((
                AuditAction::NotificationSubscriptionDisabled,
                AuditOutcome::Succeeded,
            ))
        } else if next == NotificationInboxStatus::Denied {
            Some((
                AuditAction::NotificationSubscriptionEnabled,
                AuditOutcome::Denied,
            ))
        } else if next == NotificationInboxStatus::TimedOut {
            Some((
                AuditAction::NotificationSubscriptionEnabled,
                AuditOutcome::Failed,
            ))
        } else {
            None
        };
        result.map(|(action, outcome)| AuditIntent::new(action, outcome, 0))
    }

    pub(crate) fn transfer(
        &mut self,
        sending: bool,
        stage: FileTransferStage,
        failure: Option<FileTransferFailure>,
    ) -> Option<AuditIntent> {
        let previous = if sending {
            self.send_stage.replace(stage)
        } else {
            self.receive_stage.replace(stage)
        };
        if previous == Some(stage)
            || previous.is_none_or(|previous| previous == FileTransferStage::Idle)
        {
            return None;
        }
        let outcome = match stage {
            FileTransferStage::Completed | FileTransferStage::AlreadyComplete => {
                AuditOutcome::Succeeded
            }
            FileTransferStage::Cancelled => AuditOutcome::Cancelled,
            FileTransferStage::Failed if failure == Some(FileTransferFailure::Denied) => {
                AuditOutcome::Denied
            }
            FileTransferStage::Failed => AuditOutcome::Failed,
            _ => return None,
        };
        Some(AuditIntent::new(AuditAction::TransferEnded, outcome, 0))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_meaningful_session_permission_and_notification_transitions_emit() {
        let mut tracker = AuditLifecycle::default();
        assert_eq!(tracker.session(false), None);
        assert_eq!(
            tracker.session(true).unwrap().action,
            AuditAction::SessionAuthenticated
        );
        assert_eq!(tracker.session(true), None);
        assert_eq!(
            tracker.session(false).unwrap().action,
            AuditAction::SessionDisconnected
        );
        assert_eq!(tracker.permission(4), None);
        assert_eq!(tracker.permission(4), None);
        assert_eq!(
            tracker.permission(5),
            Some(AuditIntent::new(AuditAction::PermissionChanged, AuditOutcome::Succeeded, 5))
        );
        assert_eq!(
            tracker.notification(NotificationInboxStatus::AwaitingApproval),
            None
        );
        assert_eq!(
            tracker.notification(NotificationInboxStatus::Denied).unwrap().outcome,
            AuditOutcome::Denied
        );
        assert_eq!(
            tracker.notification(NotificationInboxStatus::AwaitingApproval),
            None
        );
        assert_eq!(
            tracker.notification(NotificationInboxStatus::Active).unwrap().action,
            AuditAction::NotificationSubscriptionEnabled
        );
        assert_eq!(tracker.notification(NotificationInboxStatus::Active), None);
        assert_eq!(
            tracker.notification(NotificationInboxStatus::Idle).unwrap().action,
            AuditAction::NotificationSubscriptionDisabled
        );
    }

    #[test]
    fn pairing_success_failure_and_replayed_statuses_are_deduplicated() {
        let mut tracker = AuditLifecycle::default();
        assert_eq!(
            tracker.pairing_started().action,
            AuditAction::PairingStarted
        );
        assert_eq!(
            tracker.pairing(DesktopPairingStage::Waiting),
            [None, None]
        );
        assert_eq!(
            tracker.pairing(DesktopPairingStage::Paired)[0].unwrap().action,
            AuditAction::PairingCompleted
        );
        assert_eq!(
            tracker.pairing(DesktopPairingStage::Paired),
            [None, None]
        );
        assert_eq!(
            tracker.pairing_started().action,
            AuditAction::PairingStarted
        );
        assert_eq!(
            tracker.pairing(DesktopPairingStage::Failed)[0].unwrap().outcome,
            AuditOutcome::Failed
        );
    }

    #[test]
    fn transfer_outcomes_do_not_embed_names_or_log_duplicate_events() {
        let mut tracker = AuditLifecycle::default();
        assert_eq!(tracker.transfer(true, FileTransferStage::Idle, None), None);
        assert_eq!(tracker.transfer(true, FileTransferStage::Transferring, None), None);
        let denied = tracker
            .transfer(true, FileTransferStage::Failed, Some(FileTransferFailure::Denied))
            .unwrap();
        assert_eq!(denied.action, AuditAction::TransferEnded);
        assert_eq!(denied.outcome, AuditOutcome::Denied);
        assert_eq!(
            tracker.transfer(true, FileTransferStage::Failed, Some(FileTransferFailure::Denied)),
            None
        );
        assert_eq!(tracker.transfer(false, FileTransferStage::Preparing, None), None);
        assert_eq!(
            tracker
                .transfer(false, FileTransferStage::Completed, None)
                .unwrap()
                .outcome,
            AuditOutcome::Succeeded
        );
    }
}
