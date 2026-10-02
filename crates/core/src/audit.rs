use core::fmt;
use std::collections::VecDeque;

pub const MAX_AUDIT_EVENTS: usize = 1024;
pub const MAX_AUDIT_SNAPSHOT_BYTES: usize = 256 * 1024;
pub const AUDIT_RETENTION_HOURS: u64 = 30 * 24;

const MAGIC: &[u8; 8] = b"CLAUD01\0";
const VERSION: u16 = 1;
const HEADER_LEN: usize = 8 + 2 + 8 + 4;
const RECORD_LEN: usize = 1 + 1 + 8 + 8;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum AuditAction {
    PairingStarted = 1,
    PairingFailed = 2,
    PairingCompleted = 3,
    PairingCancelled = 4,
    PeerTrusted = 5,
    PeerRevoked = 6,
    SessionAuthenticated = 7,
    SessionDisconnected = 8,
    PermissionChanged = 9,
    TransferEnded = 10,
    NotificationSubscriptionEnabled = 11,
    NotificationSubscriptionDisabled = 12,
}

impl AuditAction {
    fn from_code(code: u8) -> Result<Self, AuditError> {
        match code {
            1 => Ok(Self::PairingStarted),
            2 => Ok(Self::PairingFailed),
            3 => Ok(Self::PairingCompleted),
            4 => Ok(Self::PairingCancelled),
            5 => Ok(Self::PeerTrusted),
            6 => Ok(Self::PeerRevoked),
            7 => Ok(Self::SessionAuthenticated),
            8 => Ok(Self::SessionDisconnected),
            9 => Ok(Self::PermissionChanged),
            10 => Ok(Self::TransferEnded),
            11 => Ok(Self::NotificationSubscriptionEnabled),
            12 => Ok(Self::NotificationSubscriptionDisabled),
            _ => Err(AuditError::InvalidCode),
        }
    }

    const fn label(self) -> &'static str {
        match self {
            Self::PairingStarted => "pairing-started",
            Self::PairingFailed => "pairing-failed",
            Self::PairingCompleted => "pairing-completed",
            Self::PairingCancelled => "pairing-cancelled",
            Self::PeerTrusted => "peer-trusted",
            Self::PeerRevoked => "peer-revoked",
            Self::SessionAuthenticated => "session-authenticated",
            Self::SessionDisconnected => "session-disconnected",
            Self::PermissionChanged => "permission-changed",
            Self::TransferEnded => "transfer-ended",
            Self::NotificationSubscriptionEnabled => "notifications-enabled",
            Self::NotificationSubscriptionDisabled => "notifications-disabled",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum AuditOutcome {
    Succeeded = 1,
    Failed = 2,
    Denied = 3,
    Cancelled = 4,
}

impl AuditOutcome {
    fn from_code(code: u8) -> Result<Self, AuditError> {
        match code {
            1 => Ok(Self::Succeeded),
            2 => Ok(Self::Failed),
            3 => Ok(Self::Denied),
            4 => Ok(Self::Cancelled),
            _ => Err(AuditError::InvalidCode),
        }
    }

    const fn label(self) -> &'static str {
        match self {
            Self::Succeeded => "ok",
            Self::Failed => "failed",
            Self::Denied => "denied",
            Self::Cancelled => "cancelled",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AuditError {
    Oversized,
    Malformed,
    UnsupportedSchema,
    InvalidCode,
    InvalidClock,
}

impl fmt::Display for AuditError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::Oversized => "audit snapshot exceeds its resource bound",
            Self::Malformed => "audit snapshot is malformed",
            Self::UnsupportedSchema => "audit snapshot format is unsupported",
            Self::InvalidCode => "audit event contains an unsupported code",
            Self::InvalidClock => "audit event timestamp is outside the retention window",
        })
    }
}

impl std::error::Error for AuditError {}

/// Intentionally carries no private content, raw peer IDs, file paths or notification data.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AuditRecord {
    action: AuditAction,
    outcome: AuditOutcome,
    occurred_hour: u64,
    revision: u64,
}

