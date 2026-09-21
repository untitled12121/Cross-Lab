use crosslab_core::{PairingBootstrap, PairingJoinerFlow};
use crosslab_crypto::{SigningProvider, random_bytes};
use crosslab_identity::{
    AuthorityDelegation, AuthorityRole, DeviceCredential, DeviceId, OwnerAuthorityState,
    OwnerRootRecord,
};
use crosslab_protocol::{
    PairingConfirmation, PairingCredentialAccepted, PairingHello, PairingRole,
};

use super::{
    PROTOCOL_MAJOR_V1, ProductPairingCommit, ProductPairingError, ProductPairingJoinerCompletion,
    ProductPairingState, ProductPairingTrustBundle,
};

pub struct ProductPairingJoiner {
    hello: PairingHello,
    bootstrap: Option<PairingBootstrap>,
    flow: Option<PairingJoinerFlow>,
    owner_root: Option<OwnerRootRecord>,
    device_signing: Option<AuthorityDelegation>,
    accepted_credential: Option<DeviceCredential>,
    state: ProductPairingState,
}

impl ProductPairingJoiner {
    pub fn new(
        bootstrap: PairingBootstrap,
        local_device_id: DeviceId,
        local_signer: &dyn SigningProvider,
    ) -> Result<Self, ProductPairingError> {
        let nonce = random_bytes().map_err(|_| ProductPairingError::Random)?;
        Self::with_nonce(bootstrap, local_device_id, local_signer, nonce)
    }

    pub(super) fn with_nonce(
        bootstrap: PairingBootstrap,
        local_device_id: DeviceId,
        local_signer: &dyn SigningProvider,
        nonce: [u8; 32],
    ) -> Result<Self, ProductPairingError> {
        if local_device_id == bootstrap.inviter_device_id() {
            return Err(ProductPairingError::LocalIdentityMismatch);
        }

        let hello = PairingHello::new(
            PairingRole::Joiner,
            PROTOCOL_MAJOR_V1,
            bootstrap.pairing_id().to_bytes(),
            bootstrap.owner_id(),
            local_device_id,
            local_signer.verifying_key(),
            nonce,
        );
        Ok(Self {
            hello,
            bootstrap: Some(bootstrap),
            flow: None,
            owner_root: None,
            device_signing: None,
            accepted_credential: None,
            state: ProductPairingState::AwaitingPeerHello,
        })
    }

    pub const fn state(&self) -> ProductPairingState {
        self.state
    }

    pub const fn hello(&self) -> PairingHello {
        self.hello
    }

    pub fn accept_inviter_hello(
        &mut self,
        inviter: PairingHello,
    ) -> Result<PairingConfirmation, ProductPairingError> {
        if self.state != ProductPairingState::AwaitingPeerHello {
            return self.fail(ProductPairingError::InvalidState);
        }

        let bootstrap = self
            .bootstrap
            .take()
            .ok_or(ProductPairingError::InvalidState)?;
        if inviter.role() != PairingRole::Inviter
            || inviter.pairing_id() != bootstrap.pairing_id().to_bytes()
            || inviter.owner_id() != bootstrap.owner_id()
            || inviter.device_id() != bootstrap.inviter_device_id()
        {
            return self.fail(ProductPairingError::BootstrapPeerMismatch);
        }

        let (_, secret, _, _) = bootstrap.into_parts();
        let flow = match PairingJoinerFlow::new(secret, inviter, self.hello) {
            Ok(flow) => flow,
            Err(error) => return self.fail(error.into()),
        };
        let confirmation = match flow.joiner_confirmation() {
            Ok(confirmation) => confirmation,
            Err(error) => return self.fail(error.into()),
        };
        self.flow = Some(flow);
        self.state = ProductPairingState::AwaitingPeerConfirmation;
        Ok(confirmation)
    }

    pub fn verify_inviter_confirmation(
        &mut self,
        confirmation: &PairingConfirmation,
    ) -> Result<(), ProductPairingError> {
        if self.state != ProductPairingState::AwaitingPeerConfirmation {
            return self.fail(ProductPairingError::InvalidState);
        }
        let result = self
            .flow
            .as_mut()
            .ok_or(ProductPairingError::InvalidState)?
            .verify_inviter_confirmation(confirmation);
        match result {
            Ok(()) => {
                self.state = ProductPairingState::AwaitingCredential;
                Ok(())
            }
            Err(error) => self.fail(error.into()),
        }
    }

