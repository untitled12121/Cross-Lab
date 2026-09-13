use core::fmt;

use crosslab_crypto::{
    CanonicalTranscript, Signature, SignatureAlgorithm, SigningKey, signed_object_digest,
};
use crosslab_identity::{
    AuthorityDelegation, DeviceCredential, DeviceId, IdentityError, KeyId, OwnerId, OwnerRootRecord,
};

use super::{TransitionId, TrustRecord};

const DOMAIN: &str = "crosslab.pairing-trust-transition.v1";
const INITIAL_CREDENTIAL_EPOCH: u64 = 0;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PairingTrustTransitionError {
    Identity(IdentityError),
    CredentialMismatch,
    NonInitialCredentialEpoch,
    UnknownIssuer,
    InvalidSignature,
}

impl fmt::Display for PairingTrustTransitionError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Identity(error) => fmt::Display::fmt(error, formatter),
            Self::CredentialMismatch => {
                formatter.write_str("pairing trust transition credential does not match")
            }
            Self::NonInitialCredentialEpoch => {
                formatter.write_str("pairing trust transition requires the initial credential epoch")
            }
            Self::UnknownIssuer => {
                formatter.write_str("pairing trust transition issuer is unknown")
            }
            Self::InvalidSignature => {
                formatter.write_str("pairing trust transition signature is invalid")
            }
        }
    }
}

impl std::error::Error for PairingTrustTransitionError {}

impl From<IdentityError> for PairingTrustTransitionError {
    fn from(error: IdentityError) -> Self {
        Self::Identity(error)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PairingTrustTransition {
    schema_version: u16,
    owner_id: OwnerId,
    device_id: DeviceId,
    credential_epoch: u64,
    credential_signed_object_digest: [u8; 32],
    transition_id: TransitionId,
    pairing_evidence_digest: [u8; 32],
    issuer_key_id: KeyId,
    signature: Signature,
}

impl PairingTrustTransition {
    #[allow(clippy::too_many_arguments)]
    pub fn issue(
        credential: &DeviceCredential,
        transition_id: TransitionId,
        pairing_evidence_digest: [u8; 32],
        root: &OwnerRootRecord,
        issuer: &AuthorityDelegation,
        issuer_key: &SigningKey,
        minimum_delegation_epoch: u64,
    ) -> Result<Self, PairingTrustTransitionError> {
        if credential.credential_epoch() != INITIAL_CREDENTIAL_EPOCH {
            return Err(PairingTrustTransitionError::NonInitialCredentialEpoch);
        }
        credential.verify(
            root,
            issuer,
            INITIAL_CREDENTIAL_EPOCH,
            minimum_delegation_epoch,
        )?;
        if issuer_key.verifying_key() != issuer.delegated_public_key() {
            return Err(PairingTrustTransitionError::UnknownIssuer);
        }

        let mut transition = Self {
            schema_version: 1,
            owner_id: credential.owner_id(),
            device_id: credential.device_id(),
            credential_epoch: INITIAL_CREDENTIAL_EPOCH,
            credential_signed_object_digest: credential_signed_object_digest(credential),
            transition_id,
            pairing_evidence_digest,
            issuer_key_id: issuer.delegated_key_id(),
            signature: Signature::from_bytes([0; 64]),
        };
        transition.signature = issuer_key.sign_digest(&transition.transcript_digest());
        Ok(transition)
    }

    pub fn establish(
        &self,
        credential: &DeviceCredential,
        root: &OwnerRootRecord,
        issuer: &AuthorityDelegation,
        minimum_delegation_epoch: u64,
    ) -> Result<TrustRecord, PairingTrustTransitionError> {
        if self.credential_epoch != INITIAL_CREDENTIAL_EPOCH
            || credential.credential_epoch() != INITIAL_CREDENTIAL_EPOCH
        {
            return Err(PairingTrustTransitionError::NonInitialCredentialEpoch);
        }
        credential.verify(
            root,
            issuer,
            INITIAL_CREDENTIAL_EPOCH,
            minimum_delegation_epoch,
        )?;
        if credential.owner_id() != self.owner_id
            || credential.device_id() != self.device_id
            || credential_signed_object_digest(credential) != self.credential_signed_object_digest
        {
            return Err(PairingTrustTransitionError::CredentialMismatch);
        }
        if issuer.delegated_key_id() != self.issuer_key_id {
            return Err(PairingTrustTransitionError::UnknownIssuer);
        }
        issuer
            .delegated_public_key()
            .verify_digest(&self.transcript_digest(), &self.signature)
            .map_err(|_| PairingTrustTransitionError::InvalidSignature)?;

        Ok(TrustRecord::established_from_pairing(
            self.owner_id,
            self.device_id,
            INITIAL_CREDENTIAL_EPOCH,
            self.transition_id,
        ))
    }

    pub fn transcript_digest(&self) -> [u8; 32] {
        let mut transcript =
            CanonicalTranscript::new(DOMAIN).expect("static canonical domain is valid");
        transcript
            .push(1, self.schema_version.to_be_bytes())
            .expect("fixed field is valid");
        transcript
            .push(2, self.owner_id.to_bytes())
            .expect("fixed field is valid");
        transcript
            .push(3, self.device_id.to_bytes())
            .expect("fixed field is valid");
        transcript
            .push(4, self.credential_epoch.to_be_bytes())
            .expect("fixed field is valid");
        transcript
            .push(5, self.credential_signed_object_digest)
            .expect("fixed field is valid");
        transcript
            .push(6, self.transition_id.to_bytes())
            .expect("fixed field is valid");
        transcript
            .push(7, self.pairing_evidence_digest)
            .expect("fixed field is valid");
        transcript
            .push(8, self.issuer_key_id.to_bytes())
            .expect("fixed field is valid");
        transcript.digest()
    }

    pub const fn pairing_evidence_digest(&self) -> [u8; 32] {
        self.pairing_evidence_digest
    }
}

fn credential_signed_object_digest(credential: &DeviceCredential) -> [u8; 32] {
    signed_object_digest(
        credential.transcript_digest(),
        SignatureAlgorithm::Ed25519,
        &credential.signature(),
    )
}
