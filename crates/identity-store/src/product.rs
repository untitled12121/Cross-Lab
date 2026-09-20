use core::fmt;

use crosslab_crypto::{Signature, SignatureAlgorithm, SigningProvider, VerifyingKey};
use crosslab_identity::{
    AuthorityDelegation, AuthorityRole, DeviceCredential, DeviceId, IdentityError,
    OwnerAuthorityState, OwnerId, OwnerRootRecord,
};

const MAGIC: &[u8; 8] = b"CLPIDV1\0";
const SCHEMA_VERSION: u16 = 1;
const ENCODED_LEN: usize = MAGIC.len() + 2 + 32 + 32 + 8 + 32 + 8 + 32 + 64 + 8 + 32 + 64;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ProductIdentityState {
    owner_id: OwnerId,
    local_device_id: DeviceId,
    root: OwnerRootRecord,
    device_signing: AuthorityDelegation,
    local_credential: DeviceCredential,
}

impl ProductIdentityState {
    pub fn bootstrap(
        root_signer: &dyn SigningProvider,
        device_signing_signer: &dyn SigningProvider,
        local_device_signer: &dyn SigningProvider,
    ) -> Result<Self, ProductIdentityError> {
        let owner_id = OwnerId::generate().map_err(|_| ProductIdentityError::Random)?;
        let local_device_id = DeviceId::generate().map_err(|_| ProductIdentityError::Random)?;

        let root = OwnerRootRecord::new_with_provider(owner_id, root_signer, 0);
        let device_signing = AuthorityDelegation::issue_with_providers(
            owner_id,
            AuthorityRole::DeviceSigning,
            device_signing_signer,
            0,
            root_signer,
        )?;
        let mut authority = OwnerAuthorityState::new(root);
        authority.accept_delegation(device_signing)?;
        let local_credential = DeviceCredential::issue_with_providers(
            owner_id,
            local_device_id,
            local_device_signer,
            0,
            &authority,
            device_signing_signer,
        )?;

        Ok(Self {
            owner_id,
            local_device_id,
            root,
            device_signing,
            local_credential,
        })
    }

    pub const fn owner_id(&self) -> OwnerId {
        self.owner_id
    }

    pub const fn local_device_id(&self) -> DeviceId {
        self.local_device_id
    }

    pub const fn root(&self) -> OwnerRootRecord {
        self.root
    }

    pub const fn device_signing(&self) -> AuthorityDelegation {
        self.device_signing
    }

    pub const fn local_credential(&self) -> DeviceCredential {
        self.local_credential
    }

    pub fn authority_state(&self) -> Result<OwnerAuthorityState, ProductIdentityError> {
        let mut authority = OwnerAuthorityState::new(self.root);
        authority.accept_delegation(self.device_signing)?;
        Ok(authority)
    }

    pub fn validate_providers(
        &self,
        root_signer: &dyn SigningProvider,
        device_signing_signer: &dyn SigningProvider,
        local_device_signer: &dyn SigningProvider,
    ) -> Result<(), ProductIdentityError> {
        if root_signer.verifying_key() != self.root.root_public_key()
            || device_signing_signer.verifying_key() != self.device_signing.delegated_public_key()
            || local_device_signer.verifying_key() != self.local_credential.device_public_key()
        {
            return Err(ProductIdentityError::ProviderMismatch);
        }

        let authority = self.authority_state()?;
        self.local_credential
            .verify(&authority, self.local_credential.credential_epoch())?;
        Ok(())
    }

    pub fn encode(&self) -> Vec<u8> {
        let mut output = Vec::with_capacity(ENCODED_LEN);
        output.extend_from_slice(MAGIC);
        output.extend_from_slice(&SCHEMA_VERSION.to_be_bytes());
        output.extend_from_slice(self.owner_id.as_bytes());
        output.extend_from_slice(self.local_device_id.as_bytes());
        output.extend_from_slice(&self.root.root_epoch().to_be_bytes());
        output.extend_from_slice(self.root.root_public_key().as_bytes());
        output.extend_from_slice(&self.device_signing.delegation_epoch().to_be_bytes());
        output.extend_from_slice(self.device_signing.delegated_public_key().as_bytes());
        output.extend_from_slice(&self.device_signing.signature().to_bytes());
        output.extend_from_slice(&self.local_credential.credential_epoch().to_be_bytes());
        output.extend_from_slice(self.local_credential.device_public_key().as_bytes());
        output.extend_from_slice(&self.local_credential.signature().to_bytes());
        output
    }

