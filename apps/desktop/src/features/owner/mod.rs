use core::fmt::Write as _;

use crosslab_identity::{DeviceId, OwnerId};
use crosslab_runtime::RuntimeStatus;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OwnerPresentation {
    owner_id: String,
    local_device_id: String,
}

impl OwnerPresentation {
    pub fn from_runtime(status: &RuntimeStatus) -> Option<Self> {
        Some(Self {
            owner_id: short_owner_id(status.owner_id()?),
            local_device_id: short_device_id(status.local_device_id()?),
        })
    }

    pub fn owner_id(&self) -> &str {
        &self.owner_id
    }

    pub fn local_device_id(&self) -> &str {
        &self.local_device_id
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct OwnerFeatureState {
    current: Option<OwnerPresentation>,
    trusted_peer_ids: Vec<String>,
    trusted_peer_device_ids: Vec<DeviceId>,
    revoked_peer_ids: Vec<String>,
    audit_rows: Vec<String>,
    audit_dropped: u64,
    audit_notice: Option<String>,
}

impl OwnerFeatureState {
    pub const fn empty() -> Self {
        Self {
            current: None,
            trusted_peer_ids: Vec::new(),
            trusted_peer_device_ids: Vec::new(),
            revoked_peer_ids: Vec::new(),
            audit_rows: Vec::new(),
            audit_dropped: 0,
            audit_notice: None,
        }
    }

    pub fn from_runtime(status: &RuntimeStatus) -> Self {
        Self {
            current: OwnerPresentation::from_runtime(status),
            trusted_peer_ids: Vec::new(),
            trusted_peer_device_ids: Vec::new(),
            revoked_peer_ids: Vec::new(),
            audit_rows: Vec::new(),
            audit_dropped: 0,
            audit_notice: None,
        }
    }

    pub fn update_runtime(&mut self, status: &RuntimeStatus) {
        self.current = OwnerPresentation::from_runtime(status);
    }

    pub fn clear(&mut self) {
        self.current = None;
    }

    pub fn set_product_identity(
        &mut self,
        owner_id: String,
        local_device_id: String,
        trusted_peer_ids: Vec<String>,
        trusted_peer_device_ids: Vec<DeviceId>,
        revoked_peer_ids: Vec<String>,
    ) {
        self.current = Some(OwnerPresentation {
            owner_id,
            local_device_id,
        });
        self.trusted_peer_ids = trusted_peer_ids;
        self.trusted_peer_device_ids = trusted_peer_device_ids;
        self.revoked_peer_ids = revoked_peer_ids;
    }

    pub fn trusted_peer_ids(&self) -> &[String] {
        &self.trusted_peer_ids
    }

    pub fn trusted_peer_device_ids(&self) -> &[DeviceId] {
        &self.trusted_peer_device_ids
    }

    pub fn revoked_peer_ids(&self) -> &[String] {
        &self.revoked_peer_ids
    }

    pub fn set_audit_rows(&mut self, rows: Vec<String>, dropped: u64) {
        self.audit_rows = rows;
        self.audit_dropped = dropped;
        self.audit_notice = None;
    }

    pub fn set_audit_notice(&mut self, notice: impl Into<String>) {
        self.audit_notice = Some(notice.into());
    }

    pub fn audit_rows(&self) -> &[String] {
        &self.audit_rows
    }

    pub const fn audit_dropped(&self) -> u64 {
        self.audit_dropped
    }

    pub fn audit_notice(&self) -> Option<&str> {
        self.audit_notice.as_deref()
    }

    pub const fn current(&self) -> Option<&OwnerPresentation> {
        self.current.as_ref()
    }
}

fn short_device_id(device_id: DeviceId) -> String {
    short_id(device_id.as_bytes())
}

fn short_owner_id(owner_id: OwnerId) -> String {
    short_id(owner_id.as_bytes())
}

fn short_id(bytes: &[u8; 32]) -> String {
    let mut output = String::with_capacity(16);
    for byte in &bytes[..8] {
        write!(&mut output, "{byte:02x}").expect("writing to String cannot fail");
    }
    output
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn product_peer_inventory_survives_transient_runtime_clear() {
        let mut state = OwnerFeatureState::empty();
        state.set_product_identity(
            "owner".to_owned(),
            "local".to_owned(),
            vec!["peer".to_owned()],
            vec![DeviceId::from_bytes([0xaa; 32])],
            Vec::new(),
        );
        state.clear();
        assert_eq!(state.trusted_peer_ids(), &["peer".to_owned()]);
    }

    #[test]
    fn audit_history_is_not_deleted_by_transient_disconnect() {
        let mut state = OwnerFeatureState::empty();
        state.set_audit_rows(vec!["peer-revoked".to_owned()], 2);
        state.clear();
        assert_eq!(state.audit_rows(), &["peer-revoked".to_owned()]);
        assert_eq!(state.audit_dropped(), 2);
    }

    #[test]
    fn abbreviated_identity_is_presentation_only() {
        let owner = OwnerId::from_bytes([0xaa; 32]);
        let device = DeviceId::from_bytes([0xbb; 32]);

        assert_eq!(short_owner_id(owner), "aaaaaaaaaaaaaaaa");
        assert_eq!(short_device_id(device), "bbbbbbbbbbbbbbbb");
    }
}
