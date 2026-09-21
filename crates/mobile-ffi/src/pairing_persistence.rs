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

#[cfg(test)]
mod tests {
    use crosslab_core::{PairingBootstrap, PairingInstant, PairingInvitation};
    use crosslab_crypto::{SigningKey, SigningProvider};
    use crosslab_identity::DeviceId;
    use crosslab_policy::TransitionId;
    use crosslab_runtime::{ProductPairingInviter, ProductPairingJoiner};

    use crate::product_identity::{MobileSigningCallbackError, MobileSigningProvider};

    use super::*;

    struct TestMobileSigner {
        key: SigningKey,
    }

    impl TestMobileSigner {
        fn from_secret(secret: [u8; 32]) -> Self {
            Self {
                key: SigningKey::from_secret_bytes(secret),
            }
        }
    }

    impl MobileSigningProvider for TestMobileSigner {
        fn public_key(&self) -> Result<Vec<u8>, MobileSigningCallbackError> {
            Ok(self.key.verifying_key().to_bytes().to_vec())
        }

        fn sign(&self, message: Vec<u8>) -> Result<Vec<u8>, MobileSigningCallbackError> {
            Ok(self.key.sign_message(&message).to_bytes().to_vec())
        }
    }

    fn mobile_signer(secret: [u8; 32]) -> Arc<dyn MobileSigningProvider> {
        Arc::new(TestMobileSigner::from_secret(secret))
    }

    #[test]
    fn opaque_pairing_completions_build_valid_product_snapshots() {
        let root_secret = [0x91; 32];
        let issuer_secret = [0x92; 32];
        let inviter_secret = [0x93; 32];
        let joiner_secret = [0x94; 32];

        let root = SigningKey::from_secret_bytes(root_secret);
        let issuer = SigningKey::from_secret_bytes(issuer_secret);
        let inviter_key = SigningKey::from_secret_bytes(inviter_secret);
        let joiner_key = SigningKey::from_secret_bytes(joiner_secret);

        let inviter_identity =
            ProductIdentityState::bootstrap(&root, &issuer, &inviter_key).unwrap();
        let authority = inviter_identity.authority_state().unwrap();

        let mut invitation = PairingInvitation::generate(
            inviter_identity.owner_id(),
            inviter_identity.local_device_id(),
            PairingInstant::from_ticks(0),
            PairingInstant::from_ticks(100),
        )
        .unwrap();
        let code = invitation
            .bootstrap_code_at(PairingInstant::from_ticks(0))
            .unwrap();
        let bootstrap = PairingBootstrap::decode(code.as_str()).unwrap();
        drop(code);

        let mut inviter =
            ProductPairingInviter::new(invitation, inviter_identity.local_credential(), &authority)
                .unwrap();
        let mut joiner =
            ProductPairingJoiner::new(bootstrap, DeviceId::from_bytes([0x95; 32]), &joiner_key)
                .unwrap();

        inviter
            .accept_joiner_hello(joiner.hello(), PairingInstant::from_ticks(10))
            .unwrap();
        let joiner_confirmation = joiner.accept_inviter_hello(inviter.hello()).unwrap();
        let inviter_confirmation = inviter
            .verify_joiner_confirmation(&joiner_confirmation, PairingInstant::from_ticks(20))
            .unwrap();
        joiner
            .verify_inviter_confirmation(&inviter_confirmation)
            .unwrap();
        let joiner_credential = inviter
            .issue_joiner_credential(&authority, &issuer, PairingInstant::from_ticks(30))
            .unwrap();
        let accepted = joiner
            .accept_credential(&authority, &joiner_credential, &joiner_key)
            .unwrap();
        let inviter_completion = inviter
            .verify_credential_acceptance(
                &accepted,
                TransitionId::from_bytes([0x96; 32]),
                &authority,
                &issuer,
                PairingInstant::from_ticks(40),
            )
            .unwrap();
        let joiner_completion = joiner
            .accept_inviter_trust(inviter_completion.reciprocal_trust(), &authority)
            .unwrap();

        let mobile_inviter_commit = Arc::new(MobileProductPairingCommit {
            inner: inviter_completion.commit(),
        });
        let inviter_updated = product_identity_apply_pairing_commit(
            inviter_identity.encode(),
            Arc::clone(&mobile_inviter_commit),
            mobile_signer(root_secret),
            mobile_signer(issuer_secret),
            mobile_signer(inviter_secret),
        )
        .unwrap();
        let inviter_restored = ProductIdentityState::decode(&inviter_updated.payload).unwrap();
        inviter_restored
            .validate_providers(&root, &issuer, &inviter_key)
            .unwrap();
        assert!(
            inviter_restored
                .trusted_peer(joiner_credential.device_id())
                .is_some()
        );

        let mobile_joiner_completion = Arc::new(MobileProductPairingJoinerCompletion {
            inner: joiner_completion,
        });
        let joiner_updated = product_identity_from_joiner_completion(
            Arc::clone(&mobile_joiner_completion),
            mobile_signer(joiner_secret),
        )
        .unwrap();
        let joiner_restored = ProductIdentityState::decode(&joiner_updated.payload).unwrap();
        joiner_restored
            .validate_local_device_provider(&joiner_key)
            .unwrap();
        assert!(
            joiner_restored
                .trusted_peer(inviter_identity.local_device_id())
                .is_some()
        );

        assert!(format!("{mobile_inviter_commit:?}").contains("[REDACTED]"));
        assert!(format!("{mobile_joiner_completion:?}").contains("[REDACTED]"));
    }
}
