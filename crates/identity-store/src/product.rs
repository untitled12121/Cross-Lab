use core::fmt;

use crosslab_crypto::{Signature, SignatureAlgorithm, SigningProvider, VerifyingKey};
use crosslab_identity::{
    AuthorityDelegation, AuthorityRole, DeviceCredential, DeviceId, IdentityError, KeyId,
    OwnerAuthorityState, OwnerId, OwnerRootRecord,
};
use crosslab_policy::{
    PairingTrustTransition, PairingTrustTransitionError, TransitionId, TrustRecord,
};

const MAGIC: &[u8; 8] = b"CLPIDV1\0";
const LEGACY_SCHEMA_VERSION: u16 = 1;
const SCHEMA_VERSION: u16 = 2;
const IDENTITY_OBJECT_SCHEMA_V1: u16 = 1;
const BASE_ENCODED_LEN: usize = MAGIC.len() + 2 + 32 + 32 + 8 + 32 + 8 + 32 + 64 + 8 + 32 + 64;
const CREDENTIAL_ENCODED_LEN: usize = 2 + 32 + 32 + 32 + 32 + 8 + 32 + 64;
const TRANSITION_ENCODED_LEN: usize = 2 + 32 + 32 + 8 + 32 + 32 + 32 + 32 + 64;
const PEER_ENCODED_LEN: usize = CREDENTIAL_ENCODED_LEN + TRANSITION_ENCODED_LEN;
pub const MAX_PRODUCT_TRUSTED_PEERS: usize = 1024;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ProductTrustedPeer {
    credential: DeviceCredential,
    transition: PairingTrustTransition,
}

impl ProductTrustedPeer {
    pub const fn credential(&self) -> DeviceCredential {
        self.credential
    }

    pub const fn transition(&self) -> PairingTrustTransition {
        self.transition
    }

    pub fn trust(
        &self,
        authority: &OwnerAuthorityState,
    ) -> Result<TrustRecord, ProductIdentityError> {
        self.transition
            .establish(&self.credential, authority)
            .map_err(Into::into)
    }

