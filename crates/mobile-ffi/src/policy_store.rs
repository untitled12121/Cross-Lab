use crosslab_identity::DeviceId;
use crosslab_policy::{CapabilityId, OperationName, PolicyError, PolicyState, RuleEffect};
use crosslab_policy_store::{
    PolicyStoreAnchor, PolicyStoreEnvelope, PolicyStoreError, prepare_commit, validate_loaded,
};

use crate::presence::MobilePermissionEffect;

#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct MobilePolicyCommit {
    pub previous_revision: Option<u64>,
    pub revision: u64,
    pub envelope: Vec<u8>,
    pub anchor: Vec<u8>,
}

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
    InvalidDeviceId,
    PolicyMutation,
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
            Self::InvalidDeviceId => "policy edit contains an invalid device identifier",
            Self::PolicyMutation => "policy edit could not advance local policy state",
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

#[uniffi::export]
pub fn policy_store_prepare_rule_effect(
    current_envelope: Option<Vec<u8>>,
    current_anchor: Option<Vec<u8>>,
    source_device_id: String,
    capability_id: String,
    operation: String,
    effect: MobilePermissionEffect,
) -> Result<Option<MobilePolicyCommit>, MobilePolicyStoreError> {
    let (current, mut policy) = decode_current(current_envelope, current_anchor)?;
    let source_device_id = parse_device_id(&source_device_id)?;
    let capability_id = CapabilityId::parse(&capability_id)
        .map_err(|_| MobilePolicyStoreError::InvalidIdentifier)?;
    let operation =
        OperationName::parse(&operation).map_err(|_| MobilePolicyStoreError::InvalidIdentifier)?;
    let effect = match effect {
        MobilePermissionEffect::Allow => RuleEffect::Allow,
        MobilePermissionEffect::Deny => RuleEffect::Deny,
        MobilePermissionEffect::Ask => RuleEffect::Ask,
    };

    if !policy.set_rule_effect(source_device_id, capability_id, operation, effect)? {
        return Ok(None);
    }

    let commit = prepare_commit(current.as_ref(), &policy)?;
    let (previous_revision, envelope, anchor) = commit.into_parts();
    Ok(Some(MobilePolicyCommit {
        previous_revision,
        revision: envelope.revision(),
        envelope: envelope.encode(),
        anchor: anchor.encode().to_vec(),
    }))
}

fn decode_current(
    envelope: Option<Vec<u8>>,
    anchor: Option<Vec<u8>>,
) -> Result<(Option<PolicyStoreEnvelope>, PolicyState), MobilePolicyStoreError> {
    match (envelope, anchor) {
        (None, None) => Ok((None, PolicyState::new())),
        (Some(envelope), Some(anchor)) => {
            let envelope = PolicyStoreEnvelope::decode(&envelope)?;
            let anchor = PolicyStoreAnchor::decode(&anchor)?;
            let policy = validate_loaded(&envelope, anchor)?;
            Ok((Some(envelope), policy))
        }
        _ => Err(MobilePolicyStoreError::StaleOrMixedState),
    }
}

fn parse_device_id(value: &str) -> Result<DeviceId, MobilePolicyStoreError> {
    if value.len() != 64 {
        return Err(MobilePolicyStoreError::InvalidDeviceId);
    }

    let encoded = value.as_bytes();
    let mut bytes = [0_u8; 32];
    for (index, byte) in bytes.iter_mut().enumerate() {
        let offset = index * 2;
        let high = hex_nibble(encoded[offset]).ok_or(MobilePolicyStoreError::InvalidDeviceId)?;
        let low = hex_nibble(encoded[offset + 1]).ok_or(MobilePolicyStoreError::InvalidDeviceId)?;
        *byte = (high << 4) | low;
    }
    Ok(DeviceId::from_bytes(bytes))
}

const fn hex_nibble(value: u8) -> Option<u8> {
    match value {
        b'0'..=b'9' => Some(value - b'0'),
        b'a'..=b'f' => Some(value - b'a' + 10),
        b'A'..=b'F' => Some(value - b'A' + 10),
        _ => None,
    }
}

impl From<PolicyError> for MobilePolicyStoreError {
    fn from(_: PolicyError) -> Self {
        Self::PolicyMutation
    }
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
    use crosslab_policy::{CapabilityId, OperationName, PolicyRule, RuleEffect, RuleId};
    use crosslab_policy_store::prepare_commit;

    use super::*;

    #[test]
    fn mobile_rule_effect_commit_advances_and_noops_exact_policy() {
        let source_device_id = "09".repeat(32);
        let first = policy_store_prepare_rule_effect(
            None,
            None,
            source_device_id.clone(),
            "clipboard.read".to_owned(),
            "get".to_owned(),
            MobilePermissionEffect::Allow,
        )
        .unwrap()
        .unwrap();
        assert_eq!(first.previous_revision, None);
        assert_eq!(first.revision, 1);
        assert_eq!(
            policy_store_validate(first.envelope.clone(), first.anchor.clone()).unwrap(),
            1
        );

        assert!(
            policy_store_prepare_rule_effect(
                Some(first.envelope.clone()),
                Some(first.anchor.clone()),
                source_device_id.clone(),
                "clipboard.read".to_owned(),
                "get".to_owned(),
                MobilePermissionEffect::Allow,
            )
            .unwrap()
            .is_none()
        );

        let second = policy_store_prepare_rule_effect(
            Some(first.envelope),
            Some(first.anchor),
            source_device_id,
            "clipboard.read".to_owned(),
            "get".to_owned(),
            MobilePermissionEffect::Deny,
        )
        .unwrap()
        .unwrap();
        assert_eq!(second.previous_revision, Some(1));
        assert_eq!(second.revision, 2);
    }

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
