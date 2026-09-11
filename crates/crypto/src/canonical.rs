use crate::blake3_256;

const MAGIC: &[u8; 17] = b"crosslab-canon-1\0";
const DIGEST_DOMAIN: &[u8] = b"crosslab.transcript-digest.v1\0";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CanonicalError {
    InvalidDomain,
    DomainTooLong,
    InvalidTag,
    NonIncreasingTag,
    TooManyFields,
    ValueTooLong,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct Field {
    tag: u16,
    value: Vec<u8>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CanonicalTranscript {
    domain: Vec<u8>,
    fields: Vec<Field>,
}

impl CanonicalTranscript {
    pub fn new(domain: &str) -> Result<Self, CanonicalError> {
        if !domain.is_ascii() {
            return Err(CanonicalError::InvalidDomain);
        }
        if domain.len() > u16::MAX as usize {
            return Err(CanonicalError::DomainTooLong);
        }

        Ok(Self {
            domain: domain.as_bytes().to_vec(),
            fields: Vec::new(),
        })
    }

    pub fn push<V>(&mut self, tag: u16, value: V) -> Result<(), CanonicalError>
    where
        V: AsRef<[u8]>,
    {
        if tag == 0 {
            return Err(CanonicalError::InvalidTag);
        }
        if self.fields.last().is_some_and(|field| tag <= field.tag) {
            return Err(CanonicalError::NonIncreasingTag);
        }
        if self.fields.len() == u16::MAX as usize {
            return Err(CanonicalError::TooManyFields);
        }

        let value = value.as_ref();
        if value.len() > u32::MAX as usize {
            return Err(CanonicalError::ValueTooLong);
        }

        self.fields.push(Field {
            tag,
            value: value.to_vec(),
        });
        Ok(())
    }

    pub fn encode(&self) -> Vec<u8> {
        let capacity = MAGIC.len()
            + 2
            + self.domain.len()
            + 2
            + self
                .fields
                .iter()
                .map(|field| 2 + 4 + field.value.len())
                .sum::<usize>();
        let mut output = Vec::with_capacity(capacity);

        output.extend_from_slice(MAGIC);
        output.extend_from_slice(&(self.domain.len() as u16).to_be_bytes());
        output.extend_from_slice(&self.domain);
        output.extend_from_slice(&(self.fields.len() as u16).to_be_bytes());

        for field in &self.fields {
            output.extend_from_slice(&field.tag.to_be_bytes());
            output.extend_from_slice(&(field.value.len() as u32).to_be_bytes());
            output.extend_from_slice(&field.value);
        }

        output
    }

    pub fn digest(&self) -> [u8; 32] {
        let encoded = self.encode();
        let mut input = Vec::with_capacity(DIGEST_DOMAIN.len() + encoded.len());
        input.extend_from_slice(DIGEST_DOMAIN);
        input.extend_from_slice(&encoded);
        blake3_256(&input)
    }
}
