use core::fmt;

use crosslab_core::PairingInstant;
use crosslab_crypto::SigningProvider;
use crosslab_identity::{AuthorityRole, OwnerAuthorityState};
use crosslab_policy::TransitionId;
use crosslab_protocol::{
    PairingRole, ProductPairingAck, ProductPairingAckKind, ProductPairingCredentialBundle,
    ProductPairingMessage, ProductPairingTrustBundleMessage,
};

use super::{
    ProductPairingCommit, ProductPairingError, ProductPairingInviter,
    ProductPairingInviterCompletion, ProductPairingJoiner, ProductPairingJoinerCompletion,
    ProductPairingTrustBundle,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProductPairingExchangeState {
    AwaitingPeerHello,
    AwaitingPeerConfirmation,
    AwaitingCredential,
    AwaitingCredentialAcceptance,
    AwaitingPeerTrust,
    AwaitingLocalPersistence,
    AwaitingPeerPersistence,
    AwaitingCompletion,
    Complete,
    Cancelled,
    Failed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProductPairingNetworkError {
    UnexpectedMessage,
    PeerCancelled,
    Pairing(ProductPairingError),
}

impl fmt::Display for ProductPairingNetworkError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnexpectedMessage => {
                formatter.write_str("product pairing message is out of sequence")
            }
            Self::PeerCancelled => formatter.write_str("peer cancelled product pairing"),
            Self::Pairing(error) => fmt::Display::fmt(error, formatter),
        }
    }
}

impl std::error::Error for ProductPairingNetworkError {}

impl From<ProductPairingError> for ProductPairingNetworkError {
    fn from(error: ProductPairingError) -> Self {
        Self::Pairing(error)
    }
}

pub struct ProductPairingInviterExchange {
    pairing: ProductPairingInviter,
    pairing_id: [u8; 16],
    completion: Option<ProductPairingInviterCompletion>,
    state: ProductPairingExchangeState,
}

impl ProductPairingInviterExchange {
    pub fn new(pairing: ProductPairingInviter) -> Self {
        let pairing_id = pairing.hello().pairing_id();
        Self {
            pairing,
            pairing_id,
            completion: None,
            state: ProductPairingExchangeState::AwaitingPeerHello,
        }
    }

    pub const fn state(&self) -> ProductPairingExchangeState {
        self.state
    }

    pub fn accept_joiner_hello(
        &mut self,
        message: ProductPairingMessage,
        now: PairingInstant,
    ) -> Result<ProductPairingMessage, ProductPairingNetworkError> {
        self.require(ProductPairingExchangeState::AwaitingPeerHello)?;
        let hello = match message {
            ProductPairingMessage::Hello(hello) => hello,
            ProductPairingMessage::Cancel { pairing_id } if pairing_id == self.pairing_id => {
                return self.peer_cancelled();
            }
            _ => return self.fail(ProductPairingNetworkError::UnexpectedMessage),
        };

        if let Err(error) = self.pairing.accept_joiner_hello(hello, now) {
            return self.fail(error.into());
        }
        self.state = ProductPairingExchangeState::AwaitingPeerConfirmation;
        Ok(ProductPairingMessage::Hello(self.pairing.hello()))
    }

    pub fn accept_joiner_confirmation(
        &mut self,
        message: ProductPairingMessage,
        now: PairingInstant,
    ) -> Result<ProductPairingMessage, ProductPairingNetworkError> {
        self.require(ProductPairingExchangeState::AwaitingPeerConfirmation)?;
        let confirmation = match message {
            ProductPairingMessage::Confirmation(confirmation) => confirmation,
            ProductPairingMessage::Cancel { pairing_id } if pairing_id == self.pairing_id => {
                return self.peer_cancelled();
            }
            _ => return self.fail(ProductPairingNetworkError::UnexpectedMessage),
        };

        let response = match self
            .pairing
            .verify_joiner_confirmation(&confirmation, now)
        {
            Ok(response) => response,
            Err(error) => return self.fail(error.into()),
        };
        self.state = ProductPairingExchangeState::AwaitingCredential;
        Ok(ProductPairingMessage::Confirmation(response))
    }