impl AuditRecord {
    pub const fn new(
        action: AuditAction,
        outcome: AuditOutcome,
        occurred_hour: u64,
        revision: u64,
    ) -> Self {
        Self {
            action,
            outcome,
            occurred_hour,
            revision,
        }
    }

    pub const fn action(self) -> AuditAction {
        self.action
    }

    pub const fn outcome(self) -> AuditOutcome {
        self.outcome
    }

    pub const fn occurred_hour(self) -> u64 {
        self.occurred_hour
    }

    pub const fn revision(self) -> u64 {
        self.revision
    }
}

/// Platform adapters enforce protected, owner-only access and durable currentness.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct AuditHistory {
    events: VecDeque<AuditRecord>,
    dropped_count: u64,
}

impl AuditHistory {
    pub fn record(&mut self, event: AuditRecord, now_hour: u64) -> Result<(), AuditError> {
        if event.occurred_hour > now_hour.saturating_add(1)
            || now_hour.saturating_sub(event.occurred_hour) > AUDIT_RETENTION_HOURS
        {
            return Err(AuditError::InvalidClock);
        }
        self.prune(now_hour);
        if self.events.len() >= MAX_AUDIT_EVENTS {
            let _ = self.events.pop_front();
            self.dropped_count = self.dropped_count.saturating_add(1);
        }
        self.events.push_back(event);
        Ok(())
    }

    pub fn entries(&self) -> &VecDeque<AuditRecord> {
        &self.events
    }

    pub const fn dropped_count(&self) -> u64 {
        self.dropped_count
    }

    pub fn clear(&mut self) {
        self.events.clear();
        self.dropped_count = 0;
    }

    pub fn prune(&mut self, now_hour: u64) {
        self.events.retain(|event| {
            now_hour.saturating_sub(event.occurred_hour) <= AUDIT_RETENTION_HOURS
                && event.occurred_hour <= now_hour.saturating_add(1)
        });
    }

    pub fn encode(&self) -> Vec<u8> {
        let mut bytes = Vec::with_capacity(HEADER_LEN + self.events.len() * RECORD_LEN);
        bytes.extend_from_slice(MAGIC);
        bytes.extend_from_slice(&VERSION.to_be_bytes());
        bytes.extend_from_slice(&self.dropped_count.to_be_bytes());
        bytes.extend_from_slice(&(self.events.len() as u32).to_be_bytes());
        for event in &self.events {
            bytes.push(event.action as u8);
            bytes.push(event.outcome as u8);
            bytes.extend_from_slice(&event.occurred_hour.to_be_bytes());
            bytes.extend_from_slice(&event.revision.to_be_bytes());
        }
        bytes
    }

    pub fn decode(bytes: &[u8], now_hour: u64) -> Result<Self, AuditError> {
        if bytes.len() > MAX_AUDIT_SNAPSHOT_BYTES {
            return Err(AuditError::Oversized);
        }
        if bytes.len() < HEADER_LEN || &bytes[..MAGIC.len()] != MAGIC {
            return Err(AuditError::Malformed);
        }
        let version = u16::from_be_bytes([bytes[8], bytes[9]]);
        if version != VERSION {
            return Err(AuditError::UnsupportedSchema);
        }
        let dropped_count = u64::from_be_bytes(copy_array(&bytes[10..18]));
        let count = u32::from_be_bytes(copy_array(&bytes[18..22])) as usize;
        if count > MAX_AUDIT_EVENTS || bytes.len() != HEADER_LEN + count * RECORD_LEN {
            return Err(AuditError::Malformed);
        }
        let mut events = VecDeque::with_capacity(count);
        for chunk in bytes[HEADER_LEN..].chunks_exact(RECORD_LEN) {
            events.push_back(AuditRecord {
                action: AuditAction::from_code(chunk[0])?,
                outcome: AuditOutcome::from_code(chunk[1])?,
                occurred_hour: u64::from_be_bytes(copy_array(&chunk[2..10])),
                revision: u64::from_be_bytes(copy_array(&chunk[10..18])),
            });
        }
        let mut history = Self {
            events,
            dropped_count,
        };
        history.prune(now_hour);
        Ok(history)
    }

