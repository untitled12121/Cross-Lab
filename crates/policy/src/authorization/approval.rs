use core::fmt;

use crosslab_crypto::{CanonicalTranscript, Signature, SigningKey};
use crosslab_identity::{AuthorityDelegation, AuthorityRole, KeyId, OwnerId, OwnerRootRecord};

use super::{ApprovalScope, Obligation};

const DOMAIN: &str = "crosslab.owner-approval.v1";

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ApprovalInstant(u64);

impl ApprovalInstant {
    pub const fn from_ticks(ticks: u64) -> Self {
        Self(ticks)
    }

    pub const fn ticks(self) -> u64 {
        self.0
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ApprovalError {
    InvalidLifetime,
    WrongIssuerRole,
    WrongOwner,
    InvalidDelegation,
    UnknownIssuer,
    InvalidSignature,
}

impl fmt::Display for ApprovalError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::InvalidLifetime => "owner approval lifetime is invalid",
            Self::WrongIssuerRole => "issuer role cannot authorize owner approval",
            Self::WrongOwner => "owner approval belongs to a different owner",
            Self::InvalidDelegation => "owner approval delegation is invalid",
            Self::UnknownIssuer => "owner approval issuer is not the expected authority",
            Self::InvalidSignature => "owner approval signature is invalid",
        })
    }
}

impl std::error::Error for ApprovalError {}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OwnerApprovalEvidence {
    schema_version: u16,
    owner_id: OwnerId,
    scope: ApprovalScope,
    issued_at: ApprovalInstant,
    expires_at: ApprovalInstant,
    issuer_key_id: KeyId,
    issuer_delegation_epoch: u64,
    signature: Signature,
}

impl OwnerApprovalEvidence {
    #[allow(clippy::too_many_arguments)]
    pub fn issue(
        scope: ApprovalScope,
        issued_at: ApprovalInstant,
        expires_at: ApprovalInstant,
        root: &OwnerRootRecord,
        delegation: &AuthorityDelegation,
        issuer_key: &SigningKey,
        minimum_delegation_epoch: u64,
    ) -> Result<Self, ApprovalError> {
        validate_lifetime(issued_at, expires_at)?;
        validate_delegation(root, delegation, minimum_delegation_epoch)?;
        if issuer_key.verifying_key() != delegation.delegated_public_key() {
            return Err(ApprovalError::UnknownIssuer);
        }

        let mut evidence = Self {
            schema_version: 1,
            owner_id: root.owner_id(),
            scope,
            issued_at,
            expires_at,
            issuer_key_id: delegation.delegated_key_id(),
            issuer_delegation_epoch: delegation.delegation_epoch(),
            signature: Signature::from_bytes([0; 64]),
        };
        evidence.signature = issuer_key.sign_digest(&evidence.transcript_digest());
        Ok(evidence)
    }

    pub fn verify(
        &self,
        root: &OwnerRootRecord,
        delegation: &AuthorityDelegation,
        minimum_delegation_epoch: u64,
    ) -> Result<VerifiedApproval, ApprovalError> {
        if self.schema_version != 1 {
            return Err(ApprovalError::InvalidSignature);
        }
        validate_lifetime(self.issued_at, self.expires_at)?;
        if self.owner_id != root.owner_id() || delegation.owner_id() != self.owner_id {
            return Err(ApprovalError::WrongOwner);
        }
        validate_delegation(root, delegation, minimum_delegation_epoch)?;
        if self.issuer_key_id != delegation.delegated_key_id()
            || self.issuer_delegation_epoch != delegation.delegation_epoch()
        {
            return Err(ApprovalError::UnknownIssuer);
        }
        delegation
            .delegated_public_key()
            .verify_digest(&self.transcript_digest(), &self.signature)
            .map_err(|_| ApprovalError::InvalidSignature)?;

        Ok(VerifiedApproval {
            obligation: Obligation::OwnerConfirmation,
            scope: self.scope.clone(),
            valid_from: self.issued_at,
            expires_at: self.expires_at,
        })
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
            .push(3, self.scope.source_device_id.to_bytes())
            .expect("fixed field is valid");
        transcript
            .push(4, self.scope.destination_device_id.to_bytes())
            .expect("fixed field is valid");
        transcript
            .push(5, self.scope.session_id.to_bytes())
            .expect("fixed field is valid");
        transcript
            .push(6, self.scope.capability_id.as_str().as_bytes())
            .expect("validated capability identifier is valid");
        transcript
            .push(7, self.scope.operation.as_str().as_bytes())
            .expect("validated operation identifier is valid");
        transcript
            .push(8, self.issued_at.ticks().to_be_bytes())
            .expect("fixed field is valid");
        transcript
            .push(9, self.expires_at.ticks().to_be_bytes())
            .expect("fixed field is valid");
        transcript
            .push(10, self.issuer_key_id.to_bytes())
            .expect("fixed field is valid");
        transcript
            .push(11, self.issuer_delegation_epoch.to_be_bytes())
            .expect("fixed field is valid");
        transcript.digest()
    }

    pub const fn issued_at(&self) -> ApprovalInstant {
        self.issued_at
    }

    pub const fn expires_at(&self) -> ApprovalInstant {
        self.expires_at
    }
}

/// A verified approval can only be obtained by validating signed local evidence.
///
/// ```compile_fail
/// use crosslab_policy::{ApprovalScope, VerifiedApproval};
/// # fn cannot_mint(scope: ApprovalScope) {
/// let _ = VerifiedApproval::owner_confirmation(scope);
/// # }
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VerifiedApproval {
    obligation: Obligation,
    scope: ApprovalScope,
    valid_from: ApprovalInstant,
    expires_at: ApprovalInstant,
}

impl VerifiedApproval {
    pub(super) fn satisfies(
        &self,
        obligation: Obligation,
        scope: &ApprovalScope,
        now: ApprovalInstant,
    ) -> bool {
        self.obligation == obligation
            && self.scope == *scope
            && self.valid_from <= now
            && now < self.expires_at
    }
}

fn validate_lifetime(
    issued_at: ApprovalInstant,
    expires_at: ApprovalInstant,
) -> Result<(), ApprovalError> {
    if issued_at >= expires_at {
        return Err(ApprovalError::InvalidLifetime);
    }
    Ok(())
}

fn validate_delegation(
    root: &OwnerRootRecord,
    delegation: &AuthorityDelegation,
    minimum_delegation_epoch: u64,
) -> Result<(), ApprovalError> {
    if delegation.role() != AuthorityRole::Administrative {
        return Err(ApprovalError::WrongIssuerRole);
    }
    if delegation.owner_id() != root.owner_id() {
        return Err(ApprovalError::WrongOwner);
    }
    delegation
        .verify(root, minimum_delegation_epoch)
        .map_err(|_| ApprovalError::InvalidDelegation)
}