    pub fn issue_credential_bundle(
        &mut self,
        authority: &OwnerAuthorityState,
        issuer: &dyn SigningProvider,
        now: PairingInstant,
    ) -> Result<ProductPairingMessage, ProductPairingNetworkError> {
        self.require(ProductPairingExchangeState::AwaitingCredential)?;
        let credential = match self
            .pairing
            .issue_joiner_credential(authority, issuer, now)
        {
            Ok(credential) => credential,
            Err(error) => return self.fail(error.into()),
        };
        let device_signing = match authority.current_delegation(AuthorityRole::DeviceSigning) {
            Ok(delegation) => *delegation,
            Err(error) => return self.fail(ProductPairingError::Identity(error).into()),
        };
        self.state = ProductPairingExchangeState::AwaitingCredentialAcceptance;
        Ok(ProductPairingMessage::CredentialBundle(Box::new(
            ProductPairingCredentialBundle::new(*authority.root(), device_signing, credential),
        )))
    }

    pub fn accept_credential_proof(
        &mut self,
        message: ProductPairingMessage,
        authority: &OwnerAuthorityState,
        issuer: &dyn SigningProvider,
        now: PairingInstant,
    ) -> Result<ProductPairingCommit, ProductPairingNetworkError> {
        self.require(ProductPairingExchangeState::AwaitingCredentialAcceptance)?;
        let accepted = match message {
            ProductPairingMessage::CredentialAccepted(accepted) => accepted,
            ProductPairingMessage::Cancel { pairing_id } if pairing_id == self.pairing_id => {
                return self.peer_cancelled();
            }
            _ => return self.fail(ProductPairingNetworkError::UnexpectedMessage),
        };
        let transition_id =
            TransitionId::generate().map_err(|_| ProductPairingError::Random)?;
        let completion = match self.pairing.verify_credential_acceptance(
            &accepted,
            transition_id,
            authority,
            issuer,
            now,
        ) {
            Ok(completion) => completion,
            Err(error) => return self.fail(error.into()),
        };
        let commit = completion.commit();
        self.completion = Some(completion);
        self.state = ProductPairingExchangeState::AwaitingLocalPersistence;
        Ok(commit)
    }

    pub fn local_persisted(
        &mut self,
    ) -> Result<ProductPairingMessage, ProductPairingNetworkError> {
        self.require(ProductPairingExchangeState::AwaitingLocalPersistence)?;
        if let Err(error) = self.pairing.mark_persisted() {
            return self.fail(error.into());
        }
        let completion = self
            .completion
            .ok_or(ProductPairingNetworkError::UnexpectedMessage)?;
        let trust = completion.reciprocal_trust();
        self.state = ProductPairingExchangeState::AwaitingPeerPersistence;
        Ok(ProductPairingMessage::TrustBundle(Box::new(
            ProductPairingTrustBundleMessage::new(trust.credential(), trust.transition()),
        )))
    }

    pub fn persistence_failed(&mut self) -> Result<(), ProductPairingNetworkError> {
        self.require(ProductPairingExchangeState::AwaitingLocalPersistence)?;
        if let Err(error) = self.pairing.persistence_failed() {
            return self.fail(error.into());
        }
        self.state = ProductPairingExchangeState::Failed;
        Ok(())
    }

    pub fn accept_peer_persisted(
        &mut self,
        message: ProductPairingMessage,
    ) -> Result<ProductPairingMessage, ProductPairingNetworkError> {
        self.require(ProductPairingExchangeState::AwaitingPeerPersistence)?;
        match message {
            ProductPairingMessage::Ack(ack)
                if ack.pairing_id() == self.pairing_id
                    && ack.role() == PairingRole::Joiner
                    && ack.kind() == ProductPairingAckKind::Persisted => {}
            ProductPairingMessage::Cancel { pairing_id } if pairing_id == self.pairing_id => {
                return self.peer_cancelled();
            }
            _ => return self.fail(ProductPairingNetworkError::UnexpectedMessage),
        }

        self.state = ProductPairingExchangeState::Complete;
        Ok(ProductPairingMessage::Ack(ProductPairingAck::new(
            self.pairing_id,
            PairingRole::Inviter,
            ProductPairingAckKind::Complete,
        )))
    }

    pub fn cancel(&mut self) -> Result<ProductPairingMessage, ProductPairingNetworkError> {
        if matches!(
            self.state,
            ProductPairingExchangeState::Complete
                | ProductPairingExchangeState::Cancelled
                | ProductPairingExchangeState::Failed
        ) {
            return Err(ProductPairingNetworkError::UnexpectedMessage);
        }
        self.pairing.cancel()?;
        self.state = ProductPairingExchangeState::Cancelled;
        Ok(ProductPairingMessage::Cancel {
            pairing_id: self.pairing_id,
        })
    }

    fn require(
        &mut self,
        expected: ProductPairingExchangeState,
    ) -> Result<(), ProductPairingNetworkError> {
        if self.state == expected {
            Ok(())
        } else {
            self.fail(ProductPairingNetworkError::UnexpectedMessage)
        }
    }

