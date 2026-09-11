//! Shared implementation-neutral cryptographic primitives for Cross-Lab.

mod canonical;
mod digest;
mod hmac;
mod keys;
mod random;

pub use canonical::{CanonicalError, CanonicalTranscript};
pub use digest::{blake3_256, signed_object_digest};
pub use hmac::{HmacVerificationError, hmac_sha256, verify_hmac_sha256};
pub use keys::{Signature, SignatureAlgorithm, SigningKey, VerificationError, VerifyingKey};
pub use random::{RandomError, random_bytes};
