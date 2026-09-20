use crosslab_identity_store::{
    IdentityStoreAnchor, IdentityStoreEnvelope, IdentityStoreError, prepare_commit, validate_loaded,
};

#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct MobileIdentityCommit {
    pub previous_revision: Option<u64>,
    pub revision: u64,
    pub envelope: Vec<u8>,
    pub anchor: Vec<u8>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, uniffi::Error)]
pub enum MobileIdentityStoreError {
    UnsupportedSchema,
    MalformedEnvelope,
    MalformedAnchor,
    PayloadDigestMismatch,
    StaleOrMixedState,
    RevisionConflict,
    RevisionOverflow,
}

impl core::fmt::Display for MobileIdentityStoreError {
    fn fmt(&self, formatter: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        formatter.write_str(match self {
            Self::UnsupportedSchema => "identity-store schema is unsupported",
            Self::MalformedEnvelope => "identity-store envelope is malformed",
            Self::MalformedAnchor => "identity-store currentness anchor is malformed",
            Self::PayloadDigestMismatch => "identity-store payload digest does not match",
            Self::StaleOrMixedState => "identity-store currentness validation failed",
            Self::RevisionConflict => "identity-store revision changed concurrently",
            Self::RevisionOverflow => "identity-store revision is exhausted",
        })
    }
}

impl std::error::Error for MobileIdentityStoreError {}

#[uniffi::export]
pub fn identity_store_prepare_commit(
    current_envelope: Option<Vec<u8>>,
    payload: Vec<u8>,
) -> Result<MobileIdentityCommit, MobileIdentityStoreError> {
    let current = current_envelope
        .as_deref()
        .map(IdentityStoreEnvelope::decode)
        .transpose()?;
    let commit = prepare_commit(current.as_ref(), payload)?;
    let (_, envelope, anchor) = commit.into_parts();

    Ok(MobileIdentityCommit {
        previous_revision: current.as_ref().map(IdentityStoreEnvelope::revision),
        revision: envelope.revision(),
        envelope: envelope.encode(),
        anchor: anchor.encode().to_vec(),
    })
}

#[uniffi::export]
pub fn identity_store_validate(
    envelope: Vec<u8>,
    anchor: Vec<u8>,
) -> Result<u64, MobileIdentityStoreError> {
    let envelope = IdentityStoreEnvelope::decode(&envelope)?;
    let anchor = IdentityStoreAnchor::decode(&anchor)?;
    validate_loaded(&envelope, anchor)?;
    Ok(envelope.revision())
}

#[uniffi::export]
pub fn identity_store_payload(
    envelope: Vec<u8>,
    anchor: Vec<u8>,
) -> Result<Vec<u8>, MobileIdentityStoreError> {
    let envelope = IdentityStoreEnvelope::decode(&envelope)?;
    let anchor = IdentityStoreAnchor::decode(&anchor)?;
    validate_loaded(&envelope, anchor)?;
    Ok(envelope.payload().to_vec())
}

impl From<IdentityStoreError> for MobileIdentityStoreError {
    fn from(error: IdentityStoreError) -> Self {
        match error {
            IdentityStoreError::UnsupportedSchema => Self::UnsupportedSchema,
            IdentityStoreError::MalformedEnvelope => Self::MalformedEnvelope,
            IdentityStoreError::MalformedAnchor => Self::MalformedAnchor,
            IdentityStoreError::PayloadDigestMismatch => Self::PayloadDigestMismatch,
            IdentityStoreError::StaleOrMixedState => Self::StaleOrMixedState,
            IdentityStoreError::RevisionConflict => Self::RevisionConflict,
            IdentityStoreError::RevisionOverflow => Self::RevisionOverflow,
        }
    }
}