    fn peer_cancelled<T>(&mut self) -> Result<T, ProductPairingNetworkError> {
        let _ = self.pairing.cancel();
        self.state = ProductPairingExchangeState::Cancelled;
        Err(ProductPairingNetworkError::PeerCancelled)
    }

    fn fail<T>(
        &mut self,
        error: ProductPairingNetworkError,
    ) -> Result<T, ProductPairingNetworkError> {
        if !matches!(
            self.state,
            ProductPairingExchangeState::Complete
                | ProductPairingExchangeState::Cancelled
                | ProductPairingExchangeState::Failed
        ) {
            let _ = self.pairing.cancel();
            self.state = ProductPairingExchangeState::Failed;
        }
        Err(error)
    }
}

pub struct ProductPairingJoinerExchange {
    pairing: ProductPairingJoiner,
    pairing_id: [u8; 16],
    expected_owner: crosslab_identity::OwnerId,
    authority: Option<OwnerAuthorityState>,
    completion: Option<ProductPairingJoinerCompletion>,
    state: ProductPairingExchangeState,
}

impl ProductPairingJoinerExchange {
    pub fn new(pairing: ProductPairingJoiner) -> Self {
        let hello = pairing.hello();
        Self {
            pairing,
            pairing_id: hello.pairing_id(),
            expected_owner: hello.owner_id(),
            authority: None,
            completion: None,
            state: ProductPairingExchangeState::AwaitingPeerHello,
        }
    }

    pub const fn state(&self) -> ProductPairingExchangeState {
        self.state
    }

    pub fn hello(&self) -> ProductPairingMessage {
        ProductPairingMessage::Hello(self.pairing.hello())
    }

    pub fn accept_inviter_hello(
        &mut self,
        message: ProductPairingMessage,
    ) -> Result<ProductPairingMessage, ProductPairingNetworkError> {
        self.require(ProductPairingExchangeState::AwaitingPeerHello)?;
        let hello = match message {
            ProductPairingMessage::Hello(hello) => hello,
            ProductPairingMessage::Cancel { pairing_id } if pairing_id == self.pairing_id => {
                return self.peer_cancelled();
            }
            _ => return self.fail(ProductPairingNetworkError::UnexpectedMessage),
        };
        let confirmation = match self.pairing.accept_inviter_hello(hello) {
            Ok(confirmation) => confirmation,
            Err(error) => return self.fail(error.into()),
        };
        self.state = ProductPairingExchangeState::AwaitingPeerConfirmation;
        Ok(ProductPairingMessage::Confirmation(confirmation))
    }

    pub fn accept_inviter_confirmation(
        &mut self,
        message: ProductPairingMessage,
    ) -> Result<(), ProductPairingNetworkError> {
        self.require(ProductPairingExchangeState::AwaitingPeerConfirmation)?;
        let confirmation = match message {
            ProductPairingMessage::Confirmation(confirmation) => confirmation,
            ProductPairingMessage::Cancel { pairing_id } if pairing_id == self.pairing_id => {
                return self.peer_cancelled();
            }
            _ => return self.fail(ProductPairingNetworkError::UnexpectedMessage),
        };
        if let Err(error) = self.pairing.verify_inviter_confirmation(&confirmation) {
            return self.fail(error.into());
        }
        self.state = ProductPairingExchangeState::AwaitingCredential;
        Ok(())
    }

    pub fn accept_credential_bundle(
        &mut self,
        message: ProductPairingMessage,
        local_signer: &dyn SigningProvider,
    ) -> Result<ProductPairingMessage, ProductPairingNetworkError> {
        self.require(ProductPairingExchangeState::AwaitingCredential)?;
        let bundle = match message {
            ProductPairingMessage::CredentialBundle(bundle) => bundle,
            ProductPairingMessage::Cancel { pairing_id } if pairing_id == self.pairing_id => {
                return self.peer_cancelled();
            }
            _ => return self.fail(ProductPairingNetworkError::UnexpectedMessage),
        };

        if bundle.owner_root().owner_id() != self.expected_owner
            || bundle.device_signing().owner_id() != self.expected_owner
        {
            return self.fail(ProductPairingError::BootstrapPeerMismatch.into());
        }

        let mut authority = OwnerAuthorityState::new(bundle.owner_root());
        if let Err(error) = authority.accept_delegation(bundle.device_signing()) {
            return self.fail(ProductPairingError::Identity(error).into());
        }
        let accepted = match self
            .pairing
            .accept_credential(&authority, &bundle.credential(), local_signer)
        {
            Ok(accepted) => accepted,
            Err(error) => return self.fail(error.into()),
        };
        self.authority = Some(authority);
        self.state = ProductPairingExchangeState::AwaitingPeerTrust;
        Ok(ProductPairingMessage::CredentialAccepted(accepted))
    }

