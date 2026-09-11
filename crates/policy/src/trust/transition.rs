use core::fmt;

use crosslab_crypto::{CanonicalTranscript, Signature, SigningKey};
use crosslab_identity::{
    AuthorityDelegation, AuthorityRole, DeviceId, KeyId, OwnerId, OwnerRootRecord,
};

use super::{TransitionId, TrustError, TrustRecord, TrustState};

const DOMAIN: &str = "crosslab.trust-transition.v1";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u16)]
enum TrustTransitionAction {
    Revoke = 1,
}

impl TrustTransitionAction {
    const fn code(self) -> u16 {
        self as u16
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TrustTransitionError {
    UnsupportedSchema,
    WrongOwner,
    WrongDevice,
    WrongIssuerRole,
    UnknownIssuer,
    InvalidDelegation,
    InvalidSignature,
    InvalidRevision,
    CredentialEpochMismatch,
    AlreadyRevoked,
}

impl fmt::Display for TrustTransitionError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::UnsupportedSchema => "unsupported trust transition schema",
            Self::WrongOwner => "trust transition owner does not match",
            Self::WrongDevice => "trust transition device does not match",
            Self::WrongIssuerRole => "issuer role cannot authorize this trust transition",
            Self::UnknownIssuer => "trust transition issuer is not the expected authority",
            Self::InvalidDelegation => "trust transition authority delegation is invalid",
            Self::InvalidSignature => "trust transition signature is invalid",
            Self::InvalidRevision => "trust transition revision is invalid",
            Self::CredentialEpochMismatch => "trust transition credential epoch does not match",
            Self::AlreadyRevoked => "device trust is already revoked",
        })
    }
}

impl std::error::Error for TrustTransitionError {}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TrustTransition {
    schema_version: u16,
    owner_id: OwnerId,
    device_id: DeviceId,
    transition_id: TransitionId,
    previous_revision: u64,
    new_revision: u64,
    action: TrustTransitionAction,
    credential_epoch_context: u64,
    issuer_role: AuthorityRole,
    issuer_key_id: KeyId,
    signature: Signature,
}

impl TrustTransition {
    pub fn issue_root_revocation(
        record: &TrustRecord,
        transition_id: TransitionId,
        root: &OwnerRootRecord,
        root_key: &SigningKey,
    ) -> Result<Self, TrustTransitionError> {
        ensure_record_active(record)?;
        if record.owner_id() != root.owner_id() {
            return Err(TrustTransitionError::WrongOwner);
        }
        if root_key.verifying_key() != root.root_public_key() {
            return Err(TrustTransitionError::UnknownIssuer);
        }

        let mut transition = Self::unsigned_revocation(
            record,
            transition_id,
            AuthorityRole::OwnerRoot,
            root.root_key_id(),
        )?;
        transition.signature = root_key.sign_digest(&transition.transcript_digest());
        Ok(transition)
    }

    pub fn issue_delegated_revocation(
        record: &TrustRecord,
        transition_id: TransitionId,
        root: &OwnerRootRecord,
        delegation: &AuthorityDelegation,
        issuer_key: &SigningKey,
        minimum_delegation_epoch: u64,
    ) -> Result<Self, TrustTransitionError> {
        ensure_record_active(record)?;
        ensure_ordinary_delegated_role(delegation.role())?;
        if record.owner_id() != root.owner_id() || delegation.owner_id() != record.owner_id() {
            return Err(TrustTransitionError::WrongOwner);
        }
        delegation
            .verify(root, minimum_delegation_epoch)
            .map_err(|_| TrustTransitionError::InvalidDelegation)?;
        if issuer_key.verifying_key() != delegation.delegated_public_key() {
            return Err(TrustTransitionError::UnknownIssuer);
        }

        let mut transition = Self::unsigned_revocation(
            record,
            transition_id,
            delegation.role(),
            delegation.delegated_key_id(),
        )?;
        transition.signature = issuer_key.sign_digest(&transition.transcript_digest());
        Ok(transition)
    }

    #[allow(clippy::too_many_arguments)]
    pub fn from_signed_revocation(
        owner_id: OwnerId,
        device_id: DeviceId,
        transition_id: TransitionId,
        previous_revision: u64,
        new_revision: u64,
        credential_epoch_context: u64,
        issuer_role: AuthorityRole,
        issuer_key_id: KeyId,
        signature: Signature,
    ) -> Self {
        Self {
            schema_version: 1,
            owner_id,
            device_id,
            transition_id,
            previous_revision,
            new_revision,
            action: TrustTransitionAction::Revoke,
            credential_epoch_context,
            issuer_role,
            issuer_key_id,
            signature,
        }
    }

    pub fn apply_root(
        &self,
        record: &mut TrustRecord,
        root: &OwnerRootRecord,
    ) -> Result<(), TrustTransitionError> {
        self.validate_common(record)?;
        if self.issuer_role != AuthorityRole::OwnerRoot {
            return Err(TrustTransitionError::WrongIssuerRole);
        }
        if root.owner_id() != self.owner_id {
            return Err(TrustTransitionError::WrongOwner);
        }
        if root.root_key_id() != self.issuer_key_id {
            return Err(TrustTransitionError::UnknownIssuer);
        }
        root.root_public_key()
            .verify_digest(&self.transcript_digest(), &self.signature)
            .map_err(|_| TrustTransitionError::InvalidSignature)?;
        apply_verified_revocation(record, self.transition_id)
    }

