use core::fmt;

use crosslab_crypto::{
    CanonicalTranscript, SignatureAlgorithm, SigningKey, VerifyingKey, signed_object_digest,
};
use crosslab_identity::{
    AuthorityDelegation, DeviceCredential, DeviceId, IdentityError, KeyId, OwnerId, OwnerRootRecord,
};
use crosslab_policy::{TransitionId, TrustRecord};
use crosslab_protocol::{
    PairingConfirmation, PairingCredentialAccepted, PairingHello, PairingRole,
};

use super::{
    PairingConfirmationRole, PairingId, PairingInvitation, PairingInvitationState, PairingSecret,
    PairingTranscript,
};

const PROTOCOL_MAJOR_V1: u16 = 1;
const CREDENTIAL_ACCEPTANCE_DOMAIN: &str = "crosslab.pairing-credential-accepted.v1";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PairingInviterState {
    AwaitingJoinerConfirmation,
    ReadyToIssueCredential,
    AwaitingCredentialAcceptance,
    Trusted,
    Failed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PairingJoinerState {
    AwaitingInviterConfirmation,
    AwaitingCredential,
    Accepted,
    Failed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PairingFlowError {
    InvitationNotPending,
    InvalidPairingContext,
    InvalidConfirmation,
    UnexpectedState,
    CredentialMismatch,
    JoinerKeyMismatch,
    InvalidCredentialAcceptance,
    Identity(IdentityError),
}

impl fmt::Display for PairingFlowError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::InvitationNotPending => "pairing invitation is no longer pending",
            Self::InvalidPairingContext => "pairing hello context is inconsistent",
            Self::InvalidConfirmation => "pairing confirmation verification failed",
            Self::UnexpectedState => "pairing flow received a message in an unexpected state",
            Self::CredentialMismatch => "device credential does not match the confirmed pairing",
            Self::JoinerKeyMismatch => "joiner private key does not match the issued credential",
            Self::InvalidCredentialAcceptance => "credential acceptance proof verification failed",
            Self::Identity(error) => return fmt::Display::fmt(error, formatter),
        })
    }
}

impl std::error::Error for PairingFlowError {}

