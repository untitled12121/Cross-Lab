use core::fmt;

use crosslab_crypto::{Signature, SignatureAlgorithm, VerifyingKey};
use crosslab_identity::{
    AuthorityDelegation, AuthorityRole, DeviceCredential, DeviceId, KeyId, OwnerId,
    OwnerRootRecord,
};
use crosslab_policy::{PairingTrustTransition, TransitionId};
use prost::Message;

use crate::{FrameLimit, decode_frame, encode_frame};

use super::codec::{ProtocolWireError, copy_16, copy_32, copy_64};
use super::v1::{
    AuthorityDelegationV1, AuthorityRoleV1, OwnerRootRecordV1, PairingAckKindV1, PairingAckV1,
    PairingBootstrapV1, PairingCancelV1, PairingConfirmationV1, PairingCredentialAcceptedV1,
    PairingHelloV1, PairingRoleV1, PairingTrustTransitionV1, ProductPairingCredentialBundleV1,
    ProductPairingTrustBundleV1, ProductPairingV1, SignatureAlgorithmV1, pairing_bootstrap_v1,
    product_pairing_v1,
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

#[derive(Clone, Copy, PartialEq, Eq)]
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

impl fmt::Debug for PairingConfirmation {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("PairingConfirmation")
            .field("role", &self.role)
            .field("pairing_id", &self.pairing_id)
            .field("confirmation", &"[REDACTED]")
            .finish()
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ProductPairingCredentialBundle {
    owner_root: OwnerRootRecord,
    device_signing: AuthorityDelegation,
    credential: DeviceCredential,
}

impl ProductPairingCredentialBundle {
    pub const fn new(
        owner_root: OwnerRootRecord,
        device_signing: AuthorityDelegation,
        credential: DeviceCredential,
    ) -> Self {
        Self {
            owner_root,
            device_signing,
            credential,
        }
    }

    pub const fn owner_root(&self) -> OwnerRootRecord {
        self.owner_root
    }

    pub const fn device_signing(&self) -> AuthorityDelegation {
        self.device_signing
    }

    pub const fn credential(&self) -> DeviceCredential {
        self.credential
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ProductPairingTrustBundleMessage {
    credential: DeviceCredential,
    transition: PairingTrustTransition,
}

impl ProductPairingTrustBundleMessage {
    pub const fn new(
        credential: DeviceCredential,
        transition: PairingTrustTransition,
    ) -> Self {
        Self {
            credential,
            transition,
        }
    }

    pub const fn credential(&self) -> DeviceCredential {
        self.credential
    }

    pub const fn transition(&self) -> PairingTrustTransition {
        self.transition
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProductPairingAckKind {
    Persisted,
    Complete,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ProductPairingAck {
    pairing_id: [u8; 16],
    role: PairingRole,
    kind: ProductPairingAckKind,
}

impl ProductPairingAck {
    pub const fn new(
        pairing_id: [u8; 16],
        role: PairingRole,
        kind: ProductPairingAckKind,
    ) -> Self {
        Self {
            pairing_id,
            role,
            kind,
        }
    }

    pub const fn pairing_id(&self) -> [u8; 16] {
        self.pairing_id
    }

    pub const fn role(&self) -> PairingRole {
        self.role
    }

    pub const fn kind(&self) -> ProductPairingAckKind {
        self.kind
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProductPairingMessage {
    Hello(PairingHello),
    Confirmation(PairingConfirmation),
    CredentialBundle(ProductPairingCredentialBundle),
    CredentialAccepted(PairingCredentialAccepted),
    TrustBundle(ProductPairingTrustBundleMessage),
    Ack(ProductPairingAck),
    Cancel { pairing_id: [u8; 16] },
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

pub fn encode_product_pairing(
    message: &ProductPairingMessage,
) -> Result<Vec<u8>, ProtocolWireError> {
    let wire = ProductPairingV1::from(message);
    encode_frame(&wire.encode_to_vec(), FrameLimit::BootstrapHello).map_err(Into::into)
}

pub fn decode_product_pairing(frame: &[u8]) -> Result<ProductPairingMessage, ProtocolWireError> {
    let payload = decode_frame(frame, FrameLimit::BootstrapHello)?;
    let wire =
        ProductPairingV1::decode(payload).map_err(|_| ProtocolWireError::MalformedProtobuf)?;
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

impl From<&ProductPairingMessage> for ProductPairingV1 {
    fn from(message: &ProductPairingMessage) -> Self {
        let body = match message {
            ProductPairingMessage::Hello(hello) => {
                product_pairing_v1::Body::Hello(PairingHelloV1::from(hello))
            }
            ProductPairingMessage::Confirmation(confirmation) => {
                product_pairing_v1::Body::Confirmation(PairingConfirmationV1::from(confirmation))
            }
            ProductPairingMessage::CredentialBundle(bundle) => {
                product_pairing_v1::Body::CredentialBundle(ProductPairingCredentialBundleV1 {
                    owner_root: Some(OwnerRootRecordV1::from(&bundle.owner_root)),
                    device_signing: Some(AuthorityDelegationV1::from(&bundle.device_signing)),
                    credential: Some(super::v1::DeviceCredentialV1::from(&bundle.credential)),
                })
            }
            ProductPairingMessage::CredentialAccepted(accepted) => {
                product_pairing_v1::Body::CredentialAccepted(PairingCredentialAcceptedV1::from(
                    accepted,
                ))
            }
            ProductPairingMessage::TrustBundle(bundle) => {
                product_pairing_v1::Body::TrustBundle(ProductPairingTrustBundleV1 {
                    credential: Some(super::v1::DeviceCredentialV1::from(&bundle.credential)),
                    transition: Some(PairingTrustTransitionV1::from(&bundle.transition)),
                })
            }
            ProductPairingMessage::Ack(ack) => {
                product_pairing_v1::Body::Ack(PairingAckV1 {
                    pairing_id: ack.pairing_id.to_vec(),
                    role: role_to_wire(ack.role),
                    kind: ack_kind_to_wire(ack.kind),
                })
            }
            ProductPairingMessage::Cancel { pairing_id } => {
                product_pairing_v1::Body::Cancel(PairingCancelV1 {
                    pairing_id: pairing_id.to_vec(),
                })
            }
        };

        Self {
            pairing_profile: PAIRING_PROFILE_V1.into(),
            body: Some(body),
        }
    }
}

impl TryFrom<ProductPairingV1> for ProductPairingMessage {
    type Error = ProtocolWireError;

    fn try_from(wire: ProductPairingV1) -> Result<Self, Self::Error> {
        if wire.pairing_profile != u32::from(PAIRING_PROFILE_V1) {
            return Err(ProtocolWireError::InvalidPairingProfile(
                wire.pairing_profile,
            ));
        }

        match wire
            .body
            .ok_or(ProtocolWireError::MissingProductPairingBody)?
        {
            product_pairing_v1::Body::Hello(hello) => hello.try_into().map(Self::Hello),
            product_pairing_v1::Body::Confirmation(confirmation) => {
                confirmation.try_into().map(Self::Confirmation)
            }
            product_pairing_v1::Body::CredentialBundle(bundle) => {
                let owner_root = bundle
                    .owner_root
                    .ok_or(ProtocolWireError::MissingProductPairingOwnerRoot)?
                    .try_into()?;
                let device_signing = bundle
                    .device_signing
                    .ok_or(ProtocolWireError::MissingProductPairingDelegation)?
                    .try_into()?;
                let credential = bundle
                    .credential
                    .ok_or(ProtocolWireError::MissingProductPairingCredential)?
                    .try_into()?;
                Ok(Self::CredentialBundle(ProductPairingCredentialBundle::new(
                    owner_root,
                    device_signing,
                    credential,
                )))
            }
            product_pairing_v1::Body::CredentialAccepted(accepted) => {
                accepted.try_into().map(Self::CredentialAccepted)
            }
            product_pairing_v1::Body::TrustBundle(bundle) => {
                let credential = bundle
                    .credential
                    .ok_or(ProtocolWireError::MissingProductPairingCredential)?
                    .try_into()?;
                let transition = bundle
                    .transition
                    .ok_or(ProtocolWireError::MissingProductPairingTrustTransition)?
                    .try_into()?;
                Ok(Self::TrustBundle(ProductPairingTrustBundleMessage::new(
                    credential,
                    transition,
                )))
            }
            product_pairing_v1::Body::Ack(ack) => Ok(Self::Ack(ProductPairingAck::new(
                copy_16(
                    ack.pairing_id,
                    ProtocolWireError::InvalidPairingIdLength,
                )?,
                role_from_wire(ack.role)?,
                ack_kind_from_wire(ack.kind)?,
            ))),
            product_pairing_v1::Body::Cancel(cancel) => Ok(Self::Cancel {
                pairing_id: copy_16(
                    cancel.pairing_id,
                    ProtocolWireError::InvalidPairingIdLength,
                )?,
            }),
        }
    }
}

impl From<&OwnerRootRecord> for OwnerRootRecordV1 {
    fn from(root: &OwnerRootRecord) -> Self {
        Self {
            schema_version: root.schema_version().into(),
            owner_id: root.owner_id().to_bytes().to_vec(),
            root_key_id: root.root_key_id().to_bytes().to_vec(),
            root_algorithm: SignatureAlgorithmV1::Ed25519 as i32,
            root_public_key: root.root_public_key().to_bytes().to_vec(),
            root_epoch: root.root_epoch(),
        }
    }
}

impl TryFrom<OwnerRootRecordV1> for OwnerRootRecord {
    type Error = ProtocolWireError;

    fn try_from(wire: OwnerRootRecordV1) -> Result<Self, Self::Error> {
        if wire.schema_version != 1 {
            return Err(ProtocolWireError::InvalidAuthorityRecord);
        }
        validate_signature_algorithm(wire.root_algorithm)?;
        let owner_id = OwnerId::from_bytes(copy_32(
            wire.owner_id,
            ProtocolWireError::InvalidOwnerIdLength,
        )?);
        let root_key_id = KeyId::from_bytes(copy_32(
            wire.root_key_id,
            ProtocolWireError::InvalidKeyIdLength,
        )?);
        let public_key = VerifyingKey::from_bytes(copy_32(
            wire.root_public_key,
            ProtocolWireError::InvalidDevicePublicKeyLength,
        )?)
        .map_err(|_| ProtocolWireError::InvalidDevicePublicKey)?;
        let root = OwnerRootRecord::from_public_key(owner_id, public_key, wire.root_epoch);
        if root.root_key_id() != root_key_id {
            return Err(ProtocolWireError::InvalidAuthorityRecord);
        }
        Ok(root)
    }
}

impl From<&AuthorityDelegation> for AuthorityDelegationV1 {
    fn from(delegation: &AuthorityDelegation) -> Self {
        Self {
            schema_version: 1,
            owner_id: delegation.owner_id().to_bytes().to_vec(),
            role: authority_role_to_wire(delegation.role()),
            delegated_key_id: delegation.delegated_key_id().to_bytes().to_vec(),
            delegated_algorithm: SignatureAlgorithmV1::Ed25519 as i32,
            delegated_public_key: delegation.delegated_public_key().to_bytes().to_vec(),
            delegation_epoch: delegation.delegation_epoch(),
            issuer_root_key_id: delegation.issuer_root_key_id().to_bytes().to_vec(),
            signature_algorithm: SignatureAlgorithmV1::Ed25519 as i32,
            signature: delegation.signature().to_bytes().to_vec(),
        }
    }
}

impl TryFrom<AuthorityDelegationV1> for AuthorityDelegation {
    type Error = ProtocolWireError;

    fn try_from(wire: AuthorityDelegationV1) -> Result<Self, Self::Error> {
        let schema_version = u16::try_from(wire.schema_version)
            .map_err(|_| ProtocolWireError::InvalidAuthorityRecord)?;
        let owner_id = OwnerId::from_bytes(copy_32(
            wire.owner_id,
            ProtocolWireError::InvalidOwnerIdLength,
        )?);
        let role = authority_role_from_wire(wire.role)?;
        let expected_key_id = KeyId::from_bytes(copy_32(
            wire.delegated_key_id,
            ProtocolWireError::InvalidKeyIdLength,
        )?);
        validate_signature_algorithm(wire.delegated_algorithm)?;
        validate_signature_algorithm(wire.signature_algorithm)?;
        let public_key = VerifyingKey::from_bytes(copy_32(
            wire.delegated_public_key,
            ProtocolWireError::InvalidDevicePublicKeyLength,
        )?)
        .map_err(|_| ProtocolWireError::InvalidDevicePublicKey)?;
        let issuer_root_key_id = KeyId::from_bytes(copy_32(
            wire.issuer_root_key_id,
            ProtocolWireError::InvalidKeyIdLength,
        )?);
        let signature = Signature::from_bytes(copy_64(
            wire.signature,
            ProtocolWireError::InvalidSignatureLength,
        )?);
        let delegation = AuthorityDelegation::from_unverified_signed_parts(
            schema_version,
            owner_id,
            role,
            SignatureAlgorithm::Ed25519,
            public_key,
            wire.delegation_epoch,
            issuer_root_key_id,
            signature,
        )
        .map_err(|_| ProtocolWireError::InvalidAuthorityRecord)?;
        if delegation.delegated_key_id() != expected_key_id {
            return Err(ProtocolWireError::InvalidAuthorityRecord);
        }
        Ok(delegation)
    }
}

impl From<&PairingTrustTransition> for PairingTrustTransitionV1 {
    fn from(transition: &PairingTrustTransition) -> Self {
        Self {
            schema_version: transition.schema_version().into(),
            owner_id: transition.owner_id().to_bytes().to_vec(),
            device_id: transition.device_id().to_bytes().to_vec(),
            credential_epoch: transition.credential_epoch(),
            credential_signed_object_digest: transition
                .credential_signed_object_digest()
                .to_vec(),
            transition_id: transition.transition_id().to_bytes().to_vec(),
            pairing_evidence_digest: transition.pairing_evidence_digest().to_vec(),
            issuer_key_id: transition.issuer_key_id().to_bytes().to_vec(),
            signature_algorithm: SignatureAlgorithmV1::Ed25519 as i32,
            signature: transition.signature().to_bytes().to_vec(),
        }
    }
}

impl TryFrom<PairingTrustTransitionV1> for PairingTrustTransition {
    type Error = ProtocolWireError;

    fn try_from(wire: PairingTrustTransitionV1) -> Result<Self, Self::Error> {
        let schema_version = u16::try_from(wire.schema_version)
            .map_err(|_| ProtocolWireError::InvalidPairingTrustTransition)?;
        if schema_version != 1 {
            return Err(ProtocolWireError::InvalidPairingTrustTransition);
        }
        validate_signature_algorithm(wire.signature_algorithm)?;
        Ok(PairingTrustTransition::from_unverified_signed_parts(
            schema_version,
            OwnerId::from_bytes(copy_32(
                wire.owner_id,
                ProtocolWireError::InvalidOwnerIdLength,
            )?),
            DeviceId::from_bytes(copy_32(
                wire.device_id,
                ProtocolWireError::InvalidDeviceIdLength,
            )?),
            wire.credential_epoch,
            copy_32(
                wire.credential_signed_object_digest,
                ProtocolWireError::InvalidSignedObjectDigestLength,
            )?,
            TransitionId::from_bytes(copy_32(
                wire.transition_id,
                ProtocolWireError::InvalidTransitionIdLength,
            )?),
            copy_32(
                wire.pairing_evidence_digest,
                ProtocolWireError::InvalidSignedObjectDigestLength,
            )?,
            KeyId::from_bytes(copy_32(
                wire.issuer_key_id,
                ProtocolWireError::InvalidKeyIdLength,
            )?),
            Signature::from_bytes(copy_64(
                wire.signature,
                ProtocolWireError::InvalidSignatureLength,
            )?),
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

const fn ack_kind_to_wire(kind: ProductPairingAckKind) -> i32 {
    match kind {
        ProductPairingAckKind::Persisted => PairingAckKindV1::Persisted as i32,
        ProductPairingAckKind::Complete => PairingAckKindV1::Complete as i32,
    }
}

fn ack_kind_from_wire(value: i32) -> Result<ProductPairingAckKind, ProtocolWireError> {
    match value {
        value if value == PairingAckKindV1::Persisted as i32 => {
            Ok(ProductPairingAckKind::Persisted)
        }
        value if value == PairingAckKindV1::Complete as i32 => Ok(ProductPairingAckKind::Complete),
        value => Err(ProtocolWireError::InvalidPairingAckKind(value)),
    }
}

const fn authority_role_to_wire(role: AuthorityRole) -> i32 {
    match role {
        AuthorityRole::DeviceSigning => AuthorityRoleV1::DeviceSigning as i32,
        AuthorityRole::Administrative => AuthorityRoleV1::Administrative as i32,
        AuthorityRole::Recovery => AuthorityRoleV1::Recovery as i32,
        AuthorityRole::OwnerRoot => AuthorityRoleV1::Unspecified as i32,
    }
}

fn authority_role_from_wire(value: i32) -> Result<AuthorityRole, ProtocolWireError> {
    match value {
        value if value == AuthorityRoleV1::DeviceSigning as i32 => Ok(AuthorityRole::DeviceSigning),
        value if value == AuthorityRoleV1::Administrative as i32 => Ok(AuthorityRole::Administrative),
        value if value == AuthorityRoleV1::Recovery as i32 => Ok(AuthorityRole::Recovery),
        value => Err(ProtocolWireError::InvalidAuthorityRole(value)),
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