    pub fn accept_credential(
        &mut self,
        authority: &OwnerAuthorityState,
        credential: &DeviceCredential,
        local_signer: &dyn SigningProvider,
    ) -> Result<PairingCredentialAccepted, ProductPairingError> {
        if self.state != ProductPairingState::AwaitingCredential {
            return self.fail(ProductPairingError::InvalidState);
        }
        let owner_root = *authority.root();
        let device_signing = match authority.current_delegation(AuthorityRole::DeviceSigning) {
            Ok(delegation) => *delegation,
            Err(error) => return self.fail(error.into()),
        };
        let result = self
            .flow
            .as_mut()
            .ok_or(ProductPairingError::InvalidState)?
            .accept_credential_with_provider(authority, credential, local_signer);
        match result {
            Ok(accepted) => {
                self.owner_root = Some(owner_root);
                self.device_signing = Some(device_signing);
                self.accepted_credential = Some(*credential);
                self.state = ProductPairingState::AwaitingPeerTrust;
                Ok(accepted)
            }
            Err(error) => self.fail(error.into()),
        }
    }

    pub const fn accepted_credential(&self) -> Option<DeviceCredential> {
        self.accepted_credential
    }

    pub fn accept_inviter_trust(
        &mut self,
        bundle: ProductPairingTrustBundle,
        authority: &OwnerAuthorityState,
    ) -> Result<ProductPairingJoinerCompletion, ProductPairingError> {
        if self.state != ProductPairingState::AwaitingPeerTrust {
            return self.fail(ProductPairingError::InvalidState);
        }

        let owner_root = match self.owner_root {
            Some(root) => root,
            None => return self.fail(ProductPairingError::InvalidState),
        };
        let device_signing = match self.device_signing {
            Some(delegation) => delegation,
            None => return self.fail(ProductPairingError::InvalidState),
        };
        let local_credential = match self.accepted_credential {
            Some(credential) => credential,
            None => return self.fail(ProductPairingError::InvalidState),
        };

        let result = self
            .flow
            .as_ref()
            .ok_or(ProductPairingError::InvalidState)?
            .establish_inviter_trust(&bundle.credential, &bundle.transition, authority);
        match result {
            Ok(trust) => {
                self.state = ProductPairingState::AwaitingPersistence;
                Ok(ProductPairingJoinerCompletion {
                    owner_root,
                    device_signing,
                    local_credential,
                    peer_commit: ProductPairingCommit {
                        peer_credential: bundle.credential,
                        peer_transition: bundle.transition,
                        peer_trust: trust,
                    },
                })
            }
            Err(error) => self.fail(error.into()),
        }
    }

    pub fn mark_persisted(&mut self) -> Result<(), ProductPairingError> {
        if self.state != ProductPairingState::AwaitingPersistence {
            return self.fail(ProductPairingError::InvalidState);
        }
        self.state = ProductPairingState::Complete;
        Ok(())
    }

    pub fn persistence_failed(&mut self) -> Result<(), ProductPairingError> {
        if self.state != ProductPairingState::AwaitingPersistence {
            return self.fail(ProductPairingError::InvalidState);
        }
        self.state = ProductPairingState::Failed;
        Ok(())
    }

    pub fn cancel(&mut self) -> Result<(), ProductPairingError> {
        if matches!(
            self.state,
            ProductPairingState::Complete
                | ProductPairingState::Cancelled
                | ProductPairingState::Failed
        ) {
            return Err(ProductPairingError::InvalidState);
        }

        self.bootstrap = None;
        self.flow = None;
        self.owner_root = None;
        self.device_signing = None;
        self.accepted_credential = None;
        self.state = ProductPairingState::Cancelled;
        Ok(())
    }

    fn fail<T>(&mut self, error: ProductPairingError) -> Result<T, ProductPairingError> {
        if !matches!(
            self.state,
            ProductPairingState::Complete
                | ProductPairingState::Cancelled
                | ProductPairingState::Failed
        ) {
            self.state = ProductPairingState::Failed;
        }
        Err(error)
    }
}
