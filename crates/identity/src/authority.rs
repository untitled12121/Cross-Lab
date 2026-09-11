use crosslab_crypto::{
    CanonicalTranscript, Signature, SignatureAlgorithm, SigningKey, VerifyingKey,
};

use crate::{IdentityError, KeyId, OwnerId, OwnerRootRecord};

const DOMAIN: &str = "crosslab.authority-delegation.v1";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u16)]
pub enum AuthorityRole {
    OwnerRoot = 1,
    DeviceSigning = 2,
    Administrative = 3,
    Recovery = 4,
}

impl AuthorityRole {
    pub const fn code(self) -> u16 {
        self as u16
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AuthorityDelegation {
    schema_version: u16,
    owner_id: OwnerId,
    role: AuthorityRole,
    delegated_key_id: KeyId,
    delegated_algorithm: SignatureAlgorithm,
    delegated_public_key: VerifyingKey,
    delegation_epoch: u64,
    issuer_root_key_id: KeyId,
    signature: Signature,
}

impl AuthorityDelegation {
    pub fn issue(
        owner_id: OwnerId,
        role: AuthorityRole,
        delegated_key: &SigningKey,
        delegation_epoch: u64,
        issuer_root: &SigningKey,
    ) -> Self {
        let delegated_algorithm = SignatureAlgorithm::Ed25519;
        let delegated_public_key = delegated_key.verifying_key();
        let delegated_key_id = KeyId::derive(delegated_algorithm, &delegated_public_key);
        let issuer_root_key_id = KeyId::derive(SignatureAlgorithm::Ed25519, &issuer_root.verifying_key());
        let digest = delegation_digest(
            1,
            owner_id,
            role,
            delegated_key_id,
            delegated_algorithm,
            delegated_public_key,
            delegation_epoch,
            issuer_root_key_id,
        );
        let signature = issuer_root.sign_digest(&digest);

        Self {
            schema_version: 1,
            owner_id,
            role,
            delegated_key_id,
            delegated_algorithm,
            delegated_public_key,
            delegation_epoch,
            issuer_root_key_id,
            signature,
        }
    }

    pub fn verify(
        &self,
        root: &OwnerRootRecord,
        minimum_epoch: u64,
    ) -> Result<(), IdentityError> {
        if self.schema_version != 1 {
            return Err(IdentityError::UnsupportedSchema);
        }
        if self.owner_id != root.owner_id() {
            return Err(IdentityError::WrongOwner);
        }
        if self.issuer_root_key_id != root.root_key_id() {
            return Err(IdentityError::UnknownIssuer);
        }
        if self.delegation_epoch < minimum_epoch {
            return Err(IdentityError::InvalidAuthorityEpoch);
        }
        if KeyId::derive(self.delegated_algorithm, &self.delegated_public_key)
            != self.delegated_key_id
        {
            return Err(IdentityError::MalformedPublicKey);
        }

        root.root_public_key()
            .verify_digest(&self.transcript_digest(), &self.signature)
            .map_err(|_| IdentityError::InvalidSignature)
    }

    pub fn transcript_digest(&self) -> [u8; 32] {
        delegation_digest(
            self.schema_version,
            self.owner_id,
            self.role,
            self.delegated_key_id,
            self.delegated_algorithm,
            self.delegated_public_key,
            self.delegation_epoch,
            self.issuer_root_key_id,
        )
    }

    pub const fn owner_id(&self) -> OwnerId {
        self.owner_id
    }

    pub const fn role(&self) -> AuthorityRole {
        self.role
    }

    pub const fn delegated_key_id(&self) -> KeyId {
        self.delegated_key_id
    }

    pub const fn delegated_algorithm(&self) -> SignatureAlgorithm {
        self.delegated_algorithm
    }

    pub const fn delegated_public_key(&self) -> VerifyingKey {
        self.delegated_public_key
    }

    pub const fn delegation_epoch(&self) -> u64 {
        self.delegation_epoch
    }

    pub const fn issuer_root_key_id(&self) -> KeyId {
        self.issuer_root_key_id
    }

    pub const fn signature(&self) -> Signature {
        self.signature
    }
}

#[allow(clippy::too_many_arguments)]
fn delegation_digest(
    schema_version: u16,
    owner_id: OwnerId,
    role: AuthorityRole,
    delegated_key_id: KeyId,
    delegated_algorithm: SignatureAlgorithm,
    delegated_public_key: VerifyingKey,
    delegation_epoch: u64,
    issuer_root_key_id: KeyId,
) -> [u8; 32] {
    let mut transcript = CanonicalTranscript::new(DOMAIN).expect("static canonical domain is valid");
    transcript.push(1, schema_version.to_be_bytes()).expect("fixed field is valid");
    transcript.push(2, owner_id.to_bytes()).expect("fixed field is valid");
    transcript.push(3, role.code().to_be_bytes()).expect("fixed field is valid");
    transcript.push(4, delegated_key_id.to_bytes()).expect("fixed field is valid");
    transcript
        .push(5, delegated_algorithm.code().to_be_bytes())
        .expect("fixed field is valid");
    transcript
        .push(6, delegated_public_key.to_bytes())
        .expect("fixed field is valid");
    transcript
        .push(7, delegation_epoch.to_be_bytes())
        .expect("fixed field is valid");
    transcript
        .push(8, issuer_root_key_id.to_bytes())
        .expect("fixed field is valid");
    transcript.digest()
}
