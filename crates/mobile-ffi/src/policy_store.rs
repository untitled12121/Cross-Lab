use crosslab_policy::PolicyState;
use crosslab_policy_store::{
    PolicyStoreAnchor, PolicyStoreEnvelope, PolicyStoreError, validate_loaded,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, uniffi::Error)]
pub enum MobilePolicyStoreError {
    UnsupportedSchema,
    UnsupportedSnapshotSchema,
    MalformedEnvelope,
    MalformedAnchor,
    MalformedSnapshot,
    PayloadDigestMismatch,
    SnapshotTooLarge,
    TooManyRules,
    InvalidIdentifier,
    DuplicateRule,
    DuplicateRuleId,
    RevisionMismatch,
    StaleOrMixedState,
    RevisionConflict,
    NonMonotonicRevision,
}

impl core::fmt::Display for MobilePolicyStoreError {
    fn fmt(&self, formatter: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        formatter.write_str(match self {
            Self::UnsupportedSchema => "policy-store envelope schema is unsupported",
            Self::UnsupportedSnapshotSchema => "policy snapshot schema is unsupported",
            Self::MalformedEnvelope => "policy-store envelope is malformed",
            Self::MalformedAnchor => "policy-store currentness anchor is malformed",
            Self::MalformedSnapshot => "policy snapshot is malformed",
            Self::PayloadDigestMismatch => "policy-store payload digest does not match",
            Self::SnapshotTooLarge => "policy snapshot exceeds its size bound",
            Self::TooManyRules => "policy snapshot exceeds its rule bound",
            Self::InvalidIdentifier => "policy snapshot contains an invalid identifier",
            Self::DuplicateRule => "policy snapshot contains a duplicate exact rule",
            Self::DuplicateRuleId => "policy snapshot contains a duplicate rule identifier",
            Self::RevisionMismatch => "policy snapshot and envelope revisions disagree",
            Self::StaleOrMixedState => "policy-store currentness validation failed",
            Self::RevisionConflict => "policy-store revision changed concurrently",
            Self::NonMonotonicRevision => "policy-store revision did not advance",
        })
    }
}

impl std::error::Error for MobilePolicyStoreError {}

pub(crate) fn decode_policy_store(
    envelope: &[u8],
    anchor: &[u8],
) -> Result<PolicyState, MobilePolicyStoreError> {
    let envelope = PolicyStoreEnvelope::decode(envelope)?;
    let anchor = PolicyStoreAnchor::decode(anchor)?;
    validate_loaded(&envelope, anchor).map_err(Into::into)
}

#[uniffi::export]
pub fn policy_store_validate(
    envelope: Vec<u8>,
    anchor: Vec<u8>,
) -> Result<u64, MobilePolicyStoreError> {
    Ok(decode_policy_store(&envelope, &anchor)?.revision())
}

impl From<PolicyStoreError> for MobilePolicyStoreError {
    fn from(error: PolicyStoreError) -> Self {
        match error {
            PolicyStoreError::UnsupportedSchema => Self::UnsupportedSchema,
            PolicyStoreError::UnsupportedSnapshotSchema => Self::UnsupportedSnapshotSchema,
            PolicyStoreError::MalformedEnvelope => Self::MalformedEnvelope,
            PolicyStoreError::MalformedAnchor => Self::MalformedAnchor,
            PolicyStoreError::MalformedSnapshot => Self::MalformedSnapshot,
            PolicyStoreError::PayloadDigestMismatch => Self::PayloadDigestMismatch,
            PolicyStoreError::SnapshotTooLarge => Self::SnapshotTooLarge,
            PolicyStoreError::TooManyRules => Self::TooManyRules,
            PolicyStoreError::InvalidIdentifier => Self::InvalidIdentifier,
            PolicyStoreError::DuplicateRule => Self::DuplicateRule,
            PolicyStoreError::DuplicateRuleId => Self::DuplicateRuleId,
            PolicyStoreError::RevisionMismatch => Self::RevisionMismatch,
            PolicyStoreError::StaleOrMixedState => Self::StaleOrMixedState,
            PolicyStoreError::RevisionConflict => Self::RevisionConflict,
            PolicyStoreError::NonMonotonicRevision => Self::NonMonotonicRevision,
        }
    }
}

#[cfg(test)]
mod tests {
    use crosslab_identity::DeviceId;
    use crosslab_policy::{
        CapabilityId, OperationName, PolicyRule, RuleEffect, RuleId,
    };
    use crosslab_policy_store::prepare_commit;

    use super::*;

    #[test]
    fn mobile_validation_restores_exact_policy_revision() {
        let mut policy = PolicyState::new();
        policy
            .insert(PolicyRule::new(
                RuleId::from_bytes([7; 32]),
                DeviceId::from_bytes([9; 32]),
                CapabilityId::parse("clipboard.write").unwrap(),
                OperationName::parse("set").unwrap(),
                RuleEffect::Deny,
            ))
            .unwrap();
        let commit = prepare_commit(None, &policy).unwrap();

        assert_eq!(
            policy_store_validate(
                commit.envelope().encode(),
                commit.anchor().encode().to_vec(),
            )
            .unwrap(),
            policy.revision()
        );
    }
}
