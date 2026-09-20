use core::fmt::Write as _;
use std::time::{Duration, Instant};

use crosslab_core::{
    PairingInstant, PairingInvitation, PairingInvitationCreateError, PairingInvitationError,
};
use crosslab_identity_store::{ProductIdentityError, ProductIdentityState};
use qrcode_rs::{Color, EcLevel, QrCode};

use crate::features::identity_store::{
    LinuxEd25519Signer, LinuxIdentityStore, LinuxIdentityStoreError, LinuxSigningSlot,
};

const INVITATION_LIFETIME: Duration = Duration::from_secs(5 * 60);

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProductIdentityPresentation {
    owner_id: String,
    local_device_id: String,
}

impl ProductIdentityPresentation {
    fn from_identity(identity: &ProductIdentityState) -> Self {
        Self {
            owner_id: short_hex(identity.owner_id().as_bytes()),
            local_device_id: short_hex(identity.local_device_id().as_bytes()),
        }
    }

    pub fn owner_id(&self) -> &str {
        &self.owner_id
    }

    pub fn local_device_id(&self) -> &str {
        &self.local_device_id
    }
}

pub async fn load_existing_product_identity(
) -> Result<Option<ProductIdentityPresentation>, DesktopPairingError> {
    let store = LinuxIdentityStore::from_environment()?;
    let Some(payload) = store.load_payload().await? else {
        return Ok(None);
    };

    let root = LinuxEd25519Signer::load_required(LinuxSigningSlot::OwnerRoot).await?;
    let issuer = LinuxEd25519Signer::load_required(LinuxSigningSlot::DeviceSigning).await?;
    let local = LinuxEd25519Signer::load_required(LinuxSigningSlot::LocalDevice).await?;
    let identity = ProductIdentityState::decode(&payload)?;
    identity.validate_providers(&root, &issuer, &local)?;
    Ok(Some(ProductIdentityPresentation::from_identity(&identity)))
}

pub struct DesktopPairingInvitation {
    invitation: PairingInvitation,
    created_at: Instant,
    modules: QrModules,
    owner_id: String,
    local_device_id: String,
}

impl DesktopPairingInvitation {
    pub async fn create() -> Result<Self, DesktopPairingError> {
        let identity = load_or_create_product_identity().await?;
        let created_at = Instant::now();
        let deadline_ticks =
            u64::try_from(INVITATION_LIFETIME.as_millis()).expect("five minutes fits u64");
        let mut invitation = PairingInvitation::generate(
            identity.owner_id(),
            identity.local_device_id(),
            PairingInstant::from_ticks(0),
            PairingInstant::from_ticks(deadline_ticks),
        )?;
        let code = invitation.bootstrap_code_at(PairingInstant::from_ticks(0))?;
        let qr = QrCode::with_error_correction_level(code.as_str().as_bytes(), EcLevel::M)
            .map_err(|_| DesktopPairingError::Qr)?;
        let modules = QrModules::from_qr(&qr);
        drop(code);

        Ok(Self {
            invitation,
            created_at,
            modules,
            owner_id: short_hex(identity.owner_id().as_bytes()),
            local_device_id: short_hex(identity.local_device_id().as_bytes()),
        })
    }

    pub fn owner_id(&self) -> &str {
        &self.owner_id
    }

    pub fn local_device_id(&self) -> &str {
        &self.local_device_id
    }

    pub const fn modules(&self) -> &QrModules {
        &self.modules
    }

    pub fn seconds_remaining(&self) -> u64 {
        INVITATION_LIFETIME
            .saturating_sub(self.created_at.elapsed())
            .as_secs()
    }

    pub fn cancel(&mut self) -> Result<(), DesktopPairingError> {
        self.invitation.cancel()?;
        Ok(())
    }

    pub fn now(&self) -> PairingInstant {
        PairingInstant::from_ticks(
            self.created_at
                .elapsed()
                .as_millis()
                .try_into()
                .unwrap_or(u64::MAX),
        )
    }

