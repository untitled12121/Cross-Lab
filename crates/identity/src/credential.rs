use crosslab_crypto::{
    CanonicalTranscript, Signature, SignatureAlgorithm, SigningKey, VerifyingKey,
};

use crate::{
    AuthorityDelegation, AuthorityRole, DeviceId, IdentityError, KeyId, OwnerId, OwnerRootRecord,
};

const DOMAIN: &str = "crosslab.device-credential.v1";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DeviceCredential {
    schema_version: u16,
    owner_id: OwnerId,
    device_id: DeviceId,
    device_key_id: KeyId,
    device_algorithm: SignatureAlgorithm,
    device_public_key: VerifyingKey,
    credential_epoch: u64,
    issuer_device_signing_key_id: KeyId,
    signature: Signature,
}

impl DeviceCredential {
    #[allow(clippy::too_many_arguments)]
    pub fn issue(
        owner_id: OwnerId,
        device_id: DeviceId,
        device_key: &SigningKey,
        credential_epoch: u64,
        root: &OwnerRootRecord,
        issuer: &AuthorityDelegation,
        issuer_key: &SigningKey,
    ) -> Result<Self, IdentityError> {
        Self::issue_for_public_key(
            owner_id,
            device_id,
            device_key.verifying_key(),
            credential_epoch,
            root,
            issuer,
            issuer_key,
        )
    }

    #[allow(clippy::too_many_arguments)]
    pub fn issue_for_public_key(
        owner_id: OwnerId,
        device_id: DeviceId,
        device_public_key: VerifyingKey,
        credential_epoch: u64,
        root: &OwnerRootRecord,
        issuer: &AuthorityDelegation,
        issuer_key: &SigningKey,
    ) -> Result<Self, IdentityError> {
        if issuer.role() != AuthorityRole::DeviceSigning {
            return Err(IdentityError::WrongIssuerRole);
        }
        if owner_id != root.owner_id() || owner_id != issuer.owner_id() {
            return Err(IdentityError::WrongOwner);
        }
        issuer.verify(root, 0)?;

        let issuer_public_key = issuer_key.verifying_key();
        let issuer_key_id = KeyId::derive(SignatureAlgorithm::Ed25519, &issuer_public_key);
        if issuer_key_id != issuer.delegated_key_id()
            || issuer_public_key != issuer.delegated_public_key()
        {
            return Err(IdentityError::UnknownIssuer);
        }

        let device_algorithm = SignatureAlgorithm::Ed25519;
        let device_key_id = KeyId::derive(device_algorithm, &device_public_key);
        let issuer_device_signing_key_id = issuer.delegated_key_id();
        let digest = credential_digest(
            1,
            owner_id,
            device_id,
            device_key_id,
            device_algorithm,
            device_public_key,
            credential_epoch,
            issuer_device_signing_key_id,
        );
        let signature = issuer_key.sign_digest(&digest);

        Ok(Self {
            schema_version: 1,
            owner_id,
            device_id,
            device_key_id,
            device_algorithm,
            device_public_key,
            credential_epoch,
            issuer_device_signing_key_id,
            signature,
        })
    }

    #[allow(clippy::too_many_arguments)]
    pub fn from_unverified_signed_parts(
        schema_version: u16,
        owner_id: OwnerId,
        device_id: DeviceId,
        device_key_id: KeyId,
        device_algorithm: SignatureAlgorithm,
        device_public_key: VerifyingKey,
        credential_epoch: u64,
        issuer_device_signing_key_id: KeyId,
        signature: Signature,
    ) -> Result<Self, IdentityError> {
        if schema_version != 1 {
            return Err(IdentityError::UnsupportedSchema);
        }
        if device_algorithm != SignatureAlgorithm::Ed25519 {
            return Err(IdentityError::UnsupportedAlgorithm);
        }
        if KeyId::derive(device_algorithm, &device_public_key) != device_key_id {
            return Err(IdentityError::MalformedPublicKey);
        }

        Ok(Self {
            schema_version,
            owner_id,
            device_id,
            device_key_id,
            device_algorithm,
            device_public_key,
            credential_epoch,
            issuer_device_signing_key_id,
            signature,
        })
    }

