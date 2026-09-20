//! Platform-neutral identity-store commit/currentness contract.

mod envelope;
mod memory;

pub use envelope::{
    IdentityStoreAnchor, IdentityStoreEnvelope, IdentityStoreError, PreparedIdentityCommit,
    STORE_SCHEMA_VERSION, prepare_commit, validate_loaded,
};
pub use memory::MemoryIdentityStore;