    pub(crate) fn invitation_mut(&mut self) -> &mut PairingInvitation {
        &mut self.invitation
    }
}

pub struct QrModules {
    width: usize,
    modules: Vec<u8>,
}

impl QrModules {
    fn from_qr(qr: &QrCode) -> Self {
        let modules = qr
            .colors()
            .iter()
            .copied()
            .map(|color| u8::from(color == Color::Dark))
            .collect();
        Self {
            width: qr.width(),
            modules,
        }
    }

    pub const fn width(&self) -> usize {
        self.width
    }

    pub fn is_dark(&self, x: usize, y: usize) -> bool {
        self.modules[y * self.width + x] == 1
    }
}

impl Drop for QrModules {
    fn drop(&mut self) {
        self.modules.fill(0);
    }
}

#[derive(Debug)]
pub enum DesktopPairingError {
    IdentityStore(LinuxIdentityStoreError),
    ProductIdentity(ProductIdentityError),
    InvitationCreate(PairingInvitationCreateError),
    Invitation(PairingInvitationError),
    Qr,
}

impl core::fmt::Display for DesktopPairingError {
    fn fmt(&self, formatter: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::IdentityStore(error) => fmt::Display::fmt(error, formatter),
            Self::ProductIdentity(error) => fmt::Display::fmt(error, formatter),
            Self::InvitationCreate(error) => fmt::Display::fmt(error, formatter),
            Self::Invitation(error) => fmt::Display::fmt(error, formatter),
            Self::Qr => formatter.write_str("failed to render the pairing QR"),
        }
    }
}

impl std::error::Error for DesktopPairingError {}

impl From<LinuxIdentityStoreError> for DesktopPairingError {
    fn from(error: LinuxIdentityStoreError) -> Self {
        Self::IdentityStore(error)
    }
}

impl From<ProductIdentityError> for DesktopPairingError {
    fn from(error: ProductIdentityError) -> Self {
        Self::ProductIdentity(error)
    }
}

impl From<PairingInvitationCreateError> for DesktopPairingError {
    fn from(error: PairingInvitationCreateError) -> Self {
        Self::InvitationCreate(error)
    }
}

impl From<PairingInvitationError> for DesktopPairingError {
    fn from(error: PairingInvitationError) -> Self {
        Self::Invitation(error)
    }
}

async fn load_or_create_product_identity() -> Result<ProductIdentityState, DesktopPairingError> {
    let store = LinuxIdentityStore::from_environment()?;

    if let Some(payload) = store.load_payload().await? {
        let root = LinuxEd25519Signer::load_required(LinuxSigningSlot::OwnerRoot).await?;
        let issuer = LinuxEd25519Signer::load_required(LinuxSigningSlot::DeviceSigning).await?;
        let local = LinuxEd25519Signer::load_required(LinuxSigningSlot::LocalDevice).await?;
        let identity = ProductIdentityState::decode(&payload)?;
        identity.validate_providers(&root, &issuer, &local)?;
        return Ok(identity);
    }

    let root = LinuxEd25519Signer::load_or_create(LinuxSigningSlot::OwnerRoot).await?;
    let issuer = LinuxEd25519Signer::load_or_create(LinuxSigningSlot::DeviceSigning).await?;
    let local = LinuxEd25519Signer::load_or_create(LinuxSigningSlot::LocalDevice).await?;
    let identity = ProductIdentityState::bootstrap(&root, &issuer, &local)?;
    store.commit_payload(identity.encode()).await?;
    Ok(identity)
}

fn short_hex(bytes: &[u8; 32]) -> String {
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
    fn qr_modules_index_row_major() {
        let qr = QrCode::new(b"crosslab pairing test").unwrap();
        let modules = QrModules::from_qr(&qr);
        assert_eq!(modules.width(), qr.width());

        for y in 0..qr.width() {
            for x in 0..qr.width() {
                assert_eq!(modules.is_dark(x, y), qr[(x, y)] == Color::Dark);
            }
        }
    }
}
