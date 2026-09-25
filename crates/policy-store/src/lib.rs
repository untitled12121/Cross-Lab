//! Platform-neutral durable owner-policy snapshot/currentness contract.

use core::fmt;
use std::{collections::BTreeSet, sync::Mutex};

use crosslab_identity::DeviceId;
use crosslab_policy::{
    CapabilityId, Constraint, Obligation, OperationName, PolicyRule, PolicyState, RuleEffect,
    RuleId,
};

pub const POLICY_STORE_SCHEMA_VERSION: u16 = 1;
pub const POLICY_SNAPSHOT_SCHEMA_VERSION: u16 = 1;
pub const MAX_POLICY_RULES: usize = 1_024;
pub const MAX_POLICY_SNAPSHOT_BYTES: usize = 1024 * 1024;

const ENVELOPE_DOMAIN: &[u8] = b"crosslab.policy-store.envelope.v1";
const ANCHOR_DOMAIN: &[u8] = b"crosslab.policy-store.anchor.v1";
const ENVELOPE_HEADER_LEN: usize = 2 + 8 + 8 + 32;
const SNAPSHOT_HEADER_LEN: usize = 2 + 8 + 4;

#[derive(Clone, PartialEq, Eq)]
pub struct PolicyStoreEnvelope {
    schema_version: u16,
    revision: u64,
    payload_digest: [u8; 32],
    payload: Vec<u8>,
}

impl fmt::Debug for PolicyStoreEnvelope {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("PolicyStoreEnvelope")
            .field("schema_version", &self.schema_version)
            .field("revision", &self.revision)
            .field("payload_len", &self.payload.len())
            .finish()
    }
}

impl PolicyStoreEnvelope {
    fn new(revision: u64, payload: Vec<u8>) -> Self {
        let payload_digest = *blake3::hash(&payload).as_bytes();
        Self {
            schema_version: POLICY_STORE_SCHEMA_VERSION,
            revision,
            payload_digest,
            payload,
        }
    }

    pub const fn schema_version(&self) -> u16 {
        self.schema_version
    }

    pub const fn revision(&self) -> u64 {
        self.revision
    }

    pub const fn payload_digest(&self) -> [u8; 32] {
        self.payload_digest
    }

    pub fn payload(&self) -> &[u8] {
        &self.payload
    }

    pub fn encode(&self) -> Vec<u8> {
        let payload_len =
            u64::try_from(self.payload.len()).expect("policy payload length fits u64");
        let mut encoded = Vec::with_capacity(ENVELOPE_HEADER_LEN + self.payload.len());
        encoded.extend_from_slice(&self.schema_version.to_be_bytes());
        encoded.extend_from_slice(&self.revision.to_be_bytes());
        encoded.extend_from_slice(&payload_len.to_be_bytes());
        encoded.extend_from_slice(&self.payload_digest);
        encoded.extend_from_slice(&self.payload);
        encoded
    }

    pub fn decode(encoded: &[u8]) -> Result<Self, PolicyStoreError> {
        if encoded.len() < ENVELOPE_HEADER_LEN {
            return Err(PolicyStoreError::MalformedEnvelope);
        }

        let schema_version = u16::from_be_bytes(copy_array(&encoded[0..2]));
        if schema_version != POLICY_STORE_SCHEMA_VERSION {
            return Err(PolicyStoreError::UnsupportedSchema);
        }

        let revision = u64::from_be_bytes(copy_array(&encoded[2..10]));
        let payload_len = u64::from_be_bytes(copy_array(&encoded[10..18]));
        if payload_len > MAX_POLICY_SNAPSHOT_BYTES as u64 {
            return Err(PolicyStoreError::SnapshotTooLarge);
        }
        let payload_len =
            usize::try_from(payload_len).map_err(|_| PolicyStoreError::MalformedEnvelope)?;
        let expected_len = ENVELOPE_HEADER_LEN
            .checked_add(payload_len)
            .ok_or(PolicyStoreError::MalformedEnvelope)?;
        if encoded.len() != expected_len {
            return Err(PolicyStoreError::MalformedEnvelope);
        }

        let payload_digest = copy_array(&encoded[18..50]);
        let payload = encoded[ENVELOPE_HEADER_LEN..].to_vec();
        if *blake3::hash(&payload).as_bytes() != payload_digest {
            return Err(PolicyStoreError::PayloadDigestMismatch);
        }

        Ok(Self {
            schema_version,
            revision,
            payload_digest,
            payload,
        })
    }

