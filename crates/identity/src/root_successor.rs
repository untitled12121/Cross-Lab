use crosslab_crypto::{
    CanonicalTranscript, Signature, SignatureAlgorithm, SigningKey, VerifyingKey,
};

use crate::{IdentityError, KeyId, OwnerId, OwnerRootRecord};

const DOMAIN: &str = "crosslab.root-successor.v1";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RootSuccessor {
    schema_version: u16,
    owner_id: OwnerId,
    current_root_key_id: KeyId,
    current_root_epoch: u64,
    next_root_key_id: KeyId,
    next_root_algorithm: SignatureAlgorithm,
    next_root_public_key: VerifyingKey,
    next_root_epoch: u64,
    current_signature: Signature,
    next_signature: Signature,
}

impl RootSuccessor {
    pub fn issue(
        current: &OwnerRootRecord,
        current_signing_key: &SigningKey,
        next_signing_key: &SigningKey,
    ) -> Result<Self, IdentityError> {
        if current_signing_key.verifying_key() != current.root_public_key() {
            return Err(IdentityError::InvalidRootSuccessor);
        }

        let next_root_epoch = current
            .root_epoch()
            .checked_add(1)
            .ok_or(IdentityError::InvalidRootSuccessor)?;
        let next_root_algorithm = SignatureAlgorithm::Ed25519;
        let next_root_public_key = next_signing_key.verifying_key();
        let next_root_key_id = KeyId::derive(next_root_algorithm, &next_root_public_key);
        let digest = successor_digest(
            1,
            current.owner_id(),
            current.root_key_id(),
            current.root_epoch(),
            next_root_key_id,
            next_root_algorithm,
            next_root_public_key,
            next_root_epoch,
        );

        Ok(Self {
            schema_version: 1,
            owner_id: current.owner_id(),
            current_root_key_id: current.root_key_id(),
            current_root_epoch: current.root_epoch(),
            next_root_key_id,
            next_root_algorithm,
            next_root_public_key,
            next_root_epoch,
            current_signature: current_signing_key.sign_digest(&digest),
            next_signature: next_signing_key.sign_digest(&digest),
        })
    }

    pub fn verify(&self, current: &OwnerRootRecord) -> Result<OwnerRootRecord, IdentityError> {
        if self.schema_version != 1 {
            return Err(IdentityError::UnsupportedSchema);
        }
        if self.owner_id != current.owner_id() {
            return Err(IdentityError::WrongOwner);
        }
        if self.current_root_key_id != current.root_key_id()
            || self.current_root_epoch != current.root_epoch()
        {
            return Err(IdentityError::InvalidRootSuccessor);
        }
        if self.next_root_epoch != current.root_epoch().checked_add(1).ok_or(IdentityError::InvalidRootSuccessor)? {
            return Err(IdentityError::InvalidRootSuccessor);
        }
        if KeyId::derive(self.next_root_algorithm, &self.next_root_public_key) != self.next_root_key_id {
            return Err(IdentityError::MalformedPublicKey);
        }

        let digest = self.transcript_digest();
        current
            .root_public_key()
            .verify_digest(&digest, &self.current_signature)
            .map_err(|_| IdentityError::InvalidRootSuccessor)?;
        self.next_root_public_key
            .verify_digest(&digest, &self.next_signature)
            .map_err(|_| IdentityError::InvalidRootSuccessor)?;

        Ok(OwnerRootRecord::from_public_key(
            self.owner_id,
            self.next_root_public_key,
            self.next_root_epoch,
        ))
    }

    pub fn transcript_digest(&self) -> [u8; 32] {
        successor_digest(
            self.schema_version,
            self.owner_id,
            self.current_root_key_id,
            self.current_root_epoch,
            self.next_root_key_id,
            self.next_root_algorithm,
            self.next_root_public_key,
            self.next_root_epoch,
        )
    }
}

#[allow(clippy::too_many_arguments)]
fn successor_digest(
    schema_version: u16,
    owner_id: OwnerId,
    current_root_key_id: KeyId,
    current_root_epoch: u64,
    next_root_key_id: KeyId,
    next_root_algorithm: SignatureAlgorithm,
    next_root_public_key: VerifyingKey,
    next_root_epoch: u64,
) -> [u8; 32] {
    let mut transcript = CanonicalTranscript::new(DOMAIN).expect("static canonical domain is valid");
    transcript.push(1, schema_version.to_be_bytes()).expect("fixed field is valid");
    transcript.push(2, owner_id.to_bytes()).expect("fixed field is valid");
    transcript
        .push(3, current_root_key_id.to_bytes())
        .expect("fixed field is valid");
    transcript
        .push(4, current_root_epoch.to_be_bytes())
        .expect("fixed field is valid");
    transcript
        .push(5, next_root_key_id.to_bytes())
        .expect("fixed field is valid");
    transcript
        .push(6, next_root_algorithm.code().to_be_bytes())
        .expect("fixed field is valid");
    transcript
        .push(7, next_root_public_key.to_bytes())
        .expect("fixed field is valid");
    transcript
        .push(8, next_root_epoch.to_be_bytes())
        .expect("fixed field is valid");
    transcript.digest()
}
