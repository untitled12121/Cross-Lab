use std::sync::Arc;

use crosslab_identity_store::ProductIdentityState;
use crosslab_runtime::{ProductPairingCommit, ProductPairingJoinerCompletion};

use crate::product_identity::{
    ForeignSigningProvider, MobileProductIdentity, MobileProductIdentityError,
    MobileSigningProvider,
};

#[derive(uniffi::Object)]
pub struct MobileProductPairingCommit {
    pub(crate) inner: ProductPairingCommit,
}

impl core::fmt::Debug for MobileProductPairingCommit {
    fn fmt(&self, formatter: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        formatter.write_str("MobileProductPairingCommit([REDACTED])")
    }
}

#[derive(uniffi::Object)]
pub struct MobileProductPairingJoinerCompletion {
    pub(crate) inner: ProductPairingJoinerCompletion,
}

impl core::fmt::Debug for MobileProductPairingJoinerCompletion {
    fn fmt(&self, formatter: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        formatter.write_str("MobileProductPairingJoinerCompletion([REDACTED])")
    }
}

#[uniffi::export]
pub fn product_identity_apply_pairing_commit(
    current_payload: Vec<u8>,
    commit: Arc<MobileProductPairingCommit>,
    root_signer: Arc<dyn MobileSigningProvider>,
    device_signing_signer: Arc<dyn MobileSigningProvider>,
    local_device_signer: Arc<dyn MobileSigningProvider>,
) -> Result<MobileProductIdentity, MobileProductIdentityError> {
    let root = ForeignSigningProvider::new(root_signer)?;
    let issuer = ForeignSigningProvider::new(device_signing_signer)?;
    let local = ForeignSigningProvider::new(local_device_signer)?;

    let identity = ProductIdentityState::decode(&current_payload)?;
    identity.validate_providers(&root, &issuer, &local)?;

    let next = identity.with_paired_peer(
        commit.inner.peer_credential(),
        commit.inner.peer_transition(),
    )?;
    next.validate_providers(&root, &issuer, &local)?;
    Ok(MobileProductIdentity::from_state(&next, false))
}

#[uniffi::export]
pub fn product_identity_from_joiner_completion(
    completion: Arc<MobileProductPairingJoinerCompletion>,
    local_device_signer: Arc<dyn MobileSigningProvider>,
) -> Result<MobileProductIdentity, MobileProductIdentityError> {
    let local = ForeignSigningProvider::new(local_device_signer)?;
    let completion = completion.inner;
    let peer = completion.peer_commit();

    let identity = ProductIdentityState::join_owner_domain(
        completion.owner_root(),
        completion.device_signing(),
        completion.local_credential(),
        &local,
    )?
    .with_paired_peer(peer.peer_credential(), peer.peer_transition())?;

    identity.validate_local_device_provider(&local)?;
    Ok(MobileProductIdentity::from_state(&identity, true))
}