    pub fn envelope_digest(&self) -> [u8; 32] {
        let mut hasher = blake3::Hasher::new();
        hasher.update(ENVELOPE_DOMAIN);
        hasher.update(&self.encode());
        *hasher.finalize().as_bytes()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PolicyStoreAnchor {
    revision: u64,
    envelope_digest: [u8; 32],
}

impl PolicyStoreAnchor {
    pub const fn new(revision: u64, envelope_digest: [u8; 32]) -> Self {
        Self {
            revision,
            envelope_digest,
        }
    }

    pub const fn revision(self) -> u64 {
        self.revision
    }

    pub const fn envelope_digest(self) -> [u8; 32] {
        self.envelope_digest
    }

    pub fn encode(self) -> [u8; 40] {
        let mut encoded = [0_u8; 40];
        encoded[..8].copy_from_slice(&self.revision.to_be_bytes());
        encoded[8..].copy_from_slice(&self.envelope_digest);
        encoded
    }

    pub fn decode(encoded: &[u8]) -> Result<Self, PolicyStoreError> {
        if encoded.len() != 40 {
            return Err(PolicyStoreError::MalformedAnchor);
        }
        Ok(Self {
            revision: u64::from_be_bytes(copy_array(&encoded[..8])),
            envelope_digest: copy_array(&encoded[8..]),
        })
    }

    pub fn protected_digest(self) -> [u8; 32] {
        let mut hasher = blake3::Hasher::new();
        hasher.update(ANCHOR_DOMAIN);
        hasher.update(&self.encode());
        *hasher.finalize().as_bytes()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PreparedPolicyCommit {
    previous_revision: Option<u64>,
    envelope: PolicyStoreEnvelope,
    anchor: PolicyStoreAnchor,
}

impl PreparedPolicyCommit {
    pub const fn previous_revision(&self) -> Option<u64> {
        self.previous_revision
    }

    pub const fn envelope(&self) -> &PolicyStoreEnvelope {
        &self.envelope
    }

    pub const fn anchor(&self) -> PolicyStoreAnchor {
        self.anchor
    }

    pub fn into_parts(self) -> (Option<u64>, PolicyStoreEnvelope, PolicyStoreAnchor) {
        (self.previous_revision, self.envelope, self.anchor)
    }
}

pub fn encode_policy_snapshot(policy: &PolicyState) -> Result<Vec<u8>, PolicyStoreError> {
    let rule_count = policy.rules().count();
    if rule_count > MAX_POLICY_RULES {
        return Err(PolicyStoreError::TooManyRules);
    }

    let mut encoded = Vec::with_capacity(SNAPSHOT_HEADER_LEN + rule_count * 96);
    encoded.extend_from_slice(&POLICY_SNAPSHOT_SCHEMA_VERSION.to_be_bytes());
    encoded.extend_from_slice(&policy.revision().to_be_bytes());
    encoded.extend_from_slice(
        &u32::try_from(rule_count)
            .map_err(|_| PolicyStoreError::TooManyRules)?
            .to_be_bytes(),
    );

    for rule in policy.rules() {
        encoded.extend_from_slice(&rule.rule_id().to_bytes());
        encoded.extend_from_slice(rule.source_device_id().as_bytes());
        push_string(&mut encoded, rule.capability_id().as_str())?;
        push_string(&mut encoded, rule.operation().as_str())?;
        encoded.push(match rule.effect() {
            RuleEffect::Allow => 1,
            RuleEffect::Deny => 2,
            RuleEffect::Ask => 3,
        });

        encoded.extend_from_slice(
            &u16::try_from(rule.constraints().len())
                .map_err(|_| PolicyStoreError::MalformedSnapshot)?
                .to_be_bytes(),
        );
        for constraint in rule.constraints() {
            encoded.push(match constraint {
                Constraint::LocalOnly => 1,
            });
        }

        encoded.extend_from_slice(
            &u16::try_from(rule.obligations().len())
                .map_err(|_| PolicyStoreError::MalformedSnapshot)?
                .to_be_bytes(),
        );
        for obligation in rule.obligations() {
            encoded.push(match obligation {
                Obligation::OwnerConfirmation => 1,
            });
        }
    }

    if encoded.len() > MAX_POLICY_SNAPSHOT_BYTES {
        return Err(PolicyStoreError::SnapshotTooLarge);
    }
    Ok(encoded)
}

pub fn decode_policy_snapshot(encoded: &[u8]) -> Result<PolicyState, PolicyStoreError> {
    if encoded.len() > MAX_POLICY_SNAPSHOT_BYTES {
        return Err(PolicyStoreError::SnapshotTooLarge);
    }

    let mut reader = Reader::new(encoded);
    if reader.u16()? != POLICY_SNAPSHOT_SCHEMA_VERSION {
        return Err(PolicyStoreError::UnsupportedSnapshotSchema);
    }
    let revision = reader.u64()?;
    let rule_count = reader.u32()? as usize;
    if rule_count > MAX_POLICY_RULES {
        return Err(PolicyStoreError::TooManyRules);
    }
    if revision < rule_count as u64 {
        return Err(PolicyStoreError::MalformedSnapshot);
    }

    let mut rules = Vec::with_capacity(rule_count);
    let mut rule_ids = BTreeSet::new();
    for _ in 0..rule_count {
        let rule_id = RuleId::from_bytes(reader.array()?);
        if !rule_ids.insert(rule_id) {
            return Err(PolicyStoreError::DuplicateRuleId);
        }
        let source_device_id = DeviceId::from_bytes(reader.array()?);
        let capability = CapabilityId::parse(reader.string()?)
            .map_err(|_| PolicyStoreError::InvalidIdentifier)?;
        let operation = OperationName::parse(reader.string()?)
            .map_err(|_| PolicyStoreError::InvalidIdentifier)?;

        let effect = match reader.u8()? {
            1 => RuleEffect::Allow,
            2 => RuleEffect::Deny,
            3 => RuleEffect::Ask,
            _ => return Err(PolicyStoreError::MalformedSnapshot),
        };

        let constraint_count = usize::from(reader.u16()?);
        if constraint_count > 1 {
            return Err(PolicyStoreError::MalformedSnapshot);
        }
        let mut constraints = Vec::with_capacity(constraint_count);
        for _ in 0..constraint_count {
            let value = match reader.u8()? {
                1 => Constraint::LocalOnly,
                _ => return Err(PolicyStoreError::MalformedSnapshot),
            };
            if constraints.contains(&value) {
                return Err(PolicyStoreError::MalformedSnapshot);
            }
            constraints.push(value);
        }

        let obligation_count = usize::from(reader.u16()?);
        if obligation_count > 1 {
            return Err(PolicyStoreError::MalformedSnapshot);
        }
        let mut obligations = Vec::with_capacity(obligation_count);
        for _ in 0..obligation_count {
            let value = match reader.u8()? {
                1 => Obligation::OwnerConfirmation,
                _ => return Err(PolicyStoreError::MalformedSnapshot),
            };
            if obligations.contains(&value) {
                return Err(PolicyStoreError::MalformedSnapshot);
            }
            obligations.push(value);
        }

        let mut rule = PolicyRule::new(rule_id, source_device_id, capability, operation, effect);
        for constraint in constraints {
            rule = rule.with_constraint(constraint);
        }
        for obligation in obligations {
            rule = rule.with_obligation(obligation);
        }
        rules.push(rule);
    }
    reader.finish()?;

    PolicyState::from_snapshot(revision, rules).map_err(|_| PolicyStoreError::DuplicateRule)
}

pub fn prepare_commit(
    current: Option<&PolicyStoreEnvelope>,
    policy: &PolicyState,
) -> Result<PreparedPolicyCommit, PolicyStoreError> {
    let previous_revision = if let Some(current) = current {
        let decoded = decode_policy_snapshot(current.payload())?;
        if decoded.revision() != current.revision() {
            return Err(PolicyStoreError::RevisionMismatch);
        }
        Some(current.revision())
    } else {
        None
    };
    let current_revision = previous_revision.unwrap_or(0);
    if policy.revision() <= current_revision {
        return Err(PolicyStoreError::NonMonotonicRevision);
    }

    let payload = encode_policy_snapshot(policy)?;
    let envelope = PolicyStoreEnvelope::new(policy.revision(), payload);
    let anchor = PolicyStoreAnchor::new(policy.revision(), envelope.envelope_digest());
    Ok(PreparedPolicyCommit {
        previous_revision,
        envelope,
        anchor,
    })
}

pub fn validate_loaded(
    envelope: &PolicyStoreEnvelope,
    anchor: PolicyStoreAnchor,
) -> Result<PolicyState, PolicyStoreError> {
    if envelope.schema_version() != POLICY_STORE_SCHEMA_VERSION {
        return Err(PolicyStoreError::UnsupportedSchema);
    }
    if envelope.revision() != anchor.revision()
        || envelope.envelope_digest() != anchor.envelope_digest()
    {
        return Err(PolicyStoreError::StaleOrMixedState);
    }

    let policy = decode_policy_snapshot(envelope.payload())?;
    if policy.revision() != envelope.revision() {
        return Err(PolicyStoreError::RevisionMismatch);
    }
    Ok(policy)
}

#[derive(Debug, Default)]
pub struct MemoryPolicyStore {
    state: Mutex<MemoryState>,
}

#[derive(Debug, Default)]
struct MemoryState {
    envelope: Option<PolicyStoreEnvelope>,
    anchor: Option<PolicyStoreAnchor>,
}

impl MemoryPolicyStore {
    pub fn load(&self) -> Result<PolicyState, PolicyStoreError> {
        let state = self
            .state
            .lock()
            .expect("memory policy-store lock poisoned");
        match (&state.envelope, state.anchor) {
            (None, None) => Ok(PolicyState::new()),
            (Some(envelope), Some(anchor)) => validate_loaded(envelope, anchor),
            _ => Err(PolicyStoreError::StaleOrMixedState),
        }
    }

    pub fn compare_and_swap(
        &self,
        expected_revision: u64,
        policy: &PolicyState,
    ) -> Result<PolicyStoreEnvelope, PolicyStoreError> {
        let mut state = self
            .state
            .lock()
            .expect("memory policy-store lock poisoned");
        let current_revision = match (&state.envelope, state.anchor) {
            (None, None) => 0,
            (Some(envelope), Some(anchor)) => validate_loaded(envelope, anchor)?.revision(),
            _ => return Err(PolicyStoreError::StaleOrMixedState),
        };
        if current_revision != expected_revision {
            return Err(PolicyStoreError::RevisionConflict);
        }

        let commit = prepare_commit(state.envelope.as_ref(), policy)?;
        state.envelope = Some(commit.envelope().clone());
        state.anchor = Some(commit.anchor());
        Ok(commit.envelope().clone())
    }

    #[cfg(test)]
    fn replace_anchor_for_test(&self, anchor: PolicyStoreAnchor) {
        self.state
            .lock()
            .expect("memory policy-store lock poisoned")
            .anchor = Some(anchor);
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PolicyStoreError {
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

impl fmt::Display for PolicyStoreError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
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
            Self::StaleOrMixedState => "policy-store envelope and currentness anchor disagree",
            Self::RevisionConflict => "policy-store revision changed concurrently",
            Self::NonMonotonicRevision => "policy-store commit does not advance policy revision",
        })
    }
}

impl std::error::Error for PolicyStoreError {}

fn push_string(encoded: &mut Vec<u8>, value: &str) -> Result<(), PolicyStoreError> {
    let len = u16::try_from(value.len()).map_err(|_| PolicyStoreError::MalformedSnapshot)?;
    encoded.extend_from_slice(&len.to_be_bytes());
    encoded.extend_from_slice(value.as_bytes());
    Ok(())
}

fn copy_array<const N: usize>(bytes: &[u8]) -> [u8; N] {
    let mut output = [0_u8; N];
    output.copy_from_slice(bytes);
    output
}

struct Reader<'a> {
    encoded: &'a [u8],
    cursor: usize,
}

impl<'a> Reader<'a> {
    const fn new(encoded: &'a [u8]) -> Self {
        Self { encoded, cursor: 0 }
    }

    fn take(&mut self, len: usize) -> Result<&'a [u8], PolicyStoreError> {
        let end = self
            .cursor
            .checked_add(len)
            .ok_or(PolicyStoreError::MalformedSnapshot)?;
        let value = self
            .encoded
            .get(self.cursor..end)
            .ok_or(PolicyStoreError::MalformedSnapshot)?;
        self.cursor = end;
        Ok(value)
    }

    fn array<const N: usize>(&mut self) -> Result<[u8; N], PolicyStoreError> {
        Ok(copy_array(self.take(N)?))
    }

    fn u8(&mut self) -> Result<u8, PolicyStoreError> {
        Ok(self.take(1)?[0])
    }

    fn u16(&mut self) -> Result<u16, PolicyStoreError> {
        Ok(u16::from_be_bytes(self.array()?))
    }

    fn u32(&mut self) -> Result<u32, PolicyStoreError> {
        Ok(u32::from_be_bytes(self.array()?))
    }

    fn u64(&mut self) -> Result<u64, PolicyStoreError> {
        Ok(u64::from_be_bytes(self.array()?))
    }

    fn string(&mut self) -> Result<&'a str, PolicyStoreError> {
        let len = usize::from(self.u16()?);
        core::str::from_utf8(self.take(len)?).map_err(|_| PolicyStoreError::MalformedSnapshot)
    }

