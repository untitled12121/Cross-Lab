use core::fmt;

use crosslab_core::{
    PairingBootstrap, PairingFlowError, PairingInstant, PairingInvitation, PairingInviterFlow,
    PairingJoinerFlow,
};
use crosslab_crypto::{SigningProvider, random_bytes};
use crosslab_identity::{DeviceCredential, DeviceId, IdentityError, OwnerAuthorityState};
use crosslab_policy::{TransitionId, TrustRecord};
use crosslab_protocol::{
    PairingConfirmation, PairingCredentialAccepted, PairingHello, PairingRole,
};

const PROTOCOL_MAJOR_V1: u16 = 1;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProductPairingState {
    AwaitingPeerHello,
    AwaitingPeerConfirmation,
    AwaitingCredential,
    AwaitingCredentialAcceptance,
    AwaitingPeerTrust,
    AwaitingPersistence,
    Complete,
    Cancelled,
    Failed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ProductPairingCommit {
    peer_credential: DeviceCredential,
    peer_trust: TrustRecord,
}

impl ProductPairingCommit {
    pub const fn peer_credential(&self) -> DeviceCredential {
        self.peer_credential
    }

    pub const fn peer_trust(&self) -> TrustRecord {
        self.peer_trust
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProductPairingError {
    Random,
    InvalidState,
    LocalIdentityMismatch,
    BootstrapPeerMismatch,
    Identity(IdentityError),
    Flow(PairingFlowError),
}

impl fmt::Display for ProductPairingError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Random => formatter.write_str("secure pairing randomness is unavailable"),
            Self::InvalidState => formatter.write_str("product pairing operation is out of sequence"),
            Self::LocalIdentityMismatch => {
                formatter.write_str("local product identity does not match the pairing invitation")
            }
            Self::BootstrapPeerMismatch => {
                formatter.write_str("pairing peer does not match the scanned bootstrap")
            }
            Self::Identity(error) => fmt::Display::fmt(error, formatter),
            Self::Flow(error) => fmt::Display::fmt(error, formatter),
        }
    }
}

impl std::error::Error for ProductPairingError {}

impl From<IdentityError> for ProductPairingError {
    fn from(error: IdentityError) -> Self {
        Self::Identity(error)
    }
}