    pub fn decode(encoded: &[u8]) -> Result<Self, ProductIdentityError> {
        if encoded.len() != ENCODED_LEN || &encoded[..MAGIC.len()] != MAGIC {
            return Err(ProductIdentityError::Malformed);
        }

        let mut offset = MAGIC.len();
        let schema = read_u16(encoded, &mut offset)?;
        if schema != SCHEMA_VERSION {
            return Err(ProductIdentityError::UnsupportedSchema);
        }

        let owner_id = OwnerId::from_bytes(read_array(encoded, &mut offset)?);
        let local_device_id = DeviceId::from_bytes(read_array(encoded, &mut offset)?);
        let root_epoch = read_u64(encoded, &mut offset)?;
        let root_key = read_verifying_key(encoded, &mut offset)?;
        let root = OwnerRootRecord::from_public_key(owner_id, root_key, root_epoch);

        let delegation_epoch = read_u64(encoded, &mut offset)?;
        let delegated_key = read_verifying_key(encoded, &mut offset)?;
        let delegation_signature = Signature::from_bytes(read_array(encoded, &mut offset)?);
        let device_signing = AuthorityDelegation::from_unverified_signed_parts(
            SCHEMA_VERSION,
            owner_id,
            AuthorityRole::DeviceSigning,
            SignatureAlgorithm::Ed25519,
            delegated_key,
            delegation_epoch,
            root.root_key_id(),
            delegation_signature,
        )?;
        device_signing.verify(&root, delegation_epoch)?;

        let credential_epoch = read_u64(encoded, &mut offset)?;
        let device_key = read_verifying_key(encoded, &mut offset)?;
        let credential_signature = Signature::from_bytes(read_array(encoded, &mut offset)?);
        let local_credential = DeviceCredential::from_unverified_signed_parts(
            SCHEMA_VERSION,
            owner_id,
            local_device_id,
            crosslab_identity::KeyId::derive(SignatureAlgorithm::Ed25519, &device_key),
            SignatureAlgorithm::Ed25519,
            device_key,
            credential_epoch,
            device_signing.delegated_key_id(),
            credential_signature,
        )?;

        let state = Self {
            owner_id,
            local_device_id,
            root,
            device_signing,
            local_credential,
        };
        let authority = state.authority_state()?;
        state
            .local_credential
            .verify(&authority, credential_epoch)?;
        Ok(state)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProductIdentityError {
    Random,
    Malformed,
    UnsupportedSchema,
    ProviderMismatch,
    Identity(IdentityError),
}

impl fmt::Display for ProductIdentityError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Random => formatter.write_str("secure identity generation failed"),
            Self::Malformed => formatter.write_str("product identity snapshot is malformed"),
            Self::UnsupportedSchema => {
                formatter.write_str("product identity snapshot schema is unsupported")
            }
            Self::ProviderMismatch => {
                formatter.write_str("protected signing provider does not match persisted identity")
            }
            Self::Identity(error) => fmt::Display::fmt(error, formatter),
        }
    }
}

impl std::error::Error for ProductIdentityError {}

impl From<IdentityError> for ProductIdentityError {
    fn from(error: IdentityError) -> Self {
        Self::Identity(error)
    }
}

fn read_u16(bytes: &[u8], offset: &mut usize) -> Result<u16, ProductIdentityError> {
    Ok(u16::from_be_bytes(read_array(bytes, offset)?))
}

fn read_u64(bytes: &[u8], offset: &mut usize) -> Result<u64, ProductIdentityError> {
    Ok(u64::from_be_bytes(read_array(bytes, offset)?))
}

fn read_verifying_key(
    bytes: &[u8],
    offset: &mut usize,
) -> Result<VerifyingKey, ProductIdentityError> {
    VerifyingKey::from_bytes(read_array(bytes, offset)?)
        .map_err(|_| ProductIdentityError::Malformed)
}

fn read_array<const N: usize>(
    bytes: &[u8],
    offset: &mut usize,
) -> Result<[u8; N], ProductIdentityError> {
    let end = offset
        .checked_add(N)
        .ok_or(ProductIdentityError::Malformed)?;
    let slice = bytes
        .get(*offset..end)
        .ok_or(ProductIdentityError::Malformed)?;
    let mut output = [0_u8; N];
    output.copy_from_slice(slice);
    *offset = end;
    Ok(output)
}

#[cfg(test)]
mod tests {
    use crosslab_crypto::SigningKey;

    use super::*;

    #[test]
    fn product_identity_round_trips_and_validates_provider_binding() {
        let root = SigningKey::generate().unwrap();
        let issuer = SigningKey::generate().unwrap();
        let device = SigningKey::generate().unwrap();
        let state = ProductIdentityState::bootstrap(&root, &issuer, &device).unwrap();

        let decoded = ProductIdentityState::decode(&state.encode()).unwrap();
        decoded.validate_providers(&root, &issuer, &device).unwrap();
        assert_eq!(decoded.owner_id(), state.owner_id());
        assert_eq!(decoded.local_device_id(), state.local_device_id());
    }

    #[test]
    fn product_identity_rejects_wrong_provider() {
        let root = SigningKey::generate().unwrap();
        let issuer = SigningKey::generate().unwrap();
        let device = SigningKey::generate().unwrap();
        let wrong = SigningKey::generate().unwrap();
        let state = ProductIdentityState::bootstrap(&root, &issuer, &device).unwrap();

        assert_eq!(
            state.validate_providers(&root, &issuer, &wrong),
            Err(ProductIdentityError::ProviderMismatch)
        );
    }

    #[test]
    fn product_identity_rejects_tampered_signature() {
        let root = SigningKey::generate().unwrap();
        let issuer = SigningKey::generate().unwrap();
        let device = SigningKey::generate().unwrap();
        let state = ProductIdentityState::bootstrap(&root, &issuer, &device).unwrap();
        let mut encoded = state.encode();
        *encoded.last_mut().unwrap() ^= 1;

        assert!(ProductIdentityState::decode(&encoded).is_err());
    }
}
