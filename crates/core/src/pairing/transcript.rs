use crosslab_crypto::{
    CanonicalTranscript, SignatureAlgorithm, VerifyingKey, hmac_sha256, verify_hmac_sha256,
};
use crosslab_identity::{DeviceId, OwnerId};

use super::{PairingConfirmationError, PairingConfirmationRole, PairingId, PairingSecret};

const PAIRING_PROFILE_V1: u16 = 1;
const PAIRING_TRANSCRIPT_DOMAIN: &str = "crosslab.pairing-transcript.v1";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PairingTranscript {
    protocol_major: u16,
    pairing_id: PairingId,
    owner_id: OwnerId,
    inviter_device_id: DeviceId,
    inviter_device_key: VerifyingKey,
    inviter_nonce: [u8; 32],
    joiner_device_id: DeviceId,
    joiner_device_key: VerifyingKey,
    joiner_nonce: [u8; 32],
}

impl PairingTranscript {
    #[allow(clippy::too_many_arguments)]
    pub const fn new(
        protocol_major: u16,
        pairing_id: PairingId,
        owner_id: OwnerId,
        inviter_device_id: DeviceId,
        inviter_device_key: VerifyingKey,
        inviter_nonce: [u8; 32],
        joiner_device_id: DeviceId,
        joiner_device_key: VerifyingKey,
        joiner_nonce: [u8; 32],
    ) -> Self {
        Self {
            protocol_major,
            pairing_id,
            owner_id,
            inviter_device_id,
            inviter_device_key,
            inviter_nonce,
            joiner_device_id,
            joiner_device_key,
            joiner_nonce,
        }
    }

    pub fn digest(&self) -> [u8; 32] {
        let mut transcript = CanonicalTranscript::new(PAIRING_TRANSCRIPT_DOMAIN)
            .expect("fixed pairing transcript domain is valid");
        transcript
            .push(1, PAIRING_PROFILE_V1.to_be_bytes())
            .expect("fixed pairing profile field is valid");
        transcript
            .push(2, self.protocol_major.to_be_bytes())
            .expect("protocol major field is valid");
        transcript
            .push(3, self.pairing_id.as_bytes())
            .expect("pairing id field is valid");
        transcript
            .push(4, self.owner_id.as_bytes())
            .expect("owner id field is valid");
        transcript
            .push(5, self.inviter_device_id.as_bytes())
            .expect("inviter device id field is valid");
        transcript
            .push(6, SignatureAlgorithm::Ed25519.code().to_be_bytes())
            .expect("inviter device algorithm field is valid");
        transcript
            .push(7, self.inviter_device_key.as_bytes())
            .expect("inviter public key field is valid");
        transcript
            .push(8, self.inviter_nonce)
            .expect("inviter nonce field is valid");
        transcript
            .push(9, self.joiner_device_id.as_bytes())
            .expect("joiner device id field is valid");
        transcript
            .push(10, SignatureAlgorithm::Ed25519.code().to_be_bytes())
            .expect("joiner device algorithm field is valid");
        transcript
            .push(11, self.joiner_device_key.as_bytes())
            .expect("joiner public key field is valid");
        transcript
            .push(12, self.joiner_nonce)
            .expect("joiner nonce field is valid");
        transcript.digest()
    }

    pub fn confirmation(
        &self,
        role: PairingConfirmationRole,
        secret: &PairingSecret,
    ) -> [u8; 32] {
        let message = self.confirmation_message(role);
        hmac_sha256(secret.as_bytes(), &message)
    }

    pub fn verify_confirmation(
        &self,
        role: PairingConfirmationRole,
        secret: &PairingSecret,
        expected: &[u8; 32],
    ) -> Result<(), PairingConfirmationError> {
        let message = self.confirmation_message(role);
        verify_hmac_sha256(secret.as_bytes(), &message, expected)
            .map_err(|_| PairingConfirmationError)
    }

    fn confirmation_message(&self, role: PairingConfirmationRole) -> Vec<u8> {
        let digest = self.digest();
        let label = role.label();
        let mut message = Vec::with_capacity(label.len() + digest.len());
        message.extend_from_slice(label);
        message.extend_from_slice(&digest);
        message
    }
}
