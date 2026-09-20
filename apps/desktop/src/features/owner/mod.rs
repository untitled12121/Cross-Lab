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
}

impl OwnerFeatureState {
    pub const fn empty() -> Self {
        Self { current: None }
    }

    pub fn from_runtime(status: &RuntimeStatus) -> Self {
        Self {
            current: OwnerPresentation::from_runtime(status),
        }
    }

    pub fn update_runtime(&mut self, status: &RuntimeStatus) {
        self.current = OwnerPresentation::from_runtime(status);
    }

    pub fn clear(&mut self) {
        self.current = None;
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
    fn abbreviated_identity_is_presentation_only() {
        let owner = OwnerId::from_bytes([0xaa; 32]);
        let device = DeviceId::from_bytes([0xbb; 32]);

        assert_eq!(short_owner_id(owner), "aaaaaaaaaaaaaaaa");
        assert_eq!(short_device_id(device), "bbbbbbbbbbbbbbbb");
    }
}