    fn validate(
        &self,
        owner_id: OwnerId,
        local_device_id: DeviceId,
        authority: &OwnerAuthorityState,
    ) -> Result<(), ProductIdentityError> {
        if self.credential.owner_id() != owner_id {
            return Err(ProductIdentityError::PeerOwnerMismatch);
        }
        if self.credential.device_id() == local_device_id {
            return Err(ProductIdentityError::LocalDeviceAsPeer);
        }
        self.trust(authority)?;
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProductIdentityState {
    owner_id: OwnerId,
    local_device_id: DeviceId,
    root: OwnerRootRecord,
    device_signing: AuthorityDelegation,
    local_credential: DeviceCredential,
    trusted_peers: Vec<ProductTrustedPeer>,
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
            trusted_peers: Vec::new(),
        })
    }

    pub fn join_owner_domain(
        root: OwnerRootRecord,
        device_signing: AuthorityDelegation,
        local_credential: DeviceCredential,
        local_device_signer: &dyn SigningProvider,
    ) -> Result<Self, ProductIdentityError> {
        let mut authority = OwnerAuthorityState::new(root);
        authority.accept_delegation(device_signing)?;
        local_credential.verify(&authority, local_credential.credential_epoch())?;

        if local_device_signer.verifying_key() != local_credential.device_public_key() {
            return Err(ProductIdentityError::ProviderMismatch);
        }

        Ok(Self {
            owner_id: root.owner_id(),
            local_device_id: local_credential.device_id(),
            root,
            device_signing,
            local_credential,
            trusted_peers: Vec::new(),
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

    pub fn trusted_peers(&self) -> &[ProductTrustedPeer] {
        &self.trusted_peers
    }

    pub fn trusted_peer(&self, device_id: DeviceId) -> Option<&ProductTrustedPeer> {
        self.trusted_peers
            .iter()
            .find(|peer| peer.credential.device_id() == device_id)
    }

    pub fn authority_state(&self) -> Result<OwnerAuthorityState, ProductIdentityError> {
        let mut authority = OwnerAuthorityState::new(self.root);
        authority.accept_delegation(self.device_signing)?;
        Ok(authority)
    }

    pub fn with_paired_peer(
        &self,
        credential: DeviceCredential,
        transition: PairingTrustTransition,
    ) -> Result<Self, ProductIdentityError> {
        let mut next = self.clone();
        next.add_paired_peer(credential, transition)?;
        Ok(next)
    }

    pub fn add_paired_peer(
        &mut self,
        credential: DeviceCredential,
        transition: PairingTrustTransition,
    ) -> Result<(), ProductIdentityError> {
        if self.trusted_peers.len() >= MAX_PRODUCT_TRUSTED_PEERS {
            return Err(ProductIdentityError::PeerLimit);
        }
        if self
            .trusted_peers
            .iter()
            .any(|peer| peer.credential.device_id() == credential.device_id())
        {
            return Err(ProductIdentityError::DuplicatePeer);
        }

        let authority = self.authority_state()?;
        let peer = ProductTrustedPeer {
            credential,
            transition,
        };
        peer.validate(self.owner_id, self.local_device_id, &authority)?;
        self.trusted_peers.push(peer);
        Ok(())
    }

    pub fn validate_local_device_provider(
        &self,
        local_device_signer: &dyn SigningProvider,
    ) -> Result<(), ProductIdentityError> {
        if local_device_signer.verifying_key() != self.local_credential.device_public_key() {
            return Err(ProductIdentityError::ProviderMismatch);
        }

        let authority = self.authority_state()?;
        self.local_credential
            .verify(&authority, self.local_credential.credential_epoch())?;
        self.validate_peers(&authority)
    }

    pub fn validate_providers(
        &self,
        root_signer: &dyn SigningProvider,
        device_signing_signer: &dyn SigningProvider,
        local_device_signer: &dyn SigningProvider,
    ) -> Result<(), ProductIdentityError> {
        if root_signer.verifying_key() != self.root.root_public_key()
            || device_signing_signer.verifying_key() != self.device_signing.delegated_public_key()
        {
            return Err(ProductIdentityError::ProviderMismatch);
        }

        self.validate_local_device_provider(local_device_signer)
    }

    pub fn encode(&self) -> Vec<u8> {
        let mut output =
            Vec::with_capacity(BASE_ENCODED_LEN + 4 + self.trusted_peers.len() * PEER_ENCODED_LEN);
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
        let peer_count =
            u32::try_from(self.trusted_peers.len()).expect("trusted peer limit fits u32");
        output.extend_from_slice(&peer_count.to_be_bytes());
        for peer in &self.trusted_peers {
            encode_peer(&mut output, peer);
        }
        output
    }

    pub fn decode(encoded: &[u8]) -> Result<Self, ProductIdentityError> {
        if encoded.len() < BASE_ENCODED_LEN || &encoded[..MAGIC.len()] != MAGIC {
            return Err(ProductIdentityError::Malformed);
        }

        let mut offset = MAGIC.len();
        let schema = read_u16(encoded, &mut offset)?;
        if schema != LEGACY_SCHEMA_VERSION && schema != SCHEMA_VERSION {
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
            IDENTITY_OBJECT_SCHEMA_V1,
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
            IDENTITY_OBJECT_SCHEMA_V1,
            owner_id,
            local_device_id,
            KeyId::derive(SignatureAlgorithm::Ed25519, &device_key),
            SignatureAlgorithm::Ed25519,
            device_key,
            credential_epoch,
            device_signing.delegated_key_id(),
            credential_signature,
        )?;

        let trusted_peers = if schema == LEGACY_SCHEMA_VERSION {
            if offset != encoded.len() {
                return Err(ProductIdentityError::Malformed);
            }
            Vec::new()
        } else {
            decode_peers(encoded, &mut offset)?
        };
        if offset != encoded.len() {
            return Err(ProductIdentityError::Malformed);
        }

        let state = Self {
            owner_id,
            local_device_id,
            root,
            device_signing,
            local_credential,
            trusted_peers,
        };
        let authority = state.authority_state()?;
        state
            .local_credential
            .verify(&authority, credential_epoch)?;
        state.validate_peers(&authority)?;
        Ok(state)
    }

    fn validate_peers(&self, authority: &OwnerAuthorityState) -> Result<(), ProductIdentityError> {
        if self.trusted_peers.len() > MAX_PRODUCT_TRUSTED_PEERS {
            return Err(ProductIdentityError::PeerLimit);
        }

        for (index, peer) in self.trusted_peers.iter().enumerate() {
            peer.validate(self.owner_id, self.local_device_id, authority)?;
            if self.trusted_peers[..index]
                .iter()
                .any(|existing| existing.credential.device_id() == peer.credential.device_id())
            {
                return Err(ProductIdentityError::DuplicatePeer);
            }
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProductIdentityError {
    Random,
    Malformed,
    UnsupportedSchema,
    ProviderMismatch,
    PeerLimit,
    PeerOwnerMismatch,
    LocalDeviceAsPeer,
    DuplicatePeer,
    Identity(IdentityError),
    Trust(PairingTrustTransitionError),
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
            Self::PeerLimit => {
                formatter.write_str("product identity trusted-peer limit is exceeded")
            }
            Self::PeerOwnerMismatch => {
                formatter.write_str("persisted trusted peer belongs to a different owner")
            }
            Self::LocalDeviceAsPeer => {
                formatter.write_str("local device cannot be persisted as its own trusted peer")
            }
            Self::DuplicatePeer => {
                formatter.write_str("trusted peer is already present in product identity")
            }
            Self::Identity(error) => fmt::Display::fmt(error, formatter),
            Self::Trust(error) => fmt::Display::fmt(error, formatter),
        }
    }
}

impl std::error::Error for ProductIdentityError {}

impl From<IdentityError> for ProductIdentityError {
    fn from(error: IdentityError) -> Self {
        Self::Identity(error)
    }
}

impl From<PairingTrustTransitionError> for ProductIdentityError {
    fn from(error: PairingTrustTransitionError) -> Self {
        Self::Trust(error)
    }
}

fn encode_peer(output: &mut Vec<u8>, peer: &ProductTrustedPeer) {
    let credential = peer.credential;
    output.extend_from_slice(&credential.schema_version().to_be_bytes());
    output.extend_from_slice(credential.owner_id().as_bytes());
    output.extend_from_slice(credential.device_id().as_bytes());
    output.extend_from_slice(credential.device_key_id().as_bytes());
    output.extend_from_slice(credential.device_public_key().as_bytes());
    output.extend_from_slice(&credential.credential_epoch().to_be_bytes());
    output.extend_from_slice(credential.issuer_device_signing_key_id().as_bytes());
    output.extend_from_slice(&credential.signature().to_bytes());

    let transition = peer.transition;
    output.extend_from_slice(&transition.schema_version().to_be_bytes());
    output.extend_from_slice(transition.owner_id().as_bytes());
    output.extend_from_slice(transition.device_id().as_bytes());
    output.extend_from_slice(&transition.credential_epoch().to_be_bytes());
    output.extend_from_slice(&transition.credential_signed_object_digest());
    output.extend_from_slice(&transition.transition_id().to_bytes());
    output.extend_from_slice(&transition.pairing_evidence_digest());
    output.extend_from_slice(transition.issuer_key_id().as_bytes());
    output.extend_from_slice(&transition.signature().to_bytes());
}

fn decode_peers(
    encoded: &[u8],
    offset: &mut usize,
) -> Result<Vec<ProductTrustedPeer>, ProductIdentityError> {
    let count =
        usize::try_from(read_u32(encoded, offset)?).map_err(|_| ProductIdentityError::Malformed)?;
    if count > MAX_PRODUCT_TRUSTED_PEERS {
        return Err(ProductIdentityError::PeerLimit);
    }
    let peer_bytes = count
        .checked_mul(PEER_ENCODED_LEN)
        .ok_or(ProductIdentityError::Malformed)?;
    if encoded.len().saturating_sub(*offset) != peer_bytes {
        return Err(ProductIdentityError::Malformed);
    }

    let mut peers = Vec::with_capacity(count);
    for _ in 0..count {
        peers.push(decode_peer(encoded, offset)?);
    }
    Ok(peers)
}

fn decode_peer(
    encoded: &[u8],
    offset: &mut usize,
) -> Result<ProductTrustedPeer, ProductIdentityError> {
    let credential_schema = read_u16(encoded, offset)?;
    let owner_id = OwnerId::from_bytes(read_array(encoded, offset)?);
    let device_id = DeviceId::from_bytes(read_array(encoded, offset)?);
    let device_key_id = KeyId::from_bytes(read_array(encoded, offset)?);
    let device_public_key = read_verifying_key(encoded, offset)?;
    let credential_epoch = read_u64(encoded, offset)?;
    let issuer_key_id = KeyId::from_bytes(read_array(encoded, offset)?);
    let credential_signature = Signature::from_bytes(read_array(encoded, offset)?);
    let credential = DeviceCredential::from_unverified_signed_parts(
        credential_schema,
        owner_id,
        device_id,
        device_key_id,
        SignatureAlgorithm::Ed25519,
        device_public_key,
        credential_epoch,
        issuer_key_id,
        credential_signature,
    )?;

    let transition_schema = read_u16(encoded, offset)?;
    let transition_owner = OwnerId::from_bytes(read_array(encoded, offset)?);
    let transition_device = DeviceId::from_bytes(read_array(encoded, offset)?);
    let transition_credential_epoch = read_u64(encoded, offset)?;
    let credential_digest = read_array(encoded, offset)?;
    let transition_id = TransitionId::from_bytes(read_array(encoded, offset)?);
    let pairing_evidence_digest = read_array(encoded, offset)?;
    let transition_issuer_key_id = KeyId::from_bytes(read_array(encoded, offset)?);
    let transition_signature = Signature::from_bytes(read_array(encoded, offset)?);
    let transition = PairingTrustTransition::from_unverified_signed_parts(
        transition_schema,
        transition_owner,
        transition_device,
        transition_credential_epoch,
        credential_digest,
        transition_id,
        pairing_evidence_digest,
        transition_issuer_key_id,
        transition_signature,
    );

    Ok(ProductTrustedPeer {
        credential,
        transition,
    })
}

fn read_u16(bytes: &[u8], offset: &mut usize) -> Result<u16, ProductIdentityError> {
    Ok(u16::from_be_bytes(read_array(bytes, offset)?))
}

fn read_u32(bytes: &[u8], offset: &mut usize) -> Result<u32, ProductIdentityError> {
    Ok(u32::from_be_bytes(read_array(bytes, offset)?))
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
    use crosslab_policy::{PairingTrustTransition, TransitionId, TrustState};

    use super::*;

    fn fixture() -> (ProductIdentityState, SigningKey, SigningKey, SigningKey) {
        let root = SigningKey::generate().unwrap();
        let issuer = SigningKey::generate().unwrap();
        let device = SigningKey::generate().unwrap();
        let state = ProductIdentityState::bootstrap(&root, &issuer, &device).unwrap();
        (state, root, issuer, device)
    }

    fn add_peer(
        state: &mut ProductIdentityState,
        issuer: &SigningKey,
        seed: u8,
    ) -> DeviceCredential {
        let authority = state.authority_state().unwrap();
        let peer_key = SigningKey::from_secret_bytes([seed; 32]);
        let credential = DeviceCredential::issue(
            state.owner_id(),
            DeviceId::from_bytes([seed.wrapping_add(1); 32]),
            &peer_key,
            0,
            &authority,
            issuer,
        )
        .unwrap();
        let transition = PairingTrustTransition::issue(
            &credential,
            TransitionId::from_bytes([seed.wrapping_add(2); 32]),
            [seed.wrapping_add(3); 32],
            &authority,
            issuer,
        )
        .unwrap();
        state.add_paired_peer(credential, transition).unwrap();
        credential
    }

    #[test]
    fn product_identity_round_trips_and_validates_provider_binding() {
        let (state, root, issuer, device) = fixture();

        let decoded = ProductIdentityState::decode(&state.encode()).unwrap();
        decoded.validate_providers(&root, &issuer, &device).unwrap();
        assert_eq!(decoded.owner_id(), state.owner_id());
        assert_eq!(decoded.local_device_id(), state.local_device_id());
        assert!(decoded.trusted_peers().is_empty());
    }

    #[test]
    fn legacy_product_identity_decodes_without_trusted_peers() {
        let (state, root, issuer, device) = fixture();
        let mut legacy = state.encode();
        legacy[MAGIC.len()..MAGIC.len() + 2].copy_from_slice(&LEGACY_SCHEMA_VERSION.to_be_bytes());
        legacy.truncate(BASE_ENCODED_LEN);

        let decoded = ProductIdentityState::decode(&legacy).unwrap();
        decoded.validate_providers(&root, &issuer, &device).unwrap();
        assert!(decoded.trusted_peers().is_empty());
    }

    #[test]
    fn paired_peer_round_trips_with_verified_trust_evidence() {
        let (mut state, root, issuer, device) = fixture();
        let peer = add_peer(&mut state, &issuer, 0x31);

        let decoded = ProductIdentityState::decode(&state.encode()).unwrap();
        decoded.validate_providers(&root, &issuer, &device).unwrap();
        let persisted = decoded.trusted_peer(peer.device_id()).unwrap();
        let trust = persisted
            .trust(&decoded.authority_state().unwrap())
            .unwrap();

        assert_eq!(persisted.credential(), peer);
        assert_eq!(trust.state(), TrustState::Trusted);
        assert_eq!(trust.device_id(), peer.device_id());
    }

    #[test]
    fn joined_device_reconstructs_owner_domain_without_authority_private_keys() {
        let (owner, _, issuer, _) = fixture();
        let authority = owner.authority_state().unwrap();
        let joiner_signer = SigningKey::from_secret_bytes([0x5a; 32]);
        let joiner_credential = DeviceCredential::issue(
            owner.owner_id(),
            DeviceId::from_bytes([0x5b; 32]),
            &joiner_signer,
            0,
            &authority,
            &issuer,
        )
        .unwrap();

        let joined = ProductIdentityState::join_owner_domain(
            owner.root(),
            owner.device_signing(),
            joiner_credential,
            &joiner_signer,
        )
        .unwrap();

        assert_eq!(joined.owner_id(), owner.owner_id());
        assert_eq!(joined.local_device_id(), joiner_credential.device_id());
        assert_eq!(joined.local_credential(), joiner_credential);
        joined
            .validate_local_device_provider(&joiner_signer)
            .unwrap();

        let restored = ProductIdentityState::decode(&joined.encode()).unwrap();
        restored
            .validate_local_device_provider(&joiner_signer)
            .unwrap();
    }

    #[test]
    fn joined_device_rejects_wrong_local_signer() {
        let (owner, _, issuer, _) = fixture();
        let authority = owner.authority_state().unwrap();
        let joiner_signer = SigningKey::from_secret_bytes([0x5c; 32]);
        let wrong_signer = SigningKey::from_secret_bytes([0x5d; 32]);
        let joiner_credential = DeviceCredential::issue(
            owner.owner_id(),
            DeviceId::from_bytes([0x5e; 32]),
            &joiner_signer,
            0,
            &authority,
            &issuer,
        )
        .unwrap();

        assert_eq!(
            ProductIdentityState::join_owner_domain(
                owner.root(),
                owner.device_signing(),
                joiner_credential,
                &wrong_signer,
            ),
            Err(ProductIdentityError::ProviderMismatch)
        );
    }

    #[test]
    fn paired_peer_commit_round_trips_through_currentness_store() {
        let (state, root, issuer, device) = fixture();
        let store = crate::MemoryIdentityStore::default();
        let first = store.compare_and_swap(None, state.encode()).unwrap();

        let authority = state.authority_state().unwrap();
        let peer_key = SigningKey::from_secret_bytes([0x61; 32]);
        let credential = DeviceCredential::issue(
            state.owner_id(),
            DeviceId::from_bytes([0x62; 32]),
            &peer_key,
            0,
            &authority,
            &issuer,
        )
        .unwrap();
        let transition = PairingTrustTransition::issue(
            &credential,
            TransitionId::from_bytes([0x63; 32]),
            [0x64; 32],
            &authority,
            &issuer,
        )
        .unwrap();
        let next = state.with_paired_peer(credential, transition).unwrap();

        assert!(state.trusted_peers().is_empty());
        let second = store
            .compare_and_swap(Some(first.revision()), next.encode())
            .unwrap();
        let loaded = store.load().unwrap().unwrap();
        assert_eq!(loaded.revision(), second.revision());

        let restored = ProductIdentityState::decode(loaded.payload()).unwrap();
        restored
            .validate_providers(&root, &issuer, &device)
            .unwrap();
        let peer = restored.trusted_peer(credential.device_id()).unwrap();
        assert_eq!(peer.credential(), credential);
        assert_eq!(
            peer.trust(&restored.authority_state().unwrap())
                .unwrap()
                .state(),
            TrustState::Trusted
        );
    }

    #[test]
    fn paired_peer_commit_rejects_stale_writer_without_partial_state() {
        let (state, _, issuer, _) = fixture();
        let store = crate::MemoryIdentityStore::default();
        let first = store.compare_and_swap(None, state.encode()).unwrap();

        let mut newer = state.clone();
        add_peer(&mut newer, &issuer, 0x71);
        let second = store
            .compare_and_swap(Some(first.revision()), newer.encode())
            .unwrap();

        let mut stale = state;
        add_peer(&mut stale, &issuer, 0x72);
        assert_eq!(
            store.compare_and_swap(Some(first.revision()), stale.encode()),
            Err(crate::IdentityStoreError::RevisionConflict)
        );

        let loaded = store.load().unwrap().unwrap();
        assert_eq!(loaded.revision(), second.revision());
        let restored = ProductIdentityState::decode(loaded.payload()).unwrap();
        assert_eq!(restored.trusted_peers().len(), 1);
        assert!(
            restored
                .trusted_peer(DeviceId::from_bytes([0x72; 32]))
                .is_some()
        );
        assert!(
            restored
                .trusted_peer(DeviceId::from_bytes([0x73; 32]))
                .is_none()
        );
    }

    #[test]
    fn paired_peer_reload_fails_closed_on_currentness_rollback() {
        let (state, _, issuer, _) = fixture();
        let store = crate::MemoryIdentityStore::default();
        let first = store.compare_and_swap(None, state.encode()).unwrap();
        let old_anchor = crate::IdentityStoreAnchor::new(first.revision(), first.envelope_digest());

        let mut paired = state;
        add_peer(&mut paired, &issuer, 0x81);
        store
            .compare_and_swap(Some(first.revision()), paired.encode())
            .unwrap();
        store.replace_anchor_for_test(old_anchor);

        assert_eq!(
            store.load(),
            Err(crate::IdentityStoreError::StaleOrMixedState)
        );
    }

    #[test]
    fn product_identity_rejects_wrong_provider() {
        let (state, root, issuer, _) = fixture();
        let wrong = SigningKey::generate().unwrap();

        assert_eq!(
            state.validate_providers(&root, &issuer, &wrong),
            Err(ProductIdentityError::ProviderMismatch)
        );
    }

    #[test]
    fn product_identity_rejects_tampered_signature() {
        let (state, _, _, _) = fixture();
        let mut encoded = state.encode();
        let local_signature_last = BASE_ENCODED_LEN - 1;
        encoded[local_signature_last] ^= 1;

        assert!(ProductIdentityState::decode(&encoded).is_err());
    }

    #[test]
    fn product_identity_rejects_tampered_peer_trust_evidence() {
        let (mut state, _, issuer, _) = fixture();
        add_peer(&mut state, &issuer, 0x41);
        let mut encoded = state.encode();
        *encoded.last_mut().unwrap() ^= 1;

        assert!(matches!(
            ProductIdentityState::decode(&encoded),
            Err(ProductIdentityError::Trust(
                PairingTrustTransitionError::InvalidSignature
            ))
        ));
    }

    #[test]
    fn duplicate_peer_is_rejected_before_persistence() {
        let (mut state, _, issuer, _) = fixture();
        let peer = add_peer(&mut state, &issuer, 0x51);
        let authority = state.authority_state().unwrap();
        let transition = PairingTrustTransition::issue(
            &peer,
            TransitionId::from_bytes([0x52; 32]),
            [0x53; 32],
            &authority,
            &issuer,
        )
        .unwrap();

        assert_eq!(
            state.add_paired_peer(peer, transition),
            Err(ProductIdentityError::DuplicatePeer)
        );
    }
}
