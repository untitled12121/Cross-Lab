use std::sync::{Arc, Mutex};

use crosslab_core::{PairingBootstrap, PairingBootstrapError};

#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct MobilePairingBootstrapSummary {
    pub pairing_id: String,
    pub owner_id: String,
    pub inviter_device_id: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, uniffi::Error)]
pub enum MobilePairingBootstrapError {
    InvalidPrefix,
    InvalidLength,
    InvalidEncoding,
    UnsupportedProfile,
    AlreadyConsumed,
}

impl core::fmt::Display for MobilePairingBootstrapError {
    fn fmt(&self, formatter: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        formatter.write_str(match self {
            Self::InvalidPrefix => "pairing QR is not a Cross-Lab invitation",
            Self::InvalidLength => "pairing QR has an invalid length",
            Self::InvalidEncoding => "pairing QR has invalid encoding",
            Self::UnsupportedProfile => "pairing QR uses an unsupported profile",
            Self::AlreadyConsumed => "pairing invitation has already been consumed",
        })
    }
}

impl std::error::Error for MobilePairingBootstrapError {}

#[derive(uniffi::Object)]
pub struct MobilePairingBootstrap {
    bootstrap: Mutex<Option<PairingBootstrap>>,
}

#[uniffi::export]
impl MobilePairingBootstrap {
    pub fn summary(&self) -> Result<MobilePairingBootstrapSummary, MobilePairingBootstrapError> {
        let bootstrap = self
            .bootstrap
            .lock()
            .expect("mobile pairing bootstrap lock poisoned");
        let bootstrap = bootstrap
            .as_ref()
            .ok_or(MobilePairingBootstrapError::AlreadyConsumed)?;

        Ok(MobilePairingBootstrapSummary {
            pairing_id: hex(bootstrap.pairing_id().as_bytes()),
            owner_id: short_hex(bootstrap.owner_id().as_bytes()),
            inviter_device_id: short_hex(bootstrap.inviter_device_id().as_bytes()),
        })
    }
}

impl core::fmt::Debug for MobilePairingBootstrap {
    fn fmt(&self, formatter: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        formatter.write_str("MobilePairingBootstrap([REDACTED])")
    }
}

#[uniffi::export]
pub fn scan_pairing_bootstrap(
    code: String,
) -> Result<Arc<MobilePairingBootstrap>, MobilePairingBootstrapError> {
    let bootstrap = PairingBootstrap::decode(&code)?;
    Ok(Arc::new(MobilePairingBootstrap {
        bootstrap: Mutex::new(Some(bootstrap)),
    }))
}

impl From<PairingBootstrapError> for MobilePairingBootstrapError {
    fn from(error: PairingBootstrapError) -> Self {
        match error {
            PairingBootstrapError::InvalidPrefix => Self::InvalidPrefix,
            PairingBootstrapError::InvalidLength => Self::InvalidLength,
            PairingBootstrapError::InvalidEncoding => Self::InvalidEncoding,
            PairingBootstrapError::UnsupportedProfile => Self::UnsupportedProfile,
        }
    }
}

fn short_hex(bytes: &[u8; 32]) -> String {
    hex(&bytes[..8])
}

fn hex(bytes: &[u8]) -> String {
    use core::fmt::Write as _;

    let mut output = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        write!(&mut output, "{byte:02x}").expect("writing to String cannot fail");
    }
    output
}

#[cfg(test)]
mod tests {
    use crosslab_core::{PairingInstant, PairingInvitation, PairingInvitationCreateError};
    use crosslab_identity::{DeviceId, OwnerId};

    use super::*;

    #[test]
    fn scanner_keeps_secret_inside_rust_object() -> Result<(), PairingInvitationCreateError> {
        let owner = OwnerId::from_bytes([0x11; 32]);
        let inviter = DeviceId::from_bytes([0x22; 32]);
        let mut invitation = PairingInvitation::generate(
            owner,
            inviter,
            PairingInstant::from_ticks(10),
            PairingInstant::from_ticks(20),
        )?;
        let code = invitation
            .bootstrap_code_at(PairingInstant::from_ticks(11))
            .expect("pending invitation");
        let scanned = scan_pairing_bootstrap(code.as_str().to_owned()).expect("valid bootstrap");

        let summary = scanned.summary().expect("available summary");
        assert_eq!(summary.owner_id, "1111111111111111");
        assert_eq!(summary.inviter_device_id, "2222222222222222");
        assert_eq!(summary.pairing_id.len(), 32);

        let debug = format!("{scanned:?}");
        assert!(!debug.contains(code.as_str()));
        Ok(())
    }

    #[test]
    fn scanner_rejects_non_crosslab_qr() {
        assert!(matches!(
            scan_pairing_bootstrap("https://example.com".to_owned()),
            Err(MobilePairingBootstrapError::InvalidPrefix)
        ));
    }
}