impl From<IdentityError> for PairingFlowError {
    fn from(error: IdentityError) -> Self {
        Self::Identity(error)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct PairingContext {
    pairing_id: PairingId,
    owner_id: OwnerId,
    inviter_device_id: DeviceId,
    joiner_device_id: DeviceId,
    joiner_device_key: VerifyingKey,
    transcript: PairingTranscript,
}

impl PairingContext {
    fn new(inviter: PairingHello, joiner: PairingHello) -> Result<Self, PairingFlowError> {
        if inviter.role() != PairingRole::Inviter
            || joiner.role() != PairingRole::Joiner
            || inviter.protocol_major() != PROTOCOL_MAJOR_V1
            || joiner.protocol_major() != PROTOCOL_MAJOR_V1
            || inviter.protocol_major() != joiner.protocol_major()
            || inviter.pairing_id() != joiner.pairing_id()
            || inviter.owner_id() != joiner.owner_id()
            || inviter.device_id() == joiner.device_id()
        {
            return Err(PairingFlowError::InvalidPairingContext);
        }

        let pairing_id = PairingId::from_bytes(inviter.pairing_id());
        let transcript = PairingTranscript::new(
            inviter.protocol_major(),
            pairing_id,
            inviter.owner_id(),
            inviter.device_id(),
            inviter.device_public_key(),
            inviter.nonce(),
            joiner.device_id(),
            joiner.device_public_key(),
            joiner.nonce(),
        );

        Ok(Self {
            pairing_id,
            owner_id: inviter.owner_id(),
            inviter_device_id: inviter.device_id(),
            joiner_device_id: joiner.device_id(),
            joiner_device_key: joiner.device_public_key(),
            transcript,
        })
    }
}

#[derive(Debug)]
pub struct PairingInviterFlow {
    invitation: PairingInvitation,
    context: PairingContext,
    state: PairingInviterState,
    issued_credential: Option<DeviceCredential>,
}

impl PairingInviterFlow {
    pub fn new(
        mut invitation: PairingInvitation,
        inviter: PairingHello,
        joiner: PairingHello,
    ) -> Result<Self, PairingFlowError> {
        if invitation.state() != PairingInvitationState::Pending {
            return Err(PairingFlowError::InvitationNotPending);
        }

        let context = match PairingContext::new(inviter, joiner) {
            Ok(context) => context,
            Err(error) => {
                let _ = invitation.consume();
                return Err(error);
            }
        };

        if invitation.pairing_id() != context.pairing_id
            || invitation.owner_id() != context.owner_id
            || invitation.inviter_device_id() != context.inviter_device_id
        {
            let _ = invitation.consume();
            return Err(PairingFlowError::InvalidPairingContext);
        }

        Ok(Self {
            invitation,
            context,
            state: PairingInviterState::AwaitingJoinerConfirmation,
            issued_credential: None,
        })
    }

    pub const fn state(&self) -> PairingInviterState {
        self.state
    }

    pub const fn invitation_state(&self) -> PairingInvitationState {
        self.invitation.state()
    }

    pub fn verify_joiner_confirmation(
        &mut self,
        confirmation: &PairingConfirmation,
    ) -> Result<PairingConfirmation, PairingFlowError> {
        if self.state != PairingInviterState::AwaitingJoinerConfirmation {
            return self.fail(PairingFlowError::UnexpectedState);
        }
        if confirmation.role() != PairingRole::Joiner
            || confirmation.pairing_id() != self.context.pairing_id.to_bytes()
            || self
                .context
                .transcript
                .verify_confirmation(
                    PairingConfirmationRole::Joiner,
                    self.invitation.secret(),
                    &confirmation.confirmation(),
                )
                .is_err()
        {
            return self.fail(PairingFlowError::InvalidConfirmation);
        }

        let value = self
            .context
            .transcript
            .confirmation(PairingConfirmationRole::Inviter, self.invitation.secret());
        self.state = PairingInviterState::ReadyToIssueCredential;
        Ok(PairingConfirmation::new(
            PairingRole::Inviter,
            self.context.pairing_id.to_bytes(),
            value,
        ))
    }

    pub fn issue_joiner_credential(
        &mut self,
        root: &OwnerRootRecord,
        issuer: &AuthorityDelegation,
        issuer_key: &SigningKey,
        credential_epoch: u64,
    ) -> Result<DeviceCredential, PairingFlowError> {
        if self.state != PairingInviterState::ReadyToIssueCredential {
            return self.fail(PairingFlowError::UnexpectedState);
        }

        let credential = match DeviceCredential::issue_for_public_key(
            self.context.owner_id,
            self.context.joiner_device_id,
            self.context.joiner_device_key,
            credential_epoch,
            root,
            issuer,
            issuer_key,
        ) {
            Ok(credential) => credential,
            Err(error) => return self.fail(PairingFlowError::Identity(error)),
        };

        self.issued_credential = Some(credential);
        self.state = PairingInviterState::AwaitingCredentialAcceptance;
        Ok(credential)
    }

    pub fn commit_trust(
        &mut self,
        accepted: &PairingCredentialAccepted,
        transition_id: TransitionId,
    ) -> Result<TrustRecord, PairingFlowError> {
        if self.state != PairingInviterState::AwaitingCredentialAcceptance {
            return self.fail(PairingFlowError::UnexpectedState);
        }
        let Some(credential) = self.issued_credential else {
            return self.fail(PairingFlowError::UnexpectedState);
        };

        let transcript_digest = self.context.transcript.digest();
        let credential_digest = signed_object_digest(
            credential.transcript_digest(),
            SignatureAlgorithm::Ed25519,
            &credential.signature(),
        );
        if accepted.pairing_id() != self.context.pairing_id.to_bytes()
            || accepted.pairing_transcript_digest() != transcript_digest
            || accepted.device_credential_signed_object_digest() != credential_digest
            || accepted.joiner_device_id() != self.context.joiner_device_id
            || accepted.joiner_device_key_id() != credential.device_key_id()
        {
            return self.fail(PairingFlowError::InvalidCredentialAcceptance);
        }

        let proof_digest = credential_acceptance_digest(
            accepted.pairing_transcript_digest(),
            accepted.device_credential_signed_object_digest(),
            accepted.joiner_device_id(),
            accepted.joiner_device_key_id(),
        );
        if credential
            .device_public_key()
            .verify_digest(&proof_digest, &accepted.signature())
            .is_err()
        {
            return self.fail(PairingFlowError::InvalidCredentialAcceptance);
        }

        if self.invitation.consume().is_err() {
            self.state = PairingInviterState::Failed;
            return Err(PairingFlowError::InvitationNotPending);
        }

        let trust = TrustRecord::trusted(
            self.context.owner_id,
            self.context.joiner_device_id,
            credential.credential_epoch(),
            transition_id,
        );
        self.state = PairingInviterState::Trusted;
        Ok(trust)
    }

    fn fail<T>(&mut self, error: PairingFlowError) -> Result<T, PairingFlowError> {
        if self.invitation.state() == PairingInvitationState::Pending {
            let _ = self.invitation.consume();
        }
        if self.state != PairingInviterState::Trusted {
            self.state = PairingInviterState::Failed;
        }
        Err(error)
    }
}

#[derive(Debug)]
pub struct PairingJoinerFlow {
    secret: PairingSecret,
    context: PairingContext,
    state: PairingJoinerState,
}

impl PairingJoinerFlow {
    pub fn new(
        secret: PairingSecret,
        inviter: PairingHello,
        joiner: PairingHello,
    ) -> Result<Self, PairingFlowError> {
        Ok(Self {
            secret,
            context: PairingContext::new(inviter, joiner)?,
            state: PairingJoinerState::AwaitingInviterConfirmation,
        })
    }

    pub const fn state(&self) -> PairingJoinerState {
        self.state
    }

    pub fn joiner_confirmation(&self) -> Result<PairingConfirmation, PairingFlowError> {
        if self.state != PairingJoinerState::AwaitingInviterConfirmation {
            return Err(PairingFlowError::UnexpectedState);
        }

        Ok(PairingConfirmation::new(
            PairingRole::Joiner,
            self.context.pairing_id.to_bytes(),
            self.context
                .transcript
                .confirmation(PairingConfirmationRole::Joiner, &self.secret),
        ))
    }

    pub fn verify_inviter_confirmation(
        &mut self,
        confirmation: &PairingConfirmation,
    ) -> Result<(), PairingFlowError> {
        if self.state != PairingJoinerState::AwaitingInviterConfirmation {
            return self.fail(PairingFlowError::UnexpectedState);
        }
        if confirmation.role() != PairingRole::Inviter
            || confirmation.pairing_id() != self.context.pairing_id.to_bytes()
            || self
                .context
                .transcript
                .verify_confirmation(
                    PairingConfirmationRole::Inviter,
                    &self.secret,
                    &confirmation.confirmation(),
                )
                .is_err()
        {
            return self.fail(PairingFlowError::InvalidConfirmation);
        }

        self.state = PairingJoinerState::AwaitingCredential;
        Ok(())
    }

    pub fn accept_credential(
        &mut self,
        root: &OwnerRootRecord,
        issuer: &AuthorityDelegation,
        credential: &DeviceCredential,
        joiner_key: &SigningKey,
    ) -> Result<PairingCredentialAccepted, PairingFlowError> {
        if self.state != PairingJoinerState::AwaitingCredential {
            return self.fail(PairingFlowError::UnexpectedState);
        }

        if let Err(error) = credential.verify(root, issuer, 0, 0) {
            return self.fail(PairingFlowError::Identity(error));
        }
        if credential.owner_id() != self.context.owner_id
            || credential.device_id() != self.context.joiner_device_id
            || credential.device_public_key() != self.context.joiner_device_key
        {
            return self.fail(PairingFlowError::CredentialMismatch);
        }
        if joiner_key.verifying_key() != credential.device_public_key() {
            return self.fail(PairingFlowError::JoinerKeyMismatch);
        }

        let transcript_digest = self.context.transcript.digest();
        let credential_digest = signed_object_digest(
            credential.transcript_digest(),
            SignatureAlgorithm::Ed25519,
            &credential.signature(),
        );
        let proof_digest = credential_acceptance_digest(
            transcript_digest,
            credential_digest,
            self.context.joiner_device_id,
            credential.device_key_id(),
        );
        let signature = joiner_key.sign_digest(&proof_digest);

        self.state = PairingJoinerState::Accepted;
        Ok(PairingCredentialAccepted::new(
            self.context.pairing_id.to_bytes(),
            transcript_digest,
            credential_digest,
            self.context.joiner_device_id,
            credential.device_key_id(),
            signature,
        ))
    }

    fn fail<T>(&mut self, error: PairingFlowError) -> Result<T, PairingFlowError> {
        if self.state != PairingJoinerState::Accepted {
            self.state = PairingJoinerState::Failed;
        }
        Err(error)
    }
}

fn credential_acceptance_digest(
    pairing_transcript_digest: [u8; 32],
    device_credential_signed_object_digest: [u8; 32],
    joiner_device_id: DeviceId,
    joiner_device_key_id: KeyId,
) -> [u8; 32] {
    let mut transcript = CanonicalTranscript::new(CREDENTIAL_ACCEPTANCE_DOMAIN)
        .expect("fixed pairing credential acceptance domain is valid");
    transcript
        .push(1, pairing_transcript_digest)
        .expect("pairing transcript digest field is valid");
    transcript
        .push(2, device_credential_signed_object_digest)
        .expect("device credential digest field is valid");
    transcript
        .push(3, joiner_device_id.to_bytes())
        .expect("joiner device id field is valid");
    transcript
        .push(4, joiner_device_key_id.to_bytes())
        .expect("joiner device key id field is valid");
    transcript.digest()
}
