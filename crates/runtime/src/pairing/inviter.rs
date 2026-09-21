use crosslab_core::{PairingFlowError, PairingInstant, PairingInvitation, PairingInviterFlow};
use crosslab_crypto::{SigningProvider, random_bytes};
use crosslab_identity::{DeviceCredential, OwnerAuthorityState};
use crosslab_policy::{PairingTrustTransition, TransitionId};
use crosslab_protocol::{
    PairingConfirmation, PairingCredentialAccepted, PairingHello, PairingRole,
};

use super::{
    PROTOCOL_MAJOR_V1, ProductPairingCommit, ProductPairingError, ProductPairingInviterCompletion,
    ProductPairingState, ProductPairingTrustBundle,
};

pub struct ProductPairingInviter {
    local_credential: DeviceCredential,
    hello: PairingHello,
    invitation: Option<PairingInvitation>,
    flow: Option<PairingInviterFlow>,
    issued_credential: Option<DeviceCredential>,
    state: ProductPairingState,
}

impl ProductPairingInviter {
    pub fn new(
        invitation: PairingInvitation,
        local_credential: DeviceCredential,
        authority: &OwnerAuthorityState,
    ) -> Result<Self, ProductPairingError> {
        let nonce = random_bytes().map_err(|_| ProductPairingError::Random)?;
        Self::with_nonce(invitation, local_credential, authority, nonce)
    }

    pub(super) fn with_nonce(
        invitation: PairingInvitation,
        local_credential: DeviceCredential,
        authority: &OwnerAuthorityState,
        nonce: [u8; 32],
    ) -> Result<Self, ProductPairingError> {
        local_credential.verify(authority, local_credential.credential_epoch())?;
        if invitation.owner_id() != local_credential.owner_id()
            || invitation.inviter_device_id() != local_credential.device_id()
        {
            return Err(ProductPairingError::LocalIdentityMismatch);
        }

        let hello = PairingHello::new(
            PairingRole::Inviter,
            PROTOCOL_MAJOR_V1,
            invitation.pairing_id().to_bytes(),
            invitation.owner_id(),
            invitation.inviter_device_id(),
            local_credential.device_public_key(),
            nonce,
        );

        Ok(Self {
            local_credential,
            hello,
            invitation: Some(invitation),
            flow: None,
            issued_credential: None,
            state: ProductPairingState::AwaitingPeerHello,
        })
    }

    pub const fn state(&self) -> ProductPairingState {
        self.state
    }

    pub const fn hello(&self) -> PairingHello {
        self.hello
    }

    pub const fn local_credential(&self) -> DeviceCredential {
        self.local_credential
    }

    pub fn accept_joiner_hello(
        &mut self,
        joiner: PairingHello,
        now: PairingInstant,
    ) -> Result<(), ProductPairingError> {
        if self.state != ProductPairingState::AwaitingPeerHello {
            return self.fail(ProductPairingError::InvalidState);
        }
        let invitation = self
            .invitation
            .take()
            .ok_or(ProductPairingError::InvalidState)?;
        let flow = match PairingInviterFlow::new(invitation, self.hello, joiner, now) {
            Ok(flow) => flow,
            Err(error) => return self.fail(error.into()),
        };
        self.flow = Some(flow);
        self.state = ProductPairingState::AwaitingPeerConfirmation;
        Ok(())
    }

    pub fn verify_joiner_confirmation(
        &mut self,
        confirmation: &PairingConfirmation,
        now: PairingInstant,
    ) -> Result<PairingConfirmation, ProductPairingError> {
        if self.state != ProductPairingState::AwaitingPeerConfirmation {
            return self.fail(ProductPairingError::InvalidState);
        }
        let result = self
            .flow
            .as_mut()
            .ok_or(ProductPairingError::InvalidState)?
            .verify_joiner_confirmation(confirmation, now);
        match result {
            Ok(response) => {
                self.state = ProductPairingState::AwaitingCredential;
                Ok(response)
            }
            Err(error) => self.fail(error.into()),
        }
    }

    pub fn issue_joiner_credential(
        &mut self,
        authority: &OwnerAuthorityState,
        issuer: &dyn SigningProvider,
        now: PairingInstant,
    ) -> Result<DeviceCredential, ProductPairingError> {
        if self.state != ProductPairingState::AwaitingCredential {
            return self.fail(ProductPairingError::InvalidState);
        }
        let result = self
            .flow
            .as_mut()
            .ok_or(ProductPairingError::InvalidState)?
            .issue_initial_joiner_credential_with_provider(authority, issuer, now);
        match result {
            Ok(credential) => {
                self.issued_credential = Some(credential);
                self.state = ProductPairingState::AwaitingCredentialAcceptance;
                Ok(credential)
            }
            Err(error) => self.fail(error.into()),
        }
    }

    pub fn verify_credential_acceptance(
        &mut self,
        accepted: &PairingCredentialAccepted,
        transition_id: TransitionId,
        authority: &OwnerAuthorityState,
        issuer: &dyn SigningProvider,
        now: PairingInstant,
    ) -> Result<ProductPairingInviterCompletion, ProductPairingError> {
        if self.state != ProductPairingState::AwaitingCredentialAcceptance {
            return self.fail(ProductPairingError::InvalidState);
        }
        let reciprocal_transition_id =
            TransitionId::generate().map_err(|_| ProductPairingError::Random)?;
        let credential = self
            .issued_credential
            .ok_or(ProductPairingError::InvalidState)?;
        let result = self
            .flow
            .as_mut()
            .ok_or(ProductPairingError::InvalidState)?
            .commit_trust_with_evidence_with_provider(
                accepted,
                transition_id,
                authority,
                issuer,
                now,
            );

        let establishment = match result {
            Ok(establishment) => establishment,
            Err(error) => return self.fail(error.into()),
        };
        let reciprocal_transition = match PairingTrustTransition::issue_with_provider(
            &self.local_credential,
            reciprocal_transition_id,
            establishment.transition().pairing_evidence_digest(),
            authority,
            issuer,
        ) {
            Ok(transition) => transition,
            Err(error) => {
                return self.fail(PairingFlowError::TrustTransition(error).into());
            }
        };

        self.state = ProductPairingState::AwaitingPersistence;
        Ok(ProductPairingInviterCompletion {
            commit: ProductPairingCommit {
                peer_credential: credential,
                peer_transition: establishment.transition(),
                peer_trust: establishment.trust(),
            },
            reciprocal_trust: ProductPairingTrustBundle {
                credential: self.local_credential,
                transition: reciprocal_transition,
            },
        })
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

        if let Some(invitation) = self.invitation.as_mut() {
            let _ = invitation.cancel();
        }
        self.invitation = None;
        self.flow = None;
        self.issued_credential = None;
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
