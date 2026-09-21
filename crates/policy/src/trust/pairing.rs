use core::fmt;

use crosslab_crypto::{
    CanonicalTranscript, Signature, SignatureAlgorithm, SigningKey, SigningProvider,
    signed_object_digest,
};
use crosslab_identity::{
    AuthorityRole, DeviceCredential, DeviceId, IdentityError, KeyId, OwnerAuthorityState, OwnerId,
};

use super::{TransitionId, TrustRecord};

const DOMAIN: &str = "crosslab.pairing-trust-transition.v1";
const INITIAL_CREDENTIAL_EPOCH: u64 = 0;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PairingTrustTransitionError {
    UnsupportedSchema,
    Identity(IdentityError),
    CredentialMismatch,
    NonInitialCredentialEpoch,
    UnknownIssuer,
    InvalidSignature,
    SigningFailed,
}

impl fmt::Display for PairingTrustTransitionError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnsupportedSchema => {
                formatter.write_str("pairing trust transition schema is unsupported")
            }
            Self::Identity(error) => fmt::Display::fmt(error, formatter),
            Self::CredentialMismatch => {
                formatter.write_str("pairing trust transition credential does not match")
            }
            Self::NonInitialCredentialEpoch => formatter
                .write_str("pairing trust transition requires the initial credential epoch"),
            Self::UnknownIssuer => {
                formatter.write_str("pairing trust transition issuer is unknown")
            }
            Self::InvalidSignature => {
                formatter.write_str("pairing trust transition signature is invalid")
            }
            Self::SigningFailed => {
                formatter.write_str("pairing trust transition signing provider failed")
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
    pub fn from_unverified_signed_parts(
        schema_version: u16,
        owner_id: OwnerId,
        device_id: DeviceId,
        credential_epoch: u64,
        credential_signed_object_digest: [u8; 32],
        transition_id: TransitionId,
        pairing_evidence_digest: [u8; 32],
        issuer_key_id: KeyId,
        signature: Signature,
    ) -> Self {
        Self {
            schema_version,
            owner_id,
            device_id,
            credential_epoch,
            credential_signed_object_digest,
            transition_id,
            pairing_evidence_digest,
            issuer_key_id,
            signature,
        }
    }

    pub fn issue(
        credential: &DeviceCredential,
        transition_id: TransitionId,
        pairing_evidence_digest: [u8; 32],
        authority: &OwnerAuthorityState,
        issuer_key: &SigningKey,
    ) -> Result<Self, PairingTrustTransitionError> {
        Self::issue_with_provider(
            credential,
            transition_id,
            pairing_evidence_digest,
            authority,
            issuer_key,
        )
    }

    pub fn issue_with_provider(
        credential: &DeviceCredential,
        transition_id: TransitionId,
        pairing_evidence_digest: [u8; 32],
        authority: &OwnerAuthorityState,
        issuer_key: &dyn SigningProvider,
    ) -> Result<Self, PairingTrustTransitionError> {
        if credential.credential_epoch() != INITIAL_CREDENTIAL_EPOCH {
            return Err(PairingTrustTransitionError::NonInitialCredentialEpoch);
        }
        credential.verify(authority, INITIAL_CREDENTIAL_EPOCH)?;
        let issuer = authority.current_delegation(AuthorityRole::DeviceSigning)?;
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
        transition.signature = issuer_key
            .sign_digest(&transition.transcript_digest())
            .map_err(|_| PairingTrustTransitionError::SigningFailed)?;
        Ok(transition)
    }

    pub fn establish(
        &self,
        credential: &DeviceCredential,
        authority: &OwnerAuthorityState,
    ) -> Result<TrustRecord, PairingTrustTransitionError> {
        if self.schema_version != 1 {
            return Err(PairingTrustTransitionError::UnsupportedSchema);
        }
        if self.credential_epoch != INITIAL_CREDENTIAL_EPOCH
            || credential.credential_epoch() != INITIAL_CREDENTIAL_EPOCH
        {
            return Err(PairingTrustTransitionError::NonInitialCredentialEpoch);
        }
        credential.verify(authority, INITIAL_CREDENTIAL_EPOCH)?;
        if credential.owner_id() != self.owner_id
            || credential.device_id() != self.device_id
            || credential_signed_object_digest(credential) != self.credential_signed_object_digest
        {
            return Err(PairingTrustTransitionError::CredentialMismatch);
        }

        let issuer = authority.current_delegation(AuthorityRole::DeviceSigning)?;
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

    pub const fn schema_version(&self) -> u16 {
        self.schema_version
    }

    pub const fn owner_id(&self) -> OwnerId {
        self.owner_id
    }

    pub const fn device_id(&self) -> DeviceId {
        self.device_id
    }

    pub const fn credential_epoch(&self) -> u64 {
        self.credential_epoch
    }

    pub const fn credential_signed_object_digest(&self) -> [u8; 32] {
        self.credential_signed_object_digest
    }

    pub const fn transition_id(&self) -> TransitionId {
        self.transition_id
    }

    pub const fn pairing_evidence_digest(&self) -> [u8; 32] {
        self.pairing_evidence_digest
    }

    pub const fn issuer_key_id(&self) -> KeyId {
        self.issuer_key_id
    }

    pub const fn signature(&self) -> Signature {
        self.signature
    }
}

fn credential_signed_object_digest(credential: &DeviceCredential) -> [u8; 32] {
    signed_object_digest(
        credential.transcript_digest(),
        SignatureAlgorithm::Ed25519,
        &credential.signature(),
    )
}
