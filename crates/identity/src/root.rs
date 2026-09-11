use crosslab_crypto::{SignatureAlgorithm, SigningKey, VerifyingKey};

use crate::{KeyId, OwnerId};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct OwnerRootRecord {
    schema_version: u16,
    owner_id: OwnerId,
    root_key_id: KeyId,
    root_algorithm: SignatureAlgorithm,
    root_public_key: VerifyingKey,
    root_epoch: u64,
}

impl OwnerRootRecord {
    pub fn new(owner_id: OwnerId, signing_key: &SigningKey, root_epoch: u64) -> Self {
        Self::from_public_key(owner_id, signing_key.verifying_key(), root_epoch)
    }

    pub fn from_public_key(
        owner_id: OwnerId,
        root_public_key: VerifyingKey,
        root_epoch: u64,
    ) -> Self {
        let root_algorithm = SignatureAlgorithm::Ed25519;
        let root_key_id = KeyId::derive(root_algorithm, &root_public_key);
        Self {
            schema_version: 1,
            owner_id,
            root_key_id,
            root_algorithm,
            root_public_key,
            root_epoch,
        }
    }

    pub const fn schema_version(&self) -> u16 {
        self.schema_version
    }

    pub const fn owner_id(&self) -> OwnerId {
        self.owner_id
    }

    pub const fn root_key_id(&self) -> KeyId {
        self.root_key_id
    }

    pub const fn root_algorithm(&self) -> SignatureAlgorithm {
        self.root_algorithm
    }

    pub const fn root_public_key(&self) -> VerifyingKey {
        self.root_public_key
    }

    pub const fn root_epoch(&self) -> u64 {
        self.root_epoch
    }
}