    fn finish(self) -> Result<(), PolicyStoreError> {
        if self.cursor == self.encoded.len() {
            Ok(())
        } else {
            Err(PolicyStoreError::MalformedSnapshot)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn device(value: u8) -> DeviceId {
        let mut bytes = [0_u8; 32];
        bytes[31] = value;
        DeviceId::from_bytes(bytes)
    }

    fn rule(device_value: u8, effect: RuleEffect) -> PolicyRule {
        PolicyRule::new(
            RuleId::from_bytes([device_value; 32]),
            device(device_value),
            CapabilityId::parse("clipboard.read").unwrap(),
            OperationName::parse("get").unwrap(),
            effect,
        )
        .with_constraint(Constraint::LocalOnly)
        .with_obligation(Obligation::OwnerConfirmation)
    }

    fn policy(effect: RuleEffect) -> PolicyState {
        let mut policy = PolicyState::new();
        policy.insert(rule(1, effect)).unwrap();
        policy
    }

    #[test]
    fn snapshot_round_trips_exact_policy_revision() {
        let mut expected = policy(RuleEffect::Deny);
        expected
            .set_rule_effect(
                device(1),
                CapabilityId::parse("clipboard.read").unwrap(),
                OperationName::parse("get").unwrap(),
                RuleEffect::Allow,
            )
            .unwrap();
        assert_eq!(expected.revision(), 2);

        let encoded = encode_policy_snapshot(&expected).unwrap();
        let decoded = decode_policy_snapshot(&encoded).unwrap();
        assert_eq!(decoded, expected);
        assert_eq!(decoded.revision(), 2);
    }

    #[test]
    fn malformed_duplicate_rule_is_rejected() {
        let policy = policy(RuleEffect::Allow);
        let mut encoded = encode_policy_snapshot(&policy).unwrap();
        let rule = encoded[SNAPSHOT_HEADER_LEN..].to_vec();
        encoded[10..14].copy_from_slice(&2_u32.to_be_bytes());
        encoded.extend_from_slice(&rule);

        assert_eq!(
            decode_policy_snapshot(&encoded),
            Err(PolicyStoreError::DuplicateRuleId)
        );
    }

    #[test]
    fn envelope_and_anchor_detect_mixed_state() {
        let first_policy = policy(RuleEffect::Deny);
        let first = prepare_commit(None, &first_policy).unwrap();

        let mut second_policy = first_policy.clone();
        second_policy
            .set_rule_effect(
                device(1),
                CapabilityId::parse("clipboard.read").unwrap(),
                OperationName::parse("get").unwrap(),
                RuleEffect::Allow,
            )
            .unwrap();
        let second = prepare_commit(Some(first.envelope()), &second_policy).unwrap();

        assert_eq!(
            validate_loaded(second.envelope(), first.anchor()),
            Err(PolicyStoreError::StaleOrMixedState)
        );
        assert_eq!(
            validate_loaded(first.envelope(), second.anchor()),
            Err(PolicyStoreError::StaleOrMixedState)
        );
    }

    #[test]
    fn memory_store_rejects_stale_writer_and_rollback() {
        let store = MemoryPolicyStore::default();
        assert_eq!(store.load().unwrap().revision(), 0);

        let first_policy = policy(RuleEffect::Deny);
        let first = store.compare_and_swap(0, &first_policy).unwrap();
        let first_anchor = PolicyStoreAnchor::new(first.revision(), first.envelope_digest());

        let mut second_policy = first_policy.clone();
        second_policy
            .set_rule_effect(
                device(1),
                CapabilityId::parse("clipboard.read").unwrap(),
                OperationName::parse("get").unwrap(),
                RuleEffect::Allow,
            )
            .unwrap();

        assert_eq!(
            store.compare_and_swap(0, &second_policy),
            Err(PolicyStoreError::RevisionConflict)
        );
        store
            .compare_and_swap(first_policy.revision(), &second_policy)
            .unwrap();
        store.replace_anchor_for_test(first_anchor);
        assert_eq!(store.load(), Err(PolicyStoreError::StaleOrMixedState));
    }

    #[test]
    fn envelope_debug_does_not_expose_policy_payload() {
        let policy = policy(RuleEffect::Allow);
        let commit = prepare_commit(None, &policy).unwrap();
        let debug = format!("{:?}", commit.envelope());
        assert!(!debug.contains("clipboard"));
        assert!(!debug.contains("get"));
    }
}
