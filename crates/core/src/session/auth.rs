use core::fmt;

use crosslab_crypto::{
    CanonicalTranscript, Signature, SignatureAlgorithm, SigningKey, VerifyingKey, blake3_256,
    signed_object_digest,
};
use crosslab_identity::{DeviceCredential, DeviceId, KeyId, OwnerId};
use crosslab_policy::SessionId;
use crosslab_protocol::{MAX_SUPPORTED_FEATURES, ProtocolVersion};

const DOMAIN: &str = "crosslab.session-auth.v1";
const FEATURE_SET_DOMAIN: &[u8] = b"crosslab.session-auth.feature-set.v1\0";
const CHANNEL_BINDING_PROFILE_DOMAIN: &[u8] = b"crosslab.session-auth.channel-binding-profile.v1\0";
const CHANNEL_BINDING_VALUE_DOMAIN: &[u8] = b"crosslab.session-auth.channel-binding-value.v1\0";
const INITIATOR_PROOF_LABEL: &[u8] = b"crosslab.session-auth.initiator-proof.v1";
const RESPONDER_PROOF_LABEL: &[u8] = b"crosslab.session-auth.responder-proof.v1";
const SESSION_ID_DOMAIN: &[u8] = b"crosslab.session-id.v1\0";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SessionAuthRole {
    Initiator,
    Responder,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SessionAuthError {
    WrongOwner,
    TooManyFeatures,
    WrongProofRole,
    WrongProofTranscript,
    WrongDeviceKey,
    InvalidProof,
}

impl fmt::Display for SessionAuthError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::WrongOwner => "session authentication credentials belong to a different owner",
            Self::TooManyFeatures => "session authentication feature set exceeds the v1 limit",
            Self::WrongProofRole => "session authentication proof has the wrong role",
            Self::WrongProofTranscript => "session authentication proof targets another transcript",
            Self::WrongDeviceKey => "session authentication proof uses the wrong device key",
            Self::InvalidProof => "session authentication proof signature is invalid",
        })
    }
}

impl std::error::Error for SessionAuthError {}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SessionAuthProof {
    role: SessionAuthRole,
    transcript_digest: [u8; 32],
    signature: Signature,
}

impl SessionAuthProof {
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
pub struct SessionAuthTranscriptV1 {
    owner_id: OwnerId,
    initiator_device_id: DeviceId,
    initiator_device_key_id: KeyId,
    initiator_credential_signed_object_digest: [u8; 32],
    initiator_nonce: [u8; 32],
    responder_device_id: DeviceId,
    responder_device_key_id: KeyId,
    responder_credential_signed_object_digest: [u8; 32],
    responder_nonce: [u8; 32],
    negotiated_protocol: ProtocolVersion,
    negotiated_feature_set_digest: [u8; 32],
    channel_binding_profile_digest: [u8; 32],
    channel_binding_value_digest: [u8; 32],
}

impl SessionAuthTranscriptV1 {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        owner_id: OwnerId,
        initiator_credential: &DeviceCredential,
        initiator_nonce: [u8; 32],
        responder_credential: &DeviceCredential,
        responder_nonce: [u8; 32],
        negotiated_protocol: ProtocolVersion,
        negotiated_features: &[u16],
        channel_binding_profile: &[u8],
        channel_binding_value: &[u8],
    ) -> Result<Self, SessionAuthError> {
        if initiator_credential.owner_id() != owner_id
            || responder_credential.owner_id() != owner_id
        {
            return Err(SessionAuthError::WrongOwner);
        }
        if negotiated_features.len() > MAX_SUPPORTED_FEATURES {
            return Err(SessionAuthError::TooManyFeatures);
        }

        let mut features = negotiated_features.to_vec();
        features.sort_unstable();
        features.dedup();

        Ok(Self {
            owner_id,
            initiator_device_id: initiator_credential.device_id(),
            initiator_device_key_id: initiator_credential.device_key_id(),
            initiator_credential_signed_object_digest: credential_signed_object_digest(
                initiator_credential,
            ),
            initiator_nonce,
            responder_device_id: responder_credential.device_id(),
            responder_device_key_id: responder_credential.device_key_id(),
            responder_credential_signed_object_digest: credential_signed_object_digest(
                responder_credential,
            ),
            responder_nonce,
            negotiated_protocol,
            negotiated_feature_set_digest: negotiated_feature_set_digest(&features),
            channel_binding_profile_digest: digest_labeled(
                CHANNEL_BINDING_PROFILE_DOMAIN,
                channel_binding_profile,
            ),
            channel_binding_value_digest: digest_labeled(
                CHANNEL_BINDING_VALUE_DOMAIN,
                channel_binding_value,
            ),
        })
    }

    pub fn canonical_bytes(&self) -> Vec<u8> {
        self.canonical_transcript().encode()
    }

    pub fn digest(&self) -> [u8; 32] {
        self.canonical_transcript().digest()
    }

    pub fn create_proof(
        &self,
        role: SessionAuthRole,
        signing_key: &SigningKey,
    ) -> Result<SessionAuthProof, SessionAuthError> {
        let public_key = signing_key.verifying_key();
        if KeyId::derive(SignatureAlgorithm::Ed25519, &public_key) != self.key_id(role) {
            return Err(SessionAuthError::WrongDeviceKey);
        }

        let transcript_digest = self.digest();
        let input = proof_input(role, transcript_digest);
        Ok(SessionAuthProof {
            role,
            transcript_digest,
            signature: signing_key.sign_message(&input),
        })
    }

