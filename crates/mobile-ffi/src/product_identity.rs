use std::sync::Arc;

use crosslab_crypto::{Signature, SigningProvider, SigningProviderError, VerifyingKey};
use crosslab_identity_store::{ProductIdentityError, ProductIdentityState};

#[derive(Debug, Clone, Copy, PartialEq, Eq, uniffi::Error)]
pub enum MobileSigningCallbackError {
    ProviderFailed,
}

impl core::fmt::Display for MobileSigningCallbackError {
    fn fmt(&self, formatter: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        formatter.write_str("platform signing provider operation failed")
    }
}

impl std::error::Error for MobileSigningCallbackError {}

impl From<uniffi::UnexpectedUniFFICallbackError> for MobileSigningCallbackError {
    fn from(_: uniffi::UnexpectedUniFFICallbackError) -> Self {
        Self::ProviderFailed
    }
}

#[uniffi::export(foreign)]
pub trait MobileSigningProvider: Send + Sync {
    fn public_key(&self) -> Result<Vec<u8>, MobileSigningCallbackError>;

    fn sign(&self, message: Vec<u8>) -> Result<Vec<u8>, MobileSigningCallbackError>;
}

#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct MobileProductIdentity {
    pub created: bool,
    pub payload: Vec<u8>,
    pub owner_id: String,
    pub local_device_id: String,
}

impl MobileProductIdentity {
    pub(crate) fn from_state(state: &ProductIdentityState, created: bool) -> Self {
        Self {
            created,
            payload: state.encode(),
            owner_id: short_hex(state.owner_id().as_bytes()),
            local_device_id: short_hex(state.local_device_id().as_bytes()),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, uniffi::Error)]
pub enum MobileProductIdentityError {
    SigningProvider,
    InvalidPublicKey,
    InvalidSignature,
    Random,
    Malformed,
    UnsupportedSchema,
    ProviderMismatch,
    Identity,
}

impl core::fmt::Display for MobileProductIdentityError {
    fn fmt(&self, formatter: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        formatter.write_str(match self {
            Self::SigningProvider => "platform signing provider operation failed",
            Self::InvalidPublicKey => "platform signing provider returned an invalid public key",
            Self::InvalidSignature => "platform signing provider returned an invalid signature",
            Self::Random => "secure identity generation failed",
            Self::Malformed => "product identity snapshot is malformed",
            Self::UnsupportedSchema => "product identity snapshot schema is unsupported",
            Self::ProviderMismatch => {
                "protected signing provider does not match persisted identity"
            }
            Self::Identity => "product identity verification failed",
        })
    }
}

impl std::error::Error for MobileProductIdentityError {}

#[uniffi::export]
pub fn product_identity_load(
    current_payload: Vec<u8>,
    local_device_signer: Arc<dyn MobileSigningProvider>,
) -> Result<MobileProductIdentity, MobileProductIdentityError> {
    let local = ForeignSigningProvider::new(local_device_signer)?;
    let state = ProductIdentityState::decode(&current_payload)?;
    state.validate_local_device_provider(&local)?;
    Ok(MobileProductIdentity::from_state(&state, false))
}

#[uniffi::export]
pub fn product_identity_load_or_create(
    current_payload: Option<Vec<u8>>,
    root_signer: Arc<dyn MobileSigningProvider>,
    device_signing_signer: Arc<dyn MobileSigningProvider>,
    local_device_signer: Arc<dyn MobileSigningProvider>,
) -> Result<MobileProductIdentity, MobileProductIdentityError> {
    let root = ForeignSigningProvider::new(root_signer)?;
    let issuer = ForeignSigningProvider::new(device_signing_signer)?;
    let device = ForeignSigningProvider::new(local_device_signer)?;

    let (state, created) = match current_payload {
        Some(payload) => {
            let state = ProductIdentityState::decode(&payload)?;
            state.validate_providers(&root, &issuer, &device)?;
            (state, false)
        }
        None => (
            ProductIdentityState::bootstrap(&root, &issuer, &device)?,
            true,
        ),
    };

    Ok(MobileProductIdentity::from_state(&state, created))
}

pub(crate) struct ForeignSigningProvider {
    inner: Arc<dyn MobileSigningProvider>,
    verifying_key: VerifyingKey,
}

impl ForeignSigningProvider {
    pub(crate) fn new(
        inner: Arc<dyn MobileSigningProvider>,
    ) -> Result<Self, MobileProductIdentityError> {
        let bytes = inner
            .public_key()
            .map_err(|_| MobileProductIdentityError::SigningProvider)?;
        let bytes: [u8; 32] = bytes
            .try_into()
            .map_err(|_| MobileProductIdentityError::InvalidPublicKey)?;
        let verifying_key = VerifyingKey::from_bytes(bytes)
            .map_err(|_| MobileProductIdentityError::InvalidPublicKey)?;

        Ok(Self {
            inner,
            verifying_key,
        })
    }
}

impl SigningProvider for ForeignSigningProvider {
    fn verifying_key(&self) -> VerifyingKey {
        self.verifying_key
    }

    fn sign_message(&self, message: &[u8]) -> Result<Signature, SigningProviderError> {
        let bytes = self
            .inner
            .sign(message.to_vec())
            .map_err(|_| SigningProviderError)?;
        let bytes: [u8; 64] = bytes.try_into().map_err(|_| SigningProviderError)?;
        Ok(Signature::from_bytes(bytes))
    }
}

impl From<ProductIdentityError> for MobileProductIdentityError {
    fn from(error: ProductIdentityError) -> Self {
        match error {
            ProductIdentityError::Random => Self::Random,
            ProductIdentityError::Malformed => Self::Malformed,
            ProductIdentityError::UnsupportedSchema => Self::UnsupportedSchema,
            ProductIdentityError::ProviderMismatch => Self::ProviderMismatch,
            ProductIdentityError::PeerLimit
            | ProductIdentityError::PeerOwnerMismatch
            | ProductIdentityError::LocalDeviceAsPeer
            | ProductIdentityError::DuplicatePeer
            | ProductIdentityError::Identity(_)
            | ProductIdentityError::Trust(_) => Self::Identity,
        }
    }
}

fn short_hex(bytes: &[u8; 32]) -> String {
    use core::fmt::Write as _;

    let mut output = String::with_capacity(16);
    for byte in &bytes[..8] {
        write!(&mut output, "{byte:02x}").expect("writing to String cannot fail");
    }
    output
}