    pub fn accept_trust_bundle(
        &mut self,
        message: ProductPairingMessage,
    ) -> Result<ProductPairingJoinerCompletion, ProductPairingNetworkError> {
        self.require(ProductPairingExchangeState::AwaitingPeerTrust)?;
        let bundle = match message {
            ProductPairingMessage::TrustBundle(bundle) => bundle,
            ProductPairingMessage::Cancel { pairing_id } if pairing_id == self.pairing_id => {
                return self.peer_cancelled();
            }
            _ => return self.fail(ProductPairingNetworkError::UnexpectedMessage),
        };
        let authority = self
            .authority
            .as_ref()
            .ok_or(ProductPairingNetworkError::UnexpectedMessage)?;
        let trust = ProductPairingTrustBundle::new(bundle.credential(), bundle.transition());
        let completion = match self.pairing.accept_inviter_trust(trust, authority) {
            Ok(completion) => completion,
            Err(error) => return self.fail(error.into()),
        };
        self.completion = Some(completion);
        self.state = ProductPairingExchangeState::AwaitingLocalPersistence;
        Ok(completion)
    }

    pub fn local_persisted(
        &mut self,
    ) -> Result<ProductPairingMessage, ProductPairingNetworkError> {
        self.require(ProductPairingExchangeState::AwaitingLocalPersistence)?;
        if let Err(error) = self.pairing.mark_persisted() {
            return self.fail(error.into());
        }
        self.state = ProductPairingExchangeState::AwaitingCompletion;
        Ok(ProductPairingMessage::Ack(ProductPairingAck::new(
            self.pairing_id,
            PairingRole::Joiner,
            ProductPairingAckKind::Persisted,
        )))
    }

    pub fn persistence_failed(&mut self) -> Result<(), ProductPairingNetworkError> {
        self.require(ProductPairingExchangeState::AwaitingLocalPersistence)?;
        if let Err(error) = self.pairing.persistence_failed() {
            return self.fail(error.into());
        }
        self.state = ProductPairingExchangeState::Failed;
        Ok(())
    }

    pub fn accept_complete(
        &mut self,
        message: ProductPairingMessage,
    ) -> Result<(), ProductPairingNetworkError> {
        self.require(ProductPairingExchangeState::AwaitingCompletion)?;
        match message {
            ProductPairingMessage::Ack(ack)
                if ack.pairing_id() == self.pairing_id
                    && ack.role() == PairingRole::Inviter
                    && ack.kind() == ProductPairingAckKind::Complete => {}
            ProductPairingMessage::Cancel { pairing_id } if pairing_id == self.pairing_id => {
                return self.peer_cancelled();
            }
            _ => return self.fail(ProductPairingNetworkError::UnexpectedMessage),
        }
        self.state = ProductPairingExchangeState::Complete;
        Ok(())
    }

    pub fn cancel(&mut self) -> Result<ProductPairingMessage, ProductPairingNetworkError> {
        if matches!(
            self.state,
            ProductPairingExchangeState::Complete
                | ProductPairingExchangeState::Cancelled
                | ProductPairingExchangeState::Failed
        ) {
            return Err(ProductPairingNetworkError::UnexpectedMessage);
        }
        self.pairing.cancel()?;
        self.state = ProductPairingExchangeState::Cancelled;
        Ok(ProductPairingMessage::Cancel {
            pairing_id: self.pairing_id,
        })
    }

    fn require(
        &mut self,
        expected: ProductPairingExchangeState,
    ) -> Result<(), ProductPairingNetworkError> {
        if self.state == expected {
            Ok(())
        } else {
            self.fail(ProductPairingNetworkError::UnexpectedMessage)
        }
    }

    fn peer_cancelled<T>(&mut self) -> Result<T, ProductPairingNetworkError> {
        let _ = self.pairing.cancel();
        self.state = ProductPairingExchangeState::Cancelled;
        Err(ProductPairingNetworkError::PeerCancelled)
    }

    fn fail<T>(
        &mut self,
        error: ProductPairingNetworkError,
    ) -> Result<T, ProductPairingNetworkError> {
        if !matches!(
            self.state,
            ProductPairingExchangeState::Complete
                | ProductPairingExchangeState::Cancelled
                | ProductPairingExchangeState::Failed
        ) {
            let _ = self.pairing.cancel();
            self.state = ProductPairingExchangeState::Failed;
        }
        Err(error)
    }
}
