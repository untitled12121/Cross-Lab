use core::fmt::{self, Write as _};
use std::time::{Duration, Instant};

use crosslab_core::{
    PairingInstant, PairingInvitation, PairingInvitationCreateError, PairingInvitationError,
};
use crosslab_identity::DeviceId;
use crosslab_identity_store::{ProductIdentityError, ProductIdentityState};
use crosslab_runtime::ProductPairingCommit;
use qrcode_rs::{Color, EcLevel, QrCode};

#[cfg(target_os = "linux")]
mod linux_discovery;
#[cfg(target_os = "linux")]
mod linux_service;

#[cfg(target_os = "linux")]
pub use linux_discovery::{LinuxPairingAdvertisement, LinuxPairingDiscoveryError};
#[cfg(target_os = "linux")]
pub use linux_service::{
    DesktopPairingStage, LinuxPairingServiceError, LinuxProductPairingService,
};

use crate::features::identity_store::{
    LinuxEd25519Signer, LinuxIdentityStore, LinuxIdentityStoreError, LinuxSigningSlot,
};

const INVITATION_LIFETIME: Duration = Duration::from_secs(5 * 60);

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProductIdentityPresentation {
    owner_id: String,
    local_device_id: String,
    trusted_peer_ids: Vec<String>,
    trusted_peer_device_ids: Vec<DeviceId>,
    revoked_peer_ids: Vec<String>,
}

impl ProductIdentityPresentation {
    fn from_identity(identity: &ProductIdentityState) -> Self {
        Self {
            owner_id: short_hex(identity.owner_id().as_bytes()),
            local_device_id: short_hex(identity.local_device_id().as_bytes()),
            trusted_peer_ids: identity
                .trusted_peers()
                .iter()
                .filter(|peer| peer.revocation().is_none())
                .map(|peer| short_hex(peer.credential().device_id().as_bytes()))
                .collect(),
            trusted_peer_device_ids: identity
                .trusted_peers()
                .iter()
                .filter(|peer| peer.revocation().is_none())
                .map(|peer| peer.credential().device_id())
                .collect(),
            revoked_peer_ids: identity
                .trusted_peers()
                .iter()
                .filter(|peer| peer.revocation().is_some())
                .map(|peer| short_hex(peer.credential().device_id().as_bytes()))
                .collect(),
        }
    }

    pub fn owner_id(&self) -> &str {
        &self.owner_id
    }

