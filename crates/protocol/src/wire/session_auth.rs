use crosslab_crypto::{Signature, SignatureAlgorithm, VerifyingKey};
use crosslab_identity::{DeviceCredential, DeviceId, KeyId, OwnerId};
use prost::Message;

use crate::{
    FeatureSet, FrameLimit, MAX_PROTOCOL_RANGES, MAX_REQUIRED_FEATURES, MAX_SUPPORTED_FEATURES,
    ProtocolRange, decode_frame, encode_frame,
};

use super::codec::{ProtocolWireError, copy_32, copy_64};
use super::v1::{
    DeviceCredentialV1, ProtocolRangeV1, SessionAuthBootstrapV1, SessionAuthHelloV1,
    SessionAuthProofV1, SessionAuthRoleV1, SignatureAlgorithmV1, session_auth_bootstrap_v1,
};

pub const SESSION_AUTH_PROFILE_V1: u16 = 1;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SessionAuthRole {
    Initiator,
    Responder,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SessionAuthHello {
    device_credential: Box<DeviceCredential>,
    protocol_ranges: Vec<ProtocolRange>,
    features: FeatureSet,
    nonce: [u8; 32],
}

impl SessionAuthHello {
    pub fn new(
        device_credential: DeviceCredential,
        protocol_ranges: Vec<ProtocolRange>,
        features: FeatureSet,
        nonce: [u8; 32],
    ) -> Self {
        Self {
            device_credential: Box::new(device_credential),
            protocol_ranges,
            features,
            nonce,
        }
    }

    pub const fn owner_id(&self) -> OwnerId {
        self.device_credential.owner_id()
    }

    pub const fn device_id(&self) -> DeviceId {
        self.device_credential.device_id()
    }

    pub const fn device_credential(&self) -> DeviceCredential {
        *self.device_credential
    }

    pub fn protocol_ranges(&self) -> &[ProtocolRange] {
        &self.protocol_ranges
    }

    pub const fn features(&self) -> &FeatureSet {
        &self.features
    }

    pub const fn nonce(&self) -> [u8; 32] {
        self.nonce
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SessionAuthProofMessage {
    role: SessionAuthRole,
    transcript_digest: [u8; 32],
    signature: Signature,
}

impl SessionAuthProofMessage {
    pub const fn new(
        role: SessionAuthRole,
        transcript_digest: [u8; 32],
        signature: Signature,
    ) -> Self {
        Self {
            role,
            transcript_digest,
            signature,
        }
    }

    pub const fn role(&self) -> SessionAuthRole {
        self.role
    }

    pub const fn transcript_digest(&self) -> [u8; 32] {
        self.transcript_digest
    }

    pub const fn signature(&self) -> Signature {
        self.signature
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SessionAuthBootstrapMessage {
    Hello(SessionAuthHello),
    Proof(SessionAuthProofMessage),
}

pub fn encode_session_auth_bootstrap(
    message: &SessionAuthBootstrapMessage,
) -> Result<Vec<u8>, ProtocolWireError> {
    let wire = SessionAuthBootstrapV1::from(message);
    encode_frame(&wire.encode_to_vec(), FrameLimit::BootstrapHello).map_err(Into::into)
}

pub fn decode_session_auth_bootstrap(
    frame: &[u8],
) -> Result<SessionAuthBootstrapMessage, ProtocolWireError> {
    let payload = decode_frame(frame, FrameLimit::BootstrapHello)?;
    let wire = SessionAuthBootstrapV1::decode(payload)
        .map_err(|_| ProtocolWireError::MalformedProtobuf)?;
    wire.try_into()
}

impl From<&SessionAuthBootstrapMessage> for SessionAuthBootstrapV1 {
    fn from(message: &SessionAuthBootstrapMessage) -> Self {
        let body = match message {
            SessionAuthBootstrapMessage::Hello(hello) => session_auth_bootstrap_v1::Body::Hello(
                Box::new(SessionAuthHelloV1::from(hello)),
            ),
            SessionAuthBootstrapMessage::Proof(proof) => {
                session_auth_bootstrap_v1::Body::Proof(SessionAuthProofV1::from(proof))
            }
        };

        Self {
            session_auth_profile: SESSION_AUTH_PROFILE_V1.into(),
            body: Some(body),
        }
    }
}

impl TryFrom<SessionAuthBootstrapV1> for SessionAuthBootstrapMessage {
    type Error = ProtocolWireError;

    fn try_from(wire: SessionAuthBootstrapV1) -> Result<Self, Self::Error> {
        if wire.session_auth_profile != u32::from(SESSION_AUTH_PROFILE_V1) {
            return Err(ProtocolWireError::InvalidSessionAuthProfile(
                wire.session_auth_profile,
            ));
        }

        match wire
            .body
            .ok_or(ProtocolWireError::MissingSessionAuthBootstrapBody)?
        {
            session_auth_bootstrap_v1::Body::Hello(hello) => {
                (*hello).try_into().map(Self::Hello)
            }
            session_auth_bootstrap_v1::Body::Proof(proof) => proof.try_into().map(Self::Proof),
        }
    }
}

impl From<&SessionAuthHello> for SessionAuthHelloV1 {
    fn from(hello: &SessionAuthHello) -> Self {
        Self {
            owner_id: hello.owner_id().to_bytes().to_vec(),
            device_id: hello.device_id().to_bytes().to_vec(),
            device_credential: Some(DeviceCredentialV1::from(hello.device_credential.as_ref())),
            protocol_ranges: hello
                .protocol_ranges
                .iter()
                .copied()
                .map(ProtocolRangeV1::from)
                .collect(),
            supported_features: hello
                .features
                .supported()
                .iter()
                .copied()
                .map(u32::from)
                .collect(),
            required_features: hello
                .features
                .required()
                .iter()
                .copied()
                .map(u32::from)
                .collect(),
            nonce: hello.nonce.to_vec(),
        }
    }
}

impl TryFrom<SessionAuthHelloV1> for SessionAuthHello {
    type Error = ProtocolWireError;

    fn try_from(wire: SessionAuthHelloV1) -> Result<Self, Self::Error> {
        let owner_id = OwnerId::from_bytes(copy_32(
            wire.owner_id,
            ProtocolWireError::InvalidOwnerIdLength,
        )?);
        let device_id = DeviceId::from_bytes(copy_32(
            wire.device_id,
            ProtocolWireError::InvalidDeviceIdLength,
        )?);
        let credential: DeviceCredential = wire
            .device_credential
            .ok_or(ProtocolWireError::MissingDeviceCredential)?
            .try_into()?;
        if credential.owner_id() != owner_id || credential.device_id() != device_id {
            return Err(ProtocolWireError::CredentialIdentityMismatch);
        }
        let protocol_ranges = protocol_ranges_from_wire(wire.protocol_ranges)?;
        let features = features_from_wire(wire.supported_features, wire.required_features)?;
        let nonce = copy_32(wire.nonce, ProtocolWireError::InvalidSessionAuthNonceLength)?;

        Ok(Self::new(credential, protocol_ranges, features, nonce))
    }
}

impl From<ProtocolRange> for ProtocolRangeV1 {
    fn from(range: ProtocolRange) -> Self {
        Self {
            major: range.major().into(),
            min_minor: range.min_minor().into(),
            max_minor: range.max_minor().into(),
        }
    }
}

impl From<&DeviceCredential> for DeviceCredentialV1 {
    fn from(credential: &DeviceCredential) -> Self {
        Self {
            schema_version: credential.schema_version().into(),
            owner_id: credential.owner_id().to_bytes().to_vec(),
            device_id: credential.device_id().to_bytes().to_vec(),
            device_key_id: credential.device_key_id().to_bytes().to_vec(),
            device_algorithm: SignatureAlgorithmV1::Ed25519 as i32,
            device_public_key: credential.device_public_key().to_bytes().to_vec(),
            credential_epoch: credential.credential_epoch(),
            issuer_device_signing_key_id: credential
                .issuer_device_signing_key_id()
                .to_bytes()
                .to_vec(),
            signature_algorithm: SignatureAlgorithmV1::Ed25519 as i32,
            signature: credential.signature().to_bytes().to_vec(),
        }
    }
}

impl TryFrom<DeviceCredentialV1> for DeviceCredential {
    type Error = ProtocolWireError;

    fn try_from(wire: DeviceCredentialV1) -> Result<Self, Self::Error> {
        let schema_version = u16::try_from(wire.schema_version)
            .map_err(|_| ProtocolWireError::InvalidCredentialSchema(wire.schema_version))?;
        validate_signature_algorithm(wire.device_algorithm)?;
        validate_signature_algorithm(wire.signature_algorithm)?;
        let owner_id = OwnerId::from_bytes(copy_32(
            wire.owner_id,
            ProtocolWireError::InvalidOwnerIdLength,
        )?);
        let device_id = DeviceId::from_bytes(copy_32(
            wire.device_id,
            ProtocolWireError::InvalidDeviceIdLength,
        )?);
        let device_key_id = KeyId::from_bytes(copy_32(
            wire.device_key_id,
            ProtocolWireError::InvalidKeyIdLength,
        )?);
        let public_key_bytes = copy_32(
            wire.device_public_key,
            ProtocolWireError::InvalidDevicePublicKeyLength,
        )?;
        let device_public_key = VerifyingKey::from_bytes(public_key_bytes)
            .map_err(|_| ProtocolWireError::InvalidDevicePublicKey)?;
        let issuer_device_signing_key_id = KeyId::from_bytes(copy_32(
            wire.issuer_device_signing_key_id,
            ProtocolWireError::InvalidKeyIdLength,
        )?);
        let signature = Signature::from_bytes(copy_64(
            wire.signature,
            ProtocolWireError::InvalidSignatureLength,
        )?);

        DeviceCredential::from_signed_parts(
            schema_version,
            owner_id,
            device_id,
            device_key_id,
            SignatureAlgorithm::Ed25519,
            device_public_key,
            wire.credential_epoch,
            issuer_device_signing_key_id,
            signature,
        )
        .map_err(|_| ProtocolWireError::InvalidDeviceCredential)
    }
}

impl From<&SessionAuthProofMessage> for SessionAuthProofV1 {
    fn from(proof: &SessionAuthProofMessage) -> Self {
        Self {
            role: role_to_wire(proof.role),
            transcript_digest: proof.transcript_digest.to_vec(),
            signature_algorithm: SignatureAlgorithmV1::Ed25519 as i32,
            signature: proof.signature.to_bytes().to_vec(),
        }
    }
}

impl TryFrom<SessionAuthProofV1> for SessionAuthProofMessage {
    type Error = ProtocolWireError;

    fn try_from(wire: SessionAuthProofV1) -> Result<Self, Self::Error> {
        let role = role_from_wire(wire.role)?;
        let transcript_digest = copy_32(
            wire.transcript_digest,
            ProtocolWireError::InvalidSessionAuthTranscriptDigestLength,
        )?;
        validate_signature_algorithm(wire.signature_algorithm)?;
        let signature = Signature::from_bytes(copy_64(
            wire.signature,
            ProtocolWireError::InvalidSignatureLength,
        )?);
        Ok(Self::new(role, transcript_digest, signature))
    }
}

fn protocol_ranges_from_wire(
    ranges: Vec<ProtocolRangeV1>,
) -> Result<Vec<ProtocolRange>, ProtocolWireError> {
    if ranges.len() > MAX_PROTOCOL_RANGES {
        return Err(ProtocolWireError::TooManyProtocolRanges(ranges.len()));
    }

    ranges
        .into_iter()
        .map(|range| {
            let major = u16::try_from(range.major)
                .map_err(|_| ProtocolWireError::InvalidProtocolVersion)?;
            let min_minor = u16::try_from(range.min_minor)
                .map_err(|_| ProtocolWireError::InvalidProtocolVersion)?;
            let max_minor = u16::try_from(range.max_minor)
                .map_err(|_| ProtocolWireError::InvalidProtocolVersion)?;
            ProtocolRange::new(major, min_minor, max_minor)
                .map_err(|_| ProtocolWireError::InvalidProtocolRange)
        })
        .collect()
}

fn features_from_wire(
    supported: Vec<u32>,
    required: Vec<u32>,
) -> Result<FeatureSet, ProtocolWireError> {
    if supported.len() > MAX_SUPPORTED_FEATURES {
        return Err(ProtocolWireError::TooManySupportedFeatures(supported.len()));
    }
    if required.len() > MAX_REQUIRED_FEATURES {
        return Err(ProtocolWireError::TooManyRequiredFeatures(required.len()));
    }

    let supported = supported
        .into_iter()
        .map(|feature| {
            u16::try_from(feature).map_err(|_| ProtocolWireError::InvalidFeatureId(feature))
        })
        .collect::<Result<Vec<_>, _>>()?;
    let required = required
        .into_iter()
        .map(|feature| {
            u16::try_from(feature).map_err(|_| ProtocolWireError::InvalidFeatureId(feature))
        })
        .collect::<Result<Vec<_>, _>>()?;

    FeatureSet::new(&supported, &required).map_err(|error| match error {
        crate::FeatureNegotiationError::TooManySupportedFeatures => {
            ProtocolWireError::TooManySupportedFeatures(supported.len())
        }
        crate::FeatureNegotiationError::TooManyRequiredFeatures => {
            ProtocolWireError::TooManyRequiredFeatures(required.len())
        }
        crate::FeatureNegotiationError::UnsupportedRequiredFeature(feature) => {
            ProtocolWireError::InvalidFeatureId(u32::from(feature))
        }
    })
}

const fn role_to_wire(role: SessionAuthRole) -> i32 {
    match role {
        SessionAuthRole::Initiator => SessionAuthRoleV1::Initiator as i32,
        SessionAuthRole::Responder => SessionAuthRoleV1::Responder as i32,
    }
}

fn role_from_wire(value: i32) -> Result<SessionAuthRole, ProtocolWireError> {
    match value {
        value if value == SessionAuthRoleV1::Initiator as i32 => Ok(SessionAuthRole::Initiator),
        value if value == SessionAuthRoleV1::Responder as i32 => Ok(SessionAuthRole::Responder),
        value => Err(ProtocolWireError::InvalidSessionAuthRole(value)),
    }
}

fn validate_signature_algorithm(value: i32) -> Result<(), ProtocolWireError> {
    if value == SignatureAlgorithmV1::Ed25519 as i32
        && SignatureAlgorithm::Ed25519.code() == SignatureAlgorithmV1::Ed25519 as u16
    {
        Ok(())
    } else {
        Err(ProtocolWireError::InvalidSignatureAlgorithm(value))
    }
}
