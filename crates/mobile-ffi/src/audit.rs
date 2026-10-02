use core::fmt;
use crosslab_core::{AuditAction, AuditError, AuditHistory, AuditOutcome, AuditRecord};

#[derive(Debug, Clone, Copy, PartialEq, Eq, uniffi::Enum)]
pub enum MobileAuditAction {
    PairingStarted,
    PairingFailed,
    PairingCompleted,
    PairingCancelled,
    PeerTrusted,
    PeerRevoked,
    SessionAuthenticated,
    SessionDisconnected,
    PermissionChanged,
    TransferEnded,
    NotificationSubscriptionEnabled,
    NotificationSubscriptionDisabled,
}

impl MobileAuditAction {
    const fn inner(self) -> AuditAction {
        match self {
            Self::PairingStarted => AuditAction::PairingStarted,
            Self::PairingFailed => AuditAction::PairingFailed,
            Self::PairingCompleted => AuditAction::PairingCompleted,
            Self::PairingCancelled => AuditAction::PairingCancelled,
            Self::PeerTrusted => AuditAction::PeerTrusted,
            Self::PeerRevoked => AuditAction::PeerRevoked,
            Self::SessionAuthenticated => AuditAction::SessionAuthenticated,
            Self::SessionDisconnected => AuditAction::SessionDisconnected,
            Self::PermissionChanged => AuditAction::PermissionChanged,
            Self::TransferEnded => AuditAction::TransferEnded,
            Self::NotificationSubscriptionEnabled => AuditAction::NotificationSubscriptionEnabled,
            Self::NotificationSubscriptionDisabled => AuditAction::NotificationSubscriptionDisabled,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, uniffi::Enum)]
pub enum MobileAuditOutcome {
    Succeeded,
    Failed,
    Denied,
    Cancelled,
}

impl MobileAuditOutcome {
    const fn inner(self) -> AuditOutcome {
        match self {
            Self::Succeeded => AuditOutcome::Succeeded,
            Self::Failed => AuditOutcome::Failed,
            Self::Denied => AuditOutcome::Denied,
            Self::Cancelled => AuditOutcome::Cancelled,
        }
    }
}

#[derive(Clone, PartialEq, Eq, uniffi::Record)]
pub struct MobileAuditHistory {
    pub payload: Vec<u8>,
    pub rows: Vec<String>,
    pub dropped_count: u64,
}

impl fmt::Debug for MobileAuditHistory {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("MobileAuditHistory")
            .field("row_count", &self.rows.len())
            .field("dropped_count", &self.dropped_count)
            .finish_non_exhaustive()
    }
}

impl MobileAuditHistory {
    fn from_history(history: &AuditHistory) -> Self {
        Self {
            payload: history.encode(),
            rows: history
                .entries()
                .iter()
                .map(|e| {
                    format!(
                        "{} · {} · {} · revision {}",
                        e.occurred_hour(),
                        e.action().label(),
                        e.outcome().label(),
                        e.revision()
                    )
                })
                .collect(),
            dropped_count: history.dropped_count(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, uniffi::Error)]
pub enum MobileAuditError {
    InvalidHistory,
}

impl fmt::Display for MobileAuditError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("owner audit history validation failed")
    }
}

impl std::error::Error for MobileAuditError {}

impl From<AuditError> for MobileAuditError {
    fn from(_: AuditError) -> Self {
        Self::InvalidHistory
    }
}

fn load(payload: Option<Vec<u8>>, now_hour: u64) -> Result<AuditHistory, MobileAuditError> {
    payload
        .map(|p| AuditHistory::decode(&p, now_hour).map_err(Into::into))
        .unwrap_or_else(|| Ok(AuditHistory::default()))
}

#[uniffi::export]
pub fn audit_history_load(
    payload: Option<Vec<u8>>,
    now_hour: u64,
) -> Result<MobileAuditHistory, MobileAuditError> {
    Ok(MobileAuditHistory::from_history(&load(payload, now_hour)?))
}

#[uniffi::export]
pub fn audit_history_record(
    payload: Option<Vec<u8>>,
    now_hour: u64,
    action: MobileAuditAction,
    outcome: MobileAuditOutcome,
    revision: u64,
) -> Result<MobileAuditHistory, MobileAuditError> {
    let mut history = load(payload, now_hour)?;
    history.record(
        AuditRecord::new(action.inner(), outcome.inner(), now_hour, revision),
        now_hour,
    )?;
    Ok(MobileAuditHistory::from_history(&history))
}

#[uniffi::export]
pub fn audit_history_clear(
    payload: Option<Vec<u8>>,
    now_hour: u64,
) -> Result<MobileAuditHistory, MobileAuditError> {
    let mut history = load(payload, now_hour)?;
    history.clear();
    Ok(MobileAuditHistory::from_history(&history))
}

#[uniffi::export]
pub fn audit_history_export(
    payload: Option<Vec<u8>>,
    now_hour: u64,
) -> Result<String, MobileAuditError> {
    Ok(load(payload, now_hour)?.export_redacted_csv())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ffi_audit_is_bounded_and_cannot_be_tampered() {
        let snapshot = audit_history_record(
            None,
            100,
            MobileAuditAction::PeerRevoked,
            MobileAuditOutcome::Succeeded,
            7,
        )
        .unwrap();
        assert_eq!(snapshot.rows.len(), 1);
        assert!(snapshot.rows[0].contains("peer-revoked"));
        assert_eq!(
            audit_history_load(Some(snapshot.payload.clone()), 100)
                .unwrap()
                .rows,
            snapshot.rows
        );
        let mut corrupted = snapshot.payload;
        corrupted.push(0);
        assert!(audit_history_load(Some(corrupted), 100).is_err());
    }

    #[test]
    fn ffi_clear_keeps_only_redacted_export_header() {
        let snapshot = audit_history_record(
            None,
            100,
            MobileAuditAction::PermissionChanged,
            MobileAuditOutcome::Denied,
            1,
        )
        .unwrap();
        let cleared = audit_history_clear(Some(snapshot.payload), 100).unwrap();
        assert!(cleared.rows.is_empty());
        assert_eq!(
            audit_history_export(Some(cleared.payload), 100).unwrap(),
            "hour,action,outcome,revision\n"
        );
    }
}