    pub fn local_device_id(&self) -> &str {
        &self.local_device_id
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
}

pub async fn load_existing_product_identity()
-> Result<Option<ProductIdentityPresentation>, DesktopPairingError> {
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

pub async fn persist_product_pairing_commit(
    commit: ProductPairingCommit,
) -> Result<ProductIdentityPresentation, DesktopPairingError> {
    let store = LinuxIdentityStore::from_environment()?;
    let payload = store
        .load_payload()
        .await?
        .ok_or(DesktopPairingError::IdentityMissing)?;

    let root = LinuxEd25519Signer::load_required(LinuxSigningSlot::OwnerRoot).await?;
    let issuer = LinuxEd25519Signer::load_required(LinuxSigningSlot::DeviceSigning).await?;
    let local = LinuxEd25519Signer::load_required(LinuxSigningSlot::LocalDevice).await?;
    let identity = ProductIdentityState::decode(&payload)?;
    identity.validate_providers(&root, &issuer, &local)?;

    let next = identity.with_paired_peer(commit.peer_credential(), commit.peer_transition())?;
    store
        .commit_payload_if_current(&payload, next.encode())
        .await?;
    Ok(ProductIdentityPresentation::from_identity(&next))
}

/// Sign with the owner's delegated device-signing key, persist the tombstone
/// before allowing any session teardown or visible success.
pub async fn revoke_product_peer(
    device_id: DeviceId,
) -> Result<ProductIdentityPresentation, DesktopPairingError> {
    let store = LinuxIdentityStore::from_environment()?;
    let payload = store
        .load_payload()
        .await?
        .ok_or(DesktopPairingError::IdentityMissing)?;

    let root = LinuxEd25519Signer::load_required(LinuxSigningSlot::OwnerRoot).await?;
    let issuer = LinuxEd25519Signer::load_required(LinuxSigningSlot::DeviceSigning).await?;
    let local = LinuxEd25519Signer::load_required(LinuxSigningSlot::LocalDevice).await?;
    let identity = ProductIdentityState::decode(&payload)?;
    identity.validate_providers(&root, &issuer, &local)?;

    let next = identity.with_revoked_peer(device_id, &issuer)?;
    store
        .commit_payload_if_current(&payload, next.encode())
        .await?;
    Ok(ProductIdentityPresentation::from_identity(&next))
}

pub struct DesktopPairingInvitation {
    service: LinuxProductPairingService,
    created_at: Instant,
    modules: QrModules,
    owner_id: String,
    local_device_id: String,
    trusted_peer_ids: Vec<String>,
    trusted_peer_device_ids: Vec<DeviceId>,
    revoked_peer_ids: Vec<String>,
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

        let display = ProductIdentityPresentation::from_identity(&identity);
        let issuer = LinuxEd25519Signer::load_required(LinuxSigningSlot::DeviceSigning).await?;
        let service =
            LinuxProductPairingService::start(invitation, &identity, issuer, created_at).await?;

        Ok(Self {
            service,
            created_at,
            modules,
            owner_id: display.owner_id,
            local_device_id: display.local_device_id,
            trusted_peer_ids: display.trusted_peer_ids,
            trusted_peer_device_ids: display.trusted_peer_device_ids,
            revoked_peer_ids: display.revoked_peer_ids,
        })
    }

    pub fn owner_id(&self) -> &str {
        &self.owner_id
    }

    pub fn local_device_id(&self) -> &str {
        &self.local_device_id
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

    pub const fn modules(&self) -> &QrModules {
        &self.modules
    }

    pub fn seconds_remaining(&self) -> u64 {
        INVITATION_LIFETIME
            .saturating_sub(self.created_at.elapsed())
            .as_secs()
    }

    pub fn cancel(&mut self) -> Result<(), DesktopPairingError> {
        self.service.cancel()?;
        Ok(())
    }

    pub fn subscribe_status(&self) -> tokio::sync::watch::Receiver<DesktopPairingStage> {
        self.service.subscribe_status()
    }

    pub fn status(&self) -> DesktopPairingStage {
        self.service.status()
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
    IdentityMissing,
    IdentityStore(LinuxIdentityStoreError),
    ProductIdentity(ProductIdentityError),
    InvitationCreate(PairingInvitationCreateError),
    Invitation(PairingInvitationError),
    Service(LinuxPairingServiceError),
    Qr,
}

impl core::fmt::Display for DesktopPairingError {
    fn fmt(&self, formatter: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::IdentityMissing => {
                formatter.write_str("product identity is unavailable for pairing persistence")
            }
            Self::IdentityStore(error) => fmt::Display::fmt(error, formatter),
            Self::ProductIdentity(error) => fmt::Display::fmt(error, formatter),
            Self::InvitationCreate(error) => fmt::Display::fmt(error, formatter),
            Self::Invitation(error) => fmt::Display::fmt(error, formatter),
            Self::Service(error) => fmt::Display::fmt(error, formatter),
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

impl From<LinuxPairingServiceError> for DesktopPairingError {
    fn from(error: LinuxPairingServiceError) -> Self {
        Self::Service(error)
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
    use crosslab_crypto::SigningKey;
    use crosslab_identity::DeviceCredential;
    use crosslab_policy::{PairingTrustTransition, TransitionId};

    use super::*;

    #[test]
    fn inventory_preserves_full_peer_ids_and_signed_revocation_tombstones() {
        let root = SigningKey::generate().unwrap();
        let issuer = SigningKey::generate().unwrap();
        let local = SigningKey::generate().unwrap();
        let mut identity = ProductIdentityState::bootstrap(&root, &issuer, &local).unwrap();
        let peer = DeviceId::from_bytes([0x42; 32]);
        let key = SigningKey::generate().unwrap();
        let authority = identity.authority_state().unwrap();
        let credential =
            DeviceCredential::issue(identity.owner_id(), peer, &key, 0, &authority, &issuer)
                .unwrap();
        let transition = PairingTrustTransition::issue(
            &credential,
            TransitionId::from_bytes([0x43; 32]),
            [0x44; 32],
            &authority,
            &issuer,
        )
        .unwrap();
        identity.add_paired_peer(credential, transition).unwrap();

        let active = ProductIdentityPresentation::from_identity(&identity);
        assert_eq!(active.trusted_peer_ids(), &["4242424242424242".to_owned()]);
        assert_eq!(active.trusted_peer_device_ids(), &[peer]);
        assert!(active.revoked_peer_ids().is_empty());

        let revoked = identity.with_revoked_peer(peer, &issuer).unwrap();
        let loaded = ProductIdentityState::decode(&revoked.encode()).unwrap();
        let display = ProductIdentityPresentation::from_identity(&loaded);
        assert!(display.trusted_peer_ids().is_empty());
        assert!(display.trusted_peer_device_ids().is_empty());
        assert_eq!(display.revoked_peer_ids(), &["4242424242424242".to_owned()]);
    }

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