    pub fn apply_delegated(
        &self,
        record: &mut TrustRecord,
        root: &OwnerRootRecord,
        delegation: &AuthorityDelegation,
        minimum_delegation_epoch: u64,
    ) -> Result<(), TrustTransitionError> {
        self.validate_common(record)?;
        ensure_ordinary_delegated_role(self.issuer_role)?;
        if root.owner_id() != self.owner_id || delegation.owner_id() != self.owner_id {
            return Err(TrustTransitionError::WrongOwner);
        }
        if delegation.role() != self.issuer_role
            || delegation.delegated_key_id() != self.issuer_key_id
        {
            return Err(TrustTransitionError::UnknownIssuer);
        }
        delegation
            .verify(root, minimum_delegation_epoch)
            .map_err(|_| TrustTransitionError::InvalidDelegation)?;
        delegation
            .delegated_public_key()
            .verify_digest(&self.transcript_digest(), &self.signature)
            .map_err(|_| TrustTransitionError::InvalidSignature)?;
        apply_verified_revocation(record, self.transition_id)
    }

    pub fn transcript_digest(&self) -> [u8; 32] {
        let mut transcript =
            CanonicalTranscript::new(DOMAIN).expect("static canonical domain is valid");
        transcript
            .push(1, self.schema_version.to_be_bytes())
            .expect("fixed field is valid");
        transcript
            .push(2, self.owner_id.to_bytes())
            .expect("fixed field is valid");
        transcript
            .push(3, self.device_id.to_bytes())
            .expect("fixed field is valid");
        transcript
            .push(4, self.transition_id.to_bytes())
            .expect("fixed field is valid");
        transcript
            .push(5, self.previous_revision.to_be_bytes())
            .expect("fixed field is valid");
        transcript
            .push(6, self.new_revision.to_be_bytes())
            .expect("fixed field is valid");
        transcript
            .push(7, self.action.code().to_be_bytes())
            .expect("fixed field is valid");
        transcript
            .push(8, self.credential_epoch_context.to_be_bytes())
            .expect("fixed field is valid");
        transcript
            .push(9, self.issuer_role.code().to_be_bytes())
            .expect("fixed field is valid");
        transcript
            .push(10, self.issuer_key_id.to_bytes())
            .expect("fixed field is valid");
        transcript.digest()
    }

    pub const fn transition_id(&self) -> TransitionId {
        self.transition_id
    }

    pub const fn previous_revision(&self) -> u64 {
        self.previous_revision
    }

    pub const fn new_revision(&self) -> u64 {
        self.new_revision
    }

    pub const fn credential_epoch_context(&self) -> u64 {
        self.credential_epoch_context
    }

    fn unsigned_revocation(
        record: &TrustRecord,
        transition_id: TransitionId,
        issuer_role: AuthorityRole,
        issuer_key_id: KeyId,
    ) -> Result<Self, TrustTransitionError> {
        let new_revision = record
            .trust_revision()
            .checked_add(1)
            .ok_or(TrustTransitionError::InvalidRevision)?;
        Ok(Self::from_signed_revocation(
            record.owner_id(),
            record.device_id(),
            transition_id,
            record.trust_revision(),
            new_revision,
            record.accepted_credential_epoch(),
            issuer_role,
            issuer_key_id,
            Signature::from_bytes([0; 64]),
        ))
    }

    fn validate_common(&self, record: &TrustRecord) -> Result<(), TrustTransitionError> {
        if self.schema_version != 1 {
            return Err(TrustTransitionError::UnsupportedSchema);
        }
        ensure_record_active(record)?;
        if self.owner_id != record.owner_id() {
            return Err(TrustTransitionError::WrongOwner);
        }
        if self.device_id != record.device_id() {
            return Err(TrustTransitionError::WrongDevice);
        }
        if self.credential_epoch_context != record.accepted_credential_epoch() {
            return Err(TrustTransitionError::CredentialEpochMismatch);
        }
        let expected_revision = record
            .trust_revision()
            .checked_add(1)
            .ok_or(TrustTransitionError::InvalidRevision)?;
        if self.previous_revision != record.trust_revision() || self.new_revision != expected_revision {
            return Err(TrustTransitionError::InvalidRevision);
        }
        Ok(())
    }
}

fn ensure_record_active(record: &TrustRecord) -> Result<(), TrustTransitionError> {
    if record.state() == TrustState::Revoked {
        return Err(TrustTransitionError::AlreadyRevoked);
    }
    Ok(())
}

fn ensure_ordinary_delegated_role(role: AuthorityRole) -> Result<(), TrustTransitionError> {
    match role {
        AuthorityRole::Administrative | AuthorityRole::DeviceSigning => Ok(()),
        AuthorityRole::OwnerRoot | AuthorityRole::Recovery => Err(TrustTransitionError::WrongIssuerRole),
    }
}

fn apply_verified_revocation(
    record: &mut TrustRecord,
    transition_id: TransitionId,
) -> Result<(), TrustTransitionError> {
    record.revoke(transition_id).map_err(|error| match error {
        TrustError::AlreadyRevoked => TrustTransitionError::AlreadyRevoked,
        TrustError::StaleCredentialEpoch | TrustError::UnexpectedCredentialEpoch => {
            TrustTransitionError::InvalidRevision
        }
    })
}