    pub fn verify(
        &self,
        root: &OwnerRootRecord,
        issuer: &AuthorityDelegation,
        minimum_credential_epoch: u64,
        minimum_delegation_epoch: u64,
    ) -> Result<(), IdentityError> {
        if self.schema_version != 1 {
            return Err(IdentityError::UnsupportedSchema);
        }
        if issuer.role() != AuthorityRole::DeviceSigning {
            return Err(IdentityError::WrongIssuerRole);
        }
        if self.owner_id != root.owner_id() || self.owner_id != issuer.owner_id() {
            return Err(IdentityError::WrongOwner);
        }
        issuer.verify(root, minimum_delegation_epoch)?;
        if self.issuer_device_signing_key_id != issuer.delegated_key_id() {
            return Err(IdentityError::UnknownIssuer);
        }
        if self.credential_epoch < minimum_credential_epoch {
            return Err(IdentityError::StaleCredentialEpoch);
        }
        if KeyId::derive(self.device_algorithm, &self.device_public_key) != self.device_key_id {
            return Err(IdentityError::MalformedPublicKey);
        }

        issuer
            .delegated_public_key()
            .verify_digest(&self.transcript_digest(), &self.signature)
            .map_err(|_| IdentityError::InvalidSignature)
    }

    pub fn rotate(
        &self,
        new_device_key: &SigningKey,
        root: &OwnerRootRecord,
        issuer: &AuthorityDelegation,
        issuer_key: &SigningKey,
    ) -> Result<Self, IdentityError> {
        self.verify(
            root,
            issuer,
            self.credential_epoch,
            issuer.delegation_epoch(),
        )?;
        let next_epoch = self
            .credential_epoch
            .checked_add(1)
            .ok_or(IdentityError::UnexpectedCredentialEpoch)?;
        Self::issue(
            self.owner_id,
            self.device_id,
            new_device_key,
            next_epoch,
            root,
            issuer,
            issuer_key,
        )
    }

    pub fn transcript_digest(&self) -> [u8; 32] {
        credential_digest(
            self.schema_version,
            self.owner_id,
            self.device_id,
            self.device_key_id,
            self.device_algorithm,
            self.device_public_key,
            self.credential_epoch,
            self.issuer_device_signing_key_id,
        )
    }

    pub const fn schema_version(&self) -> u16 {
        self.schema_version
    }

    pub const fn owner_id(&self) -> OwnerId {
        self.owner_id
    }

    pub const fn device_id(&self) -> DeviceId {
        self.device_id
    }

    pub const fn device_key_id(&self) -> KeyId {
        self.device_key_id
    }

    pub const fn device_algorithm(&self) -> SignatureAlgorithm {
        self.device_algorithm
    }

    pub const fn device_public_key(&self) -> VerifyingKey {
        self.device_public_key
    }

    pub const fn credential_epoch(&self) -> u64 {
        self.credential_epoch
    }

    pub const fn issuer_device_signing_key_id(&self) -> KeyId {
        self.issuer_device_signing_key_id
    }

    pub const fn signature(&self) -> Signature {
        self.signature
    }
}

#[allow(clippy::too_many_arguments)]
fn credential_digest(
    schema_version: u16,
    owner_id: OwnerId,
    device_id: DeviceId,
    device_key_id: KeyId,
    device_algorithm: SignatureAlgorithm,
    device_public_key: VerifyingKey,
    credential_epoch: u64,
    issuer_device_signing_key_id: KeyId,
) -> [u8; 32] {
    let mut transcript =
        CanonicalTranscript::new(DOMAIN).expect("static canonical domain is valid");
    transcript
        .push(1, schema_version.to_be_bytes())
        .expect("fixed field is valid");
    transcript
        .push(2, owner_id.to_bytes())
        .expect("fixed field is valid");
    transcript
        .push(3, device_id.to_bytes())
        .expect("fixed field is valid");
    transcript
        .push(4, device_key_id.to_bytes())
        .expect("fixed field is valid");
    transcript
        .push(5, device_algorithm.code().to_be_bytes())
        .expect("fixed field is valid");
    transcript
        .push(6, device_public_key.to_bytes())
        .expect("fixed field is valid");
    transcript
        .push(7, credential_epoch.to_be_bytes())
        .expect("fixed field is valid");
    transcript
        .push(8, issuer_device_signing_key_id.to_bytes())
        .expect("fixed field is valid");
    transcript.digest()
}