    pub fn export_redacted_csv(&self) -> String {
        use core::fmt::Write as _;
        let mut output = String::from("hour,action,outcome,revision\n");
        for event in &self.events {
            writeln!(
                &mut output,
                "{},{},{},{}",
                event.occurred_hour,
                event.action.label(),
                event.outcome.label(),
                event.revision
            )
            .expect("writing to string");
        }
        output
    }
}

fn copy_array<const N: usize>(bytes: &[u8]) -> [u8; N] {
    let mut output = [0; N];
    output.copy_from_slice(bytes);
    output
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_snapshot_has_stable_golden_bytes() {
        let history = AuditHistory::default();
        let mut expected = MAGIC.to_vec();
        expected.extend_from_slice(&VERSION.to_be_bytes());
        expected.extend_from_slice(&0_u64.to_be_bytes());
        expected.extend_from_slice(&0_u32.to_be_bytes());
        assert_eq!(history.encode(), expected);
        assert_eq!(AuditHistory::decode(&expected, 1), Ok(history));
    }

    #[test]
    fn records_are_bounded_and_old_events_are_pruned() {
        let mut history = AuditHistory::default();
        for revision in 0..MAX_AUDIT_EVENTS + 5 {
            history
                .record(
                    AuditRecord::new(
                        AuditAction::PeerRevoked,
                        AuditOutcome::Succeeded,
                        1000,
                        revision as u64,
                    ),
                    1000,
                )
                .unwrap();
        }
        assert_eq!(history.events.len(), MAX_AUDIT_EVENTS);
        assert_eq!(history.dropped_count(), 5);
        history.prune(1000 + AUDIT_RETENTION_HOURS + 1);
        assert!(history.events.is_empty());
    }

    #[test]
    fn snapshot_roundtrip_and_corruption_checks() {
        let mut history = AuditHistory::default();
        history
            .record(
                AuditRecord::new(AuditAction::PairingFailed, AuditOutcome::Denied, 120, 4),
                120,
            )
            .unwrap();
        let bytes = history.encode();
        assert_eq!(AuditHistory::decode(&bytes, 120), Ok(history));
        let mut corrupt = bytes;
        corrupt[HEADER_LEN] = 255;
        assert_eq!(AuditHistory::decode(&corrupt, 120), Err(AuditError::InvalidCode));
        corrupt[HEADER_LEN] = 2;
        corrupt.push(0);
        assert_eq!(AuditHistory::decode(&corrupt, 120), Err(AuditError::Malformed));
        assert_eq!(
            AuditHistory::decode(&vec![0; MAX_AUDIT_SNAPSHOT_BYTES + 1], 120),
            Err(AuditError::Oversized)
        );
    }

    #[test]
    fn export_contains_only_structured_safe_fields() {
        let mut history = AuditHistory::default();
        history
            .record(
                AuditRecord::new(AuditAction::TransferEnded, AuditOutcome::Failed, 350, 9),
                350,
            )
            .unwrap();
        let csv = history.export_redacted_csv();
        assert!(csv.contains("350,transfer-ended,failed,9"));
        assert!(!csv.contains("payload"));
        history.clear();
        assert!(history.entries().is_empty());
    }

    #[test]
    fn clock_anomalies_do_not_extend_retention() {
        let mut history = AuditHistory::default();
        assert_eq!(
            history.record(
                AuditRecord::new(AuditAction::PeerRevoked, AuditOutcome::Succeeded, 200, 0),
                100,
            ),
            Err(AuditError::InvalidClock)
        );
        assert_eq!(
            history.record(
                AuditRecord::new(AuditAction::PeerRevoked, AuditOutcome::Succeeded, 10, 0),
                1000,
            ),
            Err(AuditError::InvalidClock)
        );
    }
}
