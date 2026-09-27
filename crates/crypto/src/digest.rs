use crate::{Signature, SignatureAlgorithm};

const SIGNED_OBJECT_DOMAIN: &[u8] = b"crosslab.signed-object-digest.v1\0";

pub fn blake3_256(input: &[u8]) -> [u8; 32] {
    *blake3::hash(input).as_bytes()
}

pub struct Blake3Hasher(blake3::Hasher);

impl Blake3Hasher {
    pub fn new() -> Self {
        Self(blake3::Hasher::new())
    }

    pub fn update(&mut self, input: &[u8]) {
        self.0.update(input);
    }

    pub fn finalize(self) -> [u8; 32] {
        *self.0.finalize().as_bytes()
    }
}

impl Default for Blake3Hasher {
    fn default() -> Self {
        Self::new()
    }
}

pub fn signed_object_digest(
    transcript_digest: [u8; 32],
    algorithm: SignatureAlgorithm,
    signature: &Signature,
) -> [u8; 32] {
    let signature = signature.to_bytes();
    let mut input = Vec::with_capacity(SIGNED_OBJECT_DOMAIN.len() + 32 + 2 + signature.len());
    input.extend_from_slice(SIGNED_OBJECT_DOMAIN);
    input.extend_from_slice(&transcript_digest);
    input.extend_from_slice(&algorithm.code().to_be_bytes());
    input.extend_from_slice(&signature);
    blake3_256(&input)
}
