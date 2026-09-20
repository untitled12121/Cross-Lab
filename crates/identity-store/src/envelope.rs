use core::fmt;

pub const STORE_SCHEMA_VERSION: u16 = 1;

const ENVELOPE_DOMAIN: &[u8] = b"crosslab.identity-store.envelope.v1";
const ANCHOR_DOMAIN: &[u8] = b"crosslab.identity-store.anchor.v1";
const HEADER_LEN: usize = 2 + 8 + 8 + 32;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IdentityStoreEnvelope {
    schema_version: u16,
    revision: u64,
    payload_digest: [u8; 32],
    payload: Vec<u8>,
}

impl IdentityStoreEnvelope {
    pub fn new(revision: u64, payload: Vec<u8>) -> Self {
        let payload_digest = *blake3::hash(&payload).as_bytes();
        Self {
            schema_version: STORE_SCHEMA_VERSION,
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
        let payload_len = u64::try_from(self.payload.len()).expect("payload length fits u64");
        let mut encoded = Vec::with_capacity(HEADER_LEN + self.payload.len());
        encoded.extend_from_slice(&self.schema_version.to_be_bytes());
        encoded.extend_from_slice(&self.revision.to_be_bytes());
        encoded.extend_from_slice(&payload_len.to_be_bytes());
        encoded.extend_from_slice(&self.payload_digest);
        encoded.extend_from_slice(&self.payload);
        encoded
    }

    pub fn decode(encoded: &[u8]) -> Result<Self, IdentityStoreError> {
        if encoded.len() < HEADER_LEN {
            return Err(IdentityStoreError::MalformedEnvelope);
        }

        let schema_version = u16::from_be_bytes(copy_array(&encoded[0..2]));
        if schema_version != STORE_SCHEMA_VERSION {
            return Err(IdentityStoreError::UnsupportedSchema);
        }

        let revision = u64::from_be_bytes(copy_array(&encoded[2..10]));
        let payload_len = u64::from_be_bytes(copy_array(&encoded[10..18]));
        let payload_len =
            usize::try_from(payload_len).map_err(|_| IdentityStoreError::MalformedEnvelope)?;
        let expected_len = HEADER_LEN
            .checked_add(payload_len)
            .ok_or(IdentityStoreError::MalformedEnvelope)?;
        if encoded.len() != expected_len {
            return Err(IdentityStoreError::MalformedEnvelope);
        }

        let payload_digest = copy_array(&encoded[18..50]);
        let payload = encoded[HEADER_LEN..].to_vec();
        if *blake3::hash(&payload).as_bytes() != payload_digest {
            return Err(IdentityStoreError::PayloadDigestMismatch);
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
pub struct IdentityStoreAnchor {
    revision: u64,
    envelope_digest: [u8; 32],
}

impl IdentityStoreAnchor {
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

    pub fn decode(encoded: &[u8]) -> Result<Self, IdentityStoreError> {
        if encoded.len() != 40 {
            return Err(IdentityStoreError::MalformedAnchor);
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
pub struct PreparedIdentityCommit {
    previous_revision: Option<u64>,
    envelope: IdentityStoreEnvelope,
    anchor: IdentityStoreAnchor,
}

impl PreparedIdentityCommit {
    pub const fn previous_revision(&self) -> Option<u64> {
        self.previous_revision
    }

    pub const fn envelope(&self) -> &IdentityStoreEnvelope {
        &self.envelope
    }

    pub const fn anchor(&self) -> IdentityStoreAnchor {
        self.anchor
    }

    pub fn into_parts(self) -> (Option<u64>, IdentityStoreEnvelope, IdentityStoreAnchor) {
        (self.previous_revision, self.envelope, self.anchor)
    }
}

pub fn prepare_commit(
    current: Option<&IdentityStoreEnvelope>,
    payload: Vec<u8>,
) -> Result<PreparedIdentityCommit, IdentityStoreError> {
    let previous_revision = current.map(IdentityStoreEnvelope::revision);
    let next_revision = previous_revision
        .unwrap_or(0)
        .checked_add(1)
        .ok_or(IdentityStoreError::RevisionOverflow)?;
    let envelope = IdentityStoreEnvelope::new(next_revision, payload);
    let anchor = IdentityStoreAnchor::new(next_revision, envelope.envelope_digest());

    Ok(PreparedIdentityCommit {
        previous_revision,
        envelope,
        anchor,
    })
}

pub fn validate_loaded(
    envelope: &IdentityStoreEnvelope,
    anchor: IdentityStoreAnchor,
) -> Result<(), IdentityStoreError> {
    if envelope.schema_version() != STORE_SCHEMA_VERSION {
        return Err(IdentityStoreError::UnsupportedSchema);
    }
    if envelope.revision() != anchor.revision() {
        return Err(IdentityStoreError::StaleOrMixedState);
    }
    if envelope.envelope_digest() != anchor.envelope_digest() {
        return Err(IdentityStoreError::StaleOrMixedState);
    }

    Ok(())
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IdentityStoreError {
    UnsupportedSchema,
    MalformedEnvelope,
    MalformedAnchor,
    PayloadDigestMismatch,
    StaleOrMixedState,
    RevisionConflict,
    RevisionOverflow,
}

impl fmt::Display for IdentityStoreError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::UnsupportedSchema => "identity-store schema is unsupported",
            Self::MalformedEnvelope => "identity-store envelope is malformed",
            Self::MalformedAnchor => "identity-store currentness anchor is malformed",
            Self::PayloadDigestMismatch => "identity-store payload digest does not match",
            Self::StaleOrMixedState => "identity-store envelope and currentness anchor disagree",
            Self::RevisionConflict => "identity-store revision changed concurrently",
            Self::RevisionOverflow => "identity-store revision is exhausted",
        })
    }
}

impl std::error::Error for IdentityStoreError {}

fn copy_array<const N: usize>(bytes: &[u8]) -> [u8; N] {
    let mut output = [0_u8; N];
    output.copy_from_slice(bytes);
    output
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn envelope_round_trips_and_detects_payload_tamper() {
        let envelope = IdentityStoreEnvelope::new(7, b"owner-state".to_vec());
        let encoded = envelope.encode();
        assert_eq!(IdentityStoreEnvelope::decode(&encoded).unwrap(), envelope);

        let mut tampered = encoded;
        *tampered.last_mut().unwrap() ^= 1;
        assert_eq!(
            IdentityStoreEnvelope::decode(&tampered),
            Err(IdentityStoreError::PayloadDigestMismatch)
        );
    }

    #[test]
    fn commit_revision_and_anchor_are_monotonic() {
        let first = prepare_commit(None, b"one".to_vec()).unwrap();
        assert_eq!(first.envelope().revision(), 1);
        validate_loaded(first.envelope(), first.anchor()).unwrap();

        let second = prepare_commit(Some(first.envelope()), b"two".to_vec()).unwrap();
        assert_eq!(second.previous_revision(), Some(1));
        assert_eq!(second.envelope().revision(), 2);
        validate_loaded(second.envelope(), second.anchor()).unwrap();
    }

    #[test]
    fn rollback_or_mixed_anchor_is_rejected() {
        let first = prepare_commit(None, b"one".to_vec()).unwrap();
        let second = prepare_commit(Some(first.envelope()), b"two".to_vec()).unwrap();

        assert_eq!(
            validate_loaded(first.envelope(), second.anchor()),
            Err(IdentityStoreError::StaleOrMixedState)
        );
        assert_eq!(
            validate_loaded(second.envelope(), first.anchor()),
            Err(IdentityStoreError::StaleOrMixedState)
        );
    }
}