impl From<PairingFlowError> for ProductPairingError {
    fn from(error: PairingFlowError) -> Self {
        Self::Flow(error)
    }
}

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

    fn with_nonce(
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
    ) -> Result<ProductPairingCommit, ProductPairingError> {
        if self.state != ProductPairingState::AwaitingCredentialAcceptance {
            return self.fail(ProductPairingError::InvalidState);
        }
        let credential = self
            .issued_credential
            .ok_or(ProductPairingError::InvalidState)?;
        let result = self
            .flow
            .as_mut()
            .ok_or(ProductPairingError::InvalidState)?
            .commit_trust_with_provider(accepted, transition_id, authority, issuer, now);
        match result {
            Ok(trust) => {
                self.state = ProductPairingState::AwaitingPersistence;
                Ok(ProductPairingCommit {
                    peer_credential: credential,
                    peer_trust: trust,
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
        self.state = ProductPairingState::Failed;
        Err(error)
    }
}

pub struct ProductPairingJoiner {
    hello: PairingHello,
    bootstrap: Option<PairingBootstrap>,
    flow: Option<PairingJoinerFlow>,
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

    fn with_nonce(
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
        let result = self
            .flow
            .as_mut()
            .ok_or(ProductPairingError::InvalidState)?
            .accept_credential_with_provider(authority, credential, local_signer);
        match result {
            Ok(accepted) => {
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
        self.accepted_credential = None;
        self.state = ProductPairingState::Cancelled;
        Ok(())
    }

    fn fail<T>(&mut self, error: ProductPairingError) -> Result<T, ProductPairingError> {
        self.state = ProductPairingState::Failed;
        Err(error)
    }
}

#[cfg(test)]
mod tests {
    use crosslab_core::{PairingInvitation, PairingSecret};
    use crosslab_crypto::SigningKey;
    use crosslab_identity::{
        AuthorityDelegation, AuthorityRole, OwnerId, OwnerRootRecord,
    };
    use crosslab_policy::TrustState;

    use super::*;

    struct Fixture {
        authority: OwnerAuthorityState,
        issuer: SigningKey,
        inviter_key: SigningKey,
        joiner_key: SigningKey,
        inviter_credential: DeviceCredential,
        invitation: PairingInvitation,
    }

    impl Fixture {
        fn new() -> Self {
            let owner_id = OwnerId::from_bytes([0x10; 32]);
            let root_key = SigningKey::from_secret_bytes([0x11; 32]);
            let root = OwnerRootRecord::new(owner_id, &root_key, 0);
            let issuer = SigningKey::from_secret_bytes([0x12; 32]);
            let delegation = AuthorityDelegation::issue(
                owner_id,
                AuthorityRole::DeviceSigning,
                &issuer,
                0,
                &root_key,
            );
            let mut authority = OwnerAuthorityState::new(root);
            authority.accept_delegation(delegation).unwrap();

            let inviter_key = SigningKey::from_secret_bytes([0x13; 32]);
            let inviter_device_id = DeviceId::from_bytes([0x14; 32]);
            let inviter_credential = DeviceCredential::issue(
                owner_id,
                inviter_device_id,
                &inviter_key,
                0,
                &authority,
                &issuer,
            )
            .unwrap();
            let invitation = PairingInvitation::from_parts(
                crosslab_core::PairingId::from_bytes([0x15; 16]),
                PairingSecret::from_bytes([0x16; 32]),
                owner_id,
                inviter_device_id,
                PairingInstant::from_ticks(0),
                PairingInstant::from_ticks(100),
            )
            .unwrap();

            Self {
                authority,
                issuer,
                inviter_key,
                joiner_key: SigningKey::from_secret_bytes([0x17; 32]),
                inviter_credential,
                invitation,
            }
        }

        fn coordinators(self) -> (ProductPairingInviter, ProductPairingJoiner) {
            let code = {
                let mut invitation = self.invitation;
                invitation
                    .bootstrap_code_at(PairingInstant::from_ticks(0))
                    .unwrap()
            };
            let bootstrap = PairingBootstrap::decode(code.as_str()).unwrap();
            let inviter = ProductPairingInviter::with_nonce(
                self.invitation_for_test(),
                self.inviter_credential,
                &self.authority,
                [0x18; 32],
            )
            .unwrap();
            let joiner = ProductPairingJoiner::with_nonce(
                bootstrap,
                DeviceId::from_bytes([0x19; 32]),
                &self.joiner_key,
                [0x1a; 32],
            )
            .unwrap();
            (inviter, joiner)
        }

        fn invitation_for_test(&self) -> PairingInvitation {
            PairingInvitation::from_parts(
                self.invitation.pairing_id(),
                PairingSecret::from_bytes([0x16; 32]),
                self.invitation.owner_id(),
                self.invitation.inviter_device_id(),
                PairingInstant::from_ticks(0),
                PairingInstant::from_ticks(100),
            )
            .unwrap()
        }
    }

    #[test]
    fn inviter_requires_persistence_before_product_completion() {
        let fixture = Fixture::new();
        let authority = fixture.authority.clone();
        let issuer = SigningKey::from_secret_bytes([0x12; 32]);
        let joiner_key = SigningKey::from_secret_bytes([0x17; 32]);
        let (mut inviter, mut joiner) = fixture.coordinators();

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
        let credential = inviter
            .issue_joiner_credential(&authority, &issuer, PairingInstant::from_ticks(30))
            .unwrap();
        let accepted = joiner
            .accept_credential(&authority, &credential, &joiner_key)
            .unwrap();
        let commit = inviter
            .verify_credential_acceptance(
                &accepted,
                TransitionId::from_bytes([0x1b; 32]),
                &authority,
                &issuer,
                PairingInstant::from_ticks(40),
            )
            .unwrap();

        assert_eq!(inviter.state(), ProductPairingState::AwaitingPersistence);
        assert_eq!(commit.peer_credential(), credential);
        assert_eq!(commit.peer_trust().state(), TrustState::Trusted);
        assert_eq!(commit.peer_trust().device_id(), credential.device_id());

        inviter.mark_persisted().unwrap();
        assert_eq!(inviter.state(), ProductPairingState::Complete);
    }

    #[test]
    fn scanned_bootstrap_binds_joiner_to_the_inviter_device() {
        let fixture = Fixture::new();
        let code = {
            let mut invitation = fixture.invitation_for_test();
            invitation
                .bootstrap_code_at(PairingInstant::from_ticks(0))
                .unwrap()
        };
        let bootstrap = PairingBootstrap::decode(code.as_str()).unwrap();
        let mut joiner = ProductPairingJoiner::with_nonce(
            bootstrap,
            DeviceId::from_bytes([0x19; 32]),
            &fixture.joiner_key,
            [0x1a; 32],
        )
        .unwrap();

        let substituted = PairingHello::new(
            PairingRole::Inviter,
            PROTOCOL_MAJOR_V1,
            joiner.hello().pairing_id(),
            joiner.hello().owner_id(),
            DeviceId::from_bytes([0xee; 32]),
            fixture.inviter_key.verifying_key(),
            [0xef; 32],
        );

        assert_eq!(
            joiner.accept_inviter_hello(substituted),
            Err(ProductPairingError::BootstrapPeerMismatch)
        );
        assert_eq!(joiner.state(), ProductPairingState::Failed);
    }

    #[test]
    fn persistence_failure_is_terminal() {
        let fixture = Fixture::new();
        let authority = fixture.authority.clone();
        let issuer = SigningKey::from_secret_bytes([0x12; 32]);
        let joiner_key = SigningKey::from_secret_bytes([0x17; 32]);
        let (mut inviter, mut joiner) = fixture.coordinators();

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
        let credential = inviter
            .issue_joiner_credential(&authority, &issuer, PairingInstant::from_ticks(30))
            .unwrap();
        let accepted = joiner
            .accept_credential(&authority, &credential, &joiner_key)
            .unwrap();
        inviter
            .verify_credential_acceptance(
                &accepted,
                TransitionId::from_bytes([0x1b; 32]),
                &authority,
                &issuer,
                PairingInstant::from_ticks(40),
            )
            .unwrap();

        inviter.persistence_failed().unwrap();
        assert_eq!(inviter.state(), ProductPairingState::Failed);
        assert_eq!(
            inviter.mark_persisted(),
            Err(ProductPairingError::InvalidState)
        );
    }

    #[test]
    fn cancellation_drops_uncommitted_pairing_state() {
        let fixture = Fixture::new();
        let (mut inviter, mut joiner) = fixture.coordinators();

        inviter.cancel().unwrap();
        joiner.cancel().unwrap();

        assert_eq!(inviter.state(), ProductPairingState::Cancelled);
        assert_eq!(joiner.state(), ProductPairingState::Cancelled);
        assert_eq!(
            inviter.accept_joiner_hello(joiner.hello(), PairingInstant::from_ticks(10)),
            Err(ProductPairingError::InvalidState)
        );
    }
}
