use crosslab_crypto::{Signature, SignatureAlgorithm, VerifyingKey};
use crosslab_identity::{DeviceId, KeyId, OwnerId};
use prost::Message;

use crate::{FrameLimit, decode_frame, encode_frame};

use super::codec::{ProtocolWireError, copy_16, copy_32, copy_64};
use super::v1::{
    PairingBootstrapV1, PairingConfirmationV1, PairingCredentialAcceptedV1, PairingHelloV1,
    PairingRoleV1, SignatureAlgorithmV1, pairing_bootstrap_v1,
};

pub const PAIRING_PROFILE_V1: u16 = 1;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PairingRole {
    Inviter,
    Joiner,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PairingHello {
    role: PairingRole,
    protocol_major: u16,
    pairing_id: [u8; 16],
    owner_id: OwnerId,
    device_id: DeviceId,
    device_public_key: VerifyingKey,
    nonce: [u8; 32],
}

impl PairingHello {
    pub fn new(
        role: PairingRole,
        protocol_major: u16,
        pairing_id: [u8; 16],
        owner_id: OwnerId,
        device_id: DeviceId,
        device_public_key: VerifyingKey,
        nonce: [u8; 32],
    ) -> Self {
        Self {
            role,
            protocol_major,
            pairing_id,
            owner_id,
            device_id,
            device_public_key,
            nonce,
        }
    }

    pub const fn role(&self) -> PairingRole {
        self.role
    }

    pub const fn protocol_major(&self) -> u16 {
        self.protocol_major
    }

    pub const fn pairing_id(&self) -> [u8; 16] {
        self.pairing_id
    }

    pub const fn owner_id(&self) -> OwnerId {
        self.owner_id
    }

    pub const fn device_id(&self) -> DeviceId {
        self.device_id
    }

    pub const fn device_public_key(&self) -> VerifyingKey {
        self.device_public_key
    }

    pub const fn nonce(&self) -> [u8; 32] {
        self.nonce
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PairingConfirmation {
    role: PairingRole,
    pairing_id: [u8; 16],
    confirmation: [u8; 32],
}

impl PairingConfirmation {
    pub const fn new(role: PairingRole, pairing_id: [u8; 16], confirmation: [u8; 32]) -> Self {
        Self {
            role,
            pairing_id,
            confirmation,
        }
    }

    pub const fn role(&self) -> PairingRole {
        self.role
    }

    pub const fn pairing_id(&self) -> [u8; 16] {
        self.pairing_id
    }

    pub const fn confirmation(&self) -> [u8; 32] {
        self.confirmation
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PairingCredentialAccepted {
    pairing_id: [u8; 16],
    pairing_transcript_digest: [u8; 32],
    device_credential_signed_object_digest: [u8; 32],
    joiner_device_id: DeviceId,
    joiner_device_key_id: KeyId,
    signature: Signature,
}

impl PairingCredentialAccepted {
    pub const fn new(
        pairing_id: [u8; 16],
        pairing_transcript_digest: [u8; 32],
        device_credential_signed_object_digest: [u8; 32],
        joiner_device_id: DeviceId,
        joiner_device_key_id: KeyId,
        signature: Signature,
    ) -> Self {
        Self {
            pairing_id,
            pairing_transcript_digest,
            device_credential_signed_object_digest,
            joiner_device_id,
            joiner_device_key_id,
            signature,
        }
    }

    pub const fn pairing_id(&self) -> [u8; 16] {
        self.pairing_id
    }

    pub const fn pairing_transcript_digest(&self) -> [u8; 32] {
        self.pairing_transcript_digest
    }

    pub const fn device_credential_signed_object_digest(&self) -> [u8; 32] {
        self.device_credential_signed_object_digest
    }

    pub const fn joiner_device_id(&self) -> DeviceId {
        self.joiner_device_id
    }

    pub const fn joiner_device_key_id(&self) -> KeyId {
        self.joiner_device_key_id
    }

    pub const fn signature(&self) -> Signature {
        self.signature
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PairingBootstrapMessage {
    Hello(PairingHello),
    Confirmation(PairingConfirmation),
    CredentialAccepted(PairingCredentialAccepted),
}

pub fn encode_pairing_bootstrap(
    message: &PairingBootstrapMessage,
) -> Result<Vec<u8>, ProtocolWireError> {
    let wire = PairingBootstrapV1::from(message);
    encode_frame(&wire.encode_to_vec(), FrameLimit::BootstrapHello).map_err(Into::into)
}

pub fn decode_pairing_bootstrap(
    frame: &[u8],
) -> Result<PairingBootstrapMessage, ProtocolWireError> {
    let payload = decode_frame(frame, FrameLimit::BootstrapHello)?;
    let wire =
        PairingBootstrapV1::decode(payload).map_err(|_| ProtocolWireError::MalformedProtobuf)?;
    wire.try_into()
}

impl From<&PairingBootstrapMessage> for PairingBootstrapV1 {
    fn from(message: &PairingBootstrapMessage) -> Self {
        let body = match message {
            PairingBootstrapMessage::Hello(hello) => {
                pairing_bootstrap_v1::Body::Hello(PairingHelloV1::from(hello))
            }
            PairingBootstrapMessage::Confirmation(confirmation) => {
                pairing_bootstrap_v1::Body::Confirmation(PairingConfirmationV1::from(confirmation))
            }
            PairingBootstrapMessage::CredentialAccepted(accepted) => {
                pairing_bootstrap_v1::Body::CredentialAccepted(PairingCredentialAcceptedV1::from(
                    accepted,
                ))
            }
        };

        Self {
            pairing_profile: PAIRING_PROFILE_V1.into(),
            body: Some(body),
        }
    }
}

impl TryFrom<PairingBootstrapV1> for PairingBootstrapMessage {
    type Error = ProtocolWireError;

    fn try_from(wire: PairingBootstrapV1) -> Result<Self, Self::Error> {
        if wire.pairing_profile != u32::from(PAIRING_PROFILE_V1) {
            return Err(ProtocolWireError::InvalidPairingProfile(
                wire.pairing_profile,
            ));
        }

        match wire
            .body
            .ok_or(ProtocolWireError::MissingPairingBootstrapBody)?
        {
            pairing_bootstrap_v1::Body::Hello(hello) => hello.try_into().map(Self::Hello),
            pairing_bootstrap_v1::Body::Confirmation(confirmation) => {
                confirmation.try_into().map(Self::Confirmation)
            }
            pairing_bootstrap_v1::Body::CredentialAccepted(accepted) => {
                accepted.try_into().map(Self::CredentialAccepted)
            }
        }
    }
}

impl From<&PairingHello> for PairingHelloV1 {
    fn from(hello: &PairingHello) -> Self {
        Self {
            role: role_to_wire(hello.role),
            protocol_major: hello.protocol_major.into(),
            pairing_id: hello.pairing_id.to_vec(),
            owner_id: hello.owner_id.to_bytes().to_vec(),
            device_id: hello.device_id.to_bytes().to_vec(),
            device_algorithm: SignatureAlgorithmV1::Ed25519 as i32,
            device_public_key: hello.device_public_key.to_bytes().to_vec(),
            nonce: hello.nonce.to_vec(),
        }
    }
}

impl TryFrom<PairingHelloV1> for PairingHello {
    type Error = ProtocolWireError;

    fn try_from(wire: PairingHelloV1) -> Result<Self, Self::Error> {
        let role = role_from_wire(wire.role)?;
        let protocol_major = u16::try_from(wire.protocol_major)
            .map_err(|_| ProtocolWireError::InvalidProtocolVersion)?;
        let pairing_id = copy_16(wire.pairing_id, ProtocolWireError::InvalidPairingIdLength)?;
        let owner_id = OwnerId::from_bytes(copy_32(
            wire.owner_id,
            ProtocolWireError::InvalidOwnerIdLength,
        )?);
        let device_id = DeviceId::from_bytes(copy_32(
            wire.device_id,
            ProtocolWireError::InvalidDeviceIdLength,
        )?);
        validate_signature_algorithm(wire.device_algorithm)?;
        let device_public_key_bytes = copy_32(
            wire.device_public_key,
            ProtocolWireError::InvalidDevicePublicKeyLength,
        )?;
        let device_public_key = VerifyingKey::from_bytes(device_public_key_bytes)
            .map_err(|_| ProtocolWireError::InvalidDevicePublicKey)?;
        let nonce = copy_32(wire.nonce, ProtocolWireError::InvalidPairingNonceLength)?;

        Ok(Self::new(
            role,
            protocol_major,
            pairing_id,
            owner_id,
            device_id,
            device_public_key,
            nonce,
        ))
    }
}

impl From<&PairingConfirmation> for PairingConfirmationV1 {
    fn from(confirmation: &PairingConfirmation) -> Self {
        Self {
            role: role_to_wire(confirmation.role),
            pairing_id: confirmation.pairing_id.to_vec(),
            confirmation: confirmation.confirmation.to_vec(),
        }
    }
}

impl TryFrom<PairingConfirmationV1> for PairingConfirmation {
    type Error = ProtocolWireError;

    fn try_from(wire: PairingConfirmationV1) -> Result<Self, Self::Error> {
        Ok(Self::new(
            role_from_wire(wire.role)?,
            copy_16(wire.pairing_id, ProtocolWireError::InvalidPairingIdLength)?,
            copy_32(
                wire.confirmation,
                ProtocolWireError::InvalidPairingConfirmationLength,
            )?,
        ))
    }
}

impl From<&PairingCredentialAccepted> for PairingCredentialAcceptedV1 {
    fn from(accepted: &PairingCredentialAccepted) -> Self {
        Self {
            pairing_id: accepted.pairing_id.to_vec(),
            pairing_transcript_digest: accepted.pairing_transcript_digest.to_vec(),
            device_credential_signed_object_digest: accepted
                .device_credential_signed_object_digest
                .to_vec(),
            joiner_device_id: accepted.joiner_device_id.to_bytes().to_vec(),
            joiner_device_key_id: accepted.joiner_device_key_id.to_bytes().to_vec(),
            signature_algorithm: SignatureAlgorithmV1::Ed25519 as i32,
            signature: accepted.signature.to_bytes().to_vec(),
        }
    }
}

impl TryFrom<PairingCredentialAcceptedV1> for PairingCredentialAccepted {
    type Error = ProtocolWireError;

    fn try_from(wire: PairingCredentialAcceptedV1) -> Result<Self, Self::Error> {
        let pairing_id = copy_16(wire.pairing_id, ProtocolWireError::InvalidPairingIdLength)?;
        let pairing_transcript_digest = copy_32(
            wire.pairing_transcript_digest,
            ProtocolWireError::InvalidPairingTranscriptDigestLength,
        )?;
        let device_credential_signed_object_digest = copy_32(
            wire.device_credential_signed_object_digest,
            ProtocolWireError::InvalidSignedObjectDigestLength,
        )?;
        let joiner_device_id = DeviceId::from_bytes(copy_32(
            wire.joiner_device_id,
            ProtocolWireError::InvalidDeviceIdLength,
        )?);
        let joiner_device_key_id = KeyId::from_bytes(copy_32(
            wire.joiner_device_key_id,
            ProtocolWireError::InvalidKeyIdLength,
        )?);
        validate_signature_algorithm(wire.signature_algorithm)?;
        let signature = Signature::from_bytes(copy_64(
            wire.signature,
            ProtocolWireError::InvalidSignatureLength,
        )?);

        Ok(Self::new(
            pairing_id,
            pairing_transcript_digest,
            device_credential_signed_object_digest,
            joiner_device_id,
            joiner_device_key_id,
            signature,
        ))
    }
}

const fn role_to_wire(role: PairingRole) -> i32 {
    match role {
        PairingRole::Inviter => PairingRoleV1::Inviter as i32,
        PairingRole::Joiner => PairingRoleV1::Joiner as i32,
    }
}

fn role_from_wire(value: i32) -> Result<PairingRole, ProtocolWireError> {
    match value {
        value if value == PairingRoleV1::Inviter as i32 => Ok(PairingRole::Inviter),
        value if value == PairingRoleV1::Joiner as i32 => Ok(PairingRole::Joiner),
        value => Err(ProtocolWireError::InvalidPairingRole(value)),
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