    pub fn verify_proof(
        &self,
        proof: &SessionAuthProof,
        expected_role: SessionAuthRole,
        public_key: VerifyingKey,
    ) -> Result<(), SessionAuthError> {
        if proof.role != expected_role {
            return Err(SessionAuthError::WrongProofRole);
        }
        let transcript_digest = self.digest();
        if proof.transcript_digest != transcript_digest {
            return Err(SessionAuthError::WrongProofTranscript);
        }
        if KeyId::derive(SignatureAlgorithm::Ed25519, &public_key) != self.key_id(expected_role) {
            return Err(SessionAuthError::WrongDeviceKey);
        }

        public_key
            .verify_message(
                &proof_input(expected_role, transcript_digest),
                &proof.signature,
            )
            .map_err(|_| SessionAuthError::InvalidProof)
    }

    pub fn derive_session_id(
        &self,
        initiator_proof: &SessionAuthProof,
        initiator_public_key: VerifyingKey,
        responder_proof: &SessionAuthProof,
        responder_public_key: VerifyingKey,
    ) -> Result<SessionId, SessionAuthError> {
        self.verify_proof(
            initiator_proof,
            SessionAuthRole::Initiator,
            initiator_public_key,
        )?;
        self.verify_proof(
            responder_proof,
            SessionAuthRole::Responder,
            responder_public_key,
        )?;

        let transcript_digest = self.digest();
        let initiator_signature = initiator_proof.signature.to_bytes();
        let responder_signature = responder_proof.signature.to_bytes();
        let mut input = Vec::with_capacity(SESSION_ID_DOMAIN.len() + 32 + 64 + 64);
        input.extend_from_slice(SESSION_ID_DOMAIN);
        input.extend_from_slice(&transcript_digest);
        input.extend_from_slice(&initiator_signature);
        input.extend_from_slice(&responder_signature);
        Ok(SessionId::from_bytes(blake3_256(&input)))
    }

    pub const fn owner_id(&self) -> OwnerId {
        self.owner_id
    }

    pub const fn initiator_device_id(&self) -> DeviceId {
        self.initiator_device_id
    }

    pub const fn responder_device_id(&self) -> DeviceId {
        self.responder_device_id
    }

    pub const fn negotiated_protocol(&self) -> ProtocolVersion {
        self.negotiated_protocol
    }

    pub const fn negotiated_feature_set_digest(&self) -> [u8; 32] {
        self.negotiated_feature_set_digest
    }

    fn key_id(&self, role: SessionAuthRole) -> KeyId {
        match role {
            SessionAuthRole::Initiator => self.initiator_device_key_id,
            SessionAuthRole::Responder => self.responder_device_key_id,
        }
    }

    fn canonical_transcript(&self) -> CanonicalTranscript {
        let mut transcript =
            CanonicalTranscript::new(DOMAIN).expect("static canonical domain is valid");
        transcript
            .push(1, 1_u16.to_be_bytes())
            .expect("fixed field is valid");
        transcript
            .push(2, self.owner_id.to_bytes())
            .expect("fixed field is valid");
        transcript
            .push(3, self.initiator_device_id.to_bytes())
            .expect("fixed field is valid");
        transcript
            .push(4, self.initiator_device_key_id.to_bytes())
            .expect("fixed field is valid");
        transcript
            .push(5, self.initiator_credential_signed_object_digest)
            .expect("fixed field is valid");
        transcript
            .push(6, self.initiator_nonce)
            .expect("fixed field is valid");
        transcript
            .push(7, self.responder_device_id.to_bytes())
            .expect("fixed field is valid");
        transcript
            .push(8, self.responder_device_key_id.to_bytes())
            .expect("fixed field is valid");
        transcript
            .push(9, self.responder_credential_signed_object_digest)
            .expect("fixed field is valid");
        transcript
            .push(10, self.responder_nonce)
            .expect("fixed field is valid");
        transcript
            .push(11, self.negotiated_protocol.major().to_be_bytes())
            .expect("fixed field is valid");
        transcript
            .push(12, self.negotiated_protocol.minor().to_be_bytes())
            .expect("fixed field is valid");
        transcript
            .push(13, self.negotiated_feature_set_digest)
            .expect("fixed field is valid");
        transcript
            .push(14, self.channel_binding_profile_digest)
            .expect("fixed field is valid");
        transcript
            .push(15, self.channel_binding_value_digest)
            .expect("fixed field is valid");
        transcript
    }
}

fn credential_signed_object_digest(credential: &DeviceCredential) -> [u8; 32] {
    signed_object_digest(
        credential.transcript_digest(),
        SignatureAlgorithm::Ed25519,
        &credential.signature(),
    )
}

fn negotiated_feature_set_digest(features: &[u16]) -> [u8; 32] {
    let mut input = Vec::with_capacity(FEATURE_SET_DOMAIN.len() + features.len() * 2);
    input.extend_from_slice(FEATURE_SET_DOMAIN);
    for feature in features {
        input.extend_from_slice(&feature.to_be_bytes());
    }
    blake3_256(&input)
}

fn digest_labeled(label: &[u8], value: &[u8]) -> [u8; 32] {
    let mut input = Vec::with_capacity(label.len() + value.len());
    input.extend_from_slice(label);
    input.extend_from_slice(value);
    blake3_256(&input)
}

fn proof_input(role: SessionAuthRole, transcript_digest: [u8; 32]) -> Vec<u8> {
    let label = match role {
        SessionAuthRole::Initiator => INITIATOR_PROOF_LABEL,
        SessionAuthRole::Responder => RESPONDER_PROOF_LABEL,
    };
    let mut input = Vec::with_capacity(label.len() + transcript_digest.len());
    input.extend_from_slice(label);
    input.extend_from_slice(&transcript_digest);
    input
}
