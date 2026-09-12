use core::fmt;

use crosslab_identity::{
    AuthorityDelegation, DeviceCredential, DeviceId, IdentityError, OwnerId, OwnerRootRecord,
};
use crosslab_policy::{LocalCapability, SessionId, TrustRecord, TrustState};
use crosslab_protocol::{
    CapabilityAdvertisement, FeatureNegotiationError, FeatureSet, ProtocolRange, ProtocolVersion,
    VersionNegotiationError, negotiate_features, negotiate_protocol_version,
};

use crate::{ChannelBinding, TransportSecurityClass};

use super::{
    NegotiatedCapability, SessionAuthError, SessionAuthProof, SessionAuthRole,
    SessionAuthTranscriptV1, negotiate_session_capabilities,
};

#[derive(Clone, Copy)]
pub struct SessionHandshakeSide<'a> {
    credential: &'a DeviceCredential,
    issuer: &'a AuthorityDelegation,
    protocol_ranges: &'a [ProtocolRange],
    features: &'a FeatureSet,
}

impl<'a> SessionHandshakeSide<'a> {
    pub const fn new(
        credential: &'a DeviceCredential,
        issuer: &'a AuthorityDelegation,
        protocol_ranges: &'a [ProtocolRange],
        features: &'a FeatureSet,
    ) -> Self {
        Self {
            credential,
            issuer,
            protocol_ranges,
            features,
        }
    }
}

pub struct SessionActivation<'a> {
    root: &'a OwnerRootRecord,
    initiator: SessionHandshakeSide<'a>,
    responder: SessionHandshakeSide<'a>,
    local_role: SessionAuthRole,
    peer_trust: &'a TrustRecord,
    initiator_nonce: [u8; 32],
    responder_nonce: [u8; 32],
    channel_binding: &'a ChannelBinding,
    transport_security_class: TransportSecurityClass,
    initiator_proof: &'a SessionAuthProof,
    responder_proof: &'a SessionAuthProof,
}

impl<'a> SessionActivation<'a> {
    #[allow(clippy::too_many_arguments)]
    pub const fn new(
        root: &'a OwnerRootRecord,
        initiator: SessionHandshakeSide<'a>,
        responder: SessionHandshakeSide<'a>,
        local_role: SessionAuthRole,
        peer_trust: &'a TrustRecord,
        initiator_nonce: [u8; 32],
        responder_nonce: [u8; 32],
        channel_binding: &'a ChannelBinding,
        transport_security_class: TransportSecurityClass,
        initiator_proof: &'a SessionAuthProof,
        responder_proof: &'a SessionAuthProof,
    ) -> Self {
        Self {
            root,
            initiator,
            responder,
            local_role,
            peer_trust,
            initiator_nonce,
            responder_nonce,
            channel_binding,
            transport_security_class,
            initiator_proof,
            responder_proof,
        }
    }

    fn local_and_peer(&self) -> (SessionHandshakeSide<'a>, SessionHandshakeSide<'a>) {
        match self.local_role {
            SessionAuthRole::Initiator => (self.initiator, self.responder),
            SessionAuthRole::Responder => (self.responder, self.initiator),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SessionState {
    Created,
    Authenticating,
    Active,
    Closing,
    Closed,
    Revoked,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SessionError {
    InvalidState,
    Identity(IdentityError),
    PeerNotTrusted,
    PeerNotRevoked,
    PeerTrustMismatch,
    PeerCredentialEpochMismatch,
    PeerTrustRevisionNotAdvanced,
    Protocol(VersionNegotiationError),
    Feature(FeatureNegotiationError),
    Auth(SessionAuthError),
}

impl fmt::Display for SessionError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidState => {
                formatter.write_str("logical session state transition is invalid")
            }
            Self::Identity(error) => fmt::Display::fmt(error, formatter),
            Self::PeerNotTrusted => formatter.write_str("peer device is not trusted"),
            Self::PeerNotRevoked => formatter.write_str("peer device is not revoked"),
            Self::PeerTrustMismatch => {
                formatter.write_str("peer trust record does not match the authenticated identity")
            }
            Self::PeerCredentialEpochMismatch => formatter
                .write_str("peer credential epoch does not match the locally accepted trust epoch"),
            Self::PeerTrustRevisionNotAdvanced => formatter
                .write_str("peer trust revision does not advance the authenticated snapshot"),
            Self::Protocol(error) => fmt::Display::fmt(error, formatter),
            Self::Feature(error) => fmt::Display::fmt(error, formatter),
            Self::Auth(error) => fmt::Display::fmt(error, formatter),
        }
    }
}

impl std::error::Error for SessionError {}

impl From<IdentityError> for SessionError {
    fn from(error: IdentityError) -> Self {
        Self::Identity(error)
    }
}

impl From<VersionNegotiationError> for SessionError {
    fn from(error: VersionNegotiationError) -> Self {
        Self::Protocol(error)
    }
}

impl From<FeatureNegotiationError> for SessionError {
    fn from(error: FeatureNegotiationError) -> Self {
        Self::Feature(error)
    }
}

impl From<SessionAuthError> for SessionError {
    fn from(error: SessionAuthError) -> Self {
        Self::Auth(error)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SessionContext {
    session_id: SessionId,
    local_device_id: DeviceId,
    peer_device_id: DeviceId,
    owner_id: OwnerId,
    peer_credential_epoch: u64,
    peer_trust_revision: u64,
    protocol_version: ProtocolVersion,
    negotiated_features: Vec<u16>,
    negotiated_capabilities: Vec<NegotiatedCapability>,
    transport_security_class: TransportSecurityClass,
    next_send_sequence: u64,
    next_receive_sequence: u64,
}

impl SessionContext {
    pub const fn session_id(&self) -> SessionId {
        self.session_id
    }

    pub const fn local_device_id(&self) -> DeviceId {
        self.local_device_id
    }

    pub const fn peer_device_id(&self) -> DeviceId {
        self.peer_device_id
    }

    pub const fn owner_id(&self) -> OwnerId {
        self.owner_id
    }

    pub const fn peer_credential_epoch(&self) -> u64 {
        self.peer_credential_epoch
    }

    pub const fn peer_trust_revision(&self) -> u64 {
        self.peer_trust_revision
    }

    pub const fn protocol_version(&self) -> ProtocolVersion {
        self.protocol_version
    }

    pub fn negotiated_features(&self) -> &[u16] {
        &self.negotiated_features
    }

    pub fn negotiated_capabilities(&self) -> &[NegotiatedCapability] {
        &self.negotiated_capabilities
    }

    pub const fn transport_security_class(&self) -> TransportSecurityClass {
        self.transport_security_class
    }

    pub const fn next_send_sequence(&self) -> u64 {
        self.next_send_sequence
    }

    pub const fn next_receive_sequence(&self) -> u64 {
        self.next_receive_sequence
    }
}

#[derive(Debug)]
pub struct LogicalSession {
    state: SessionState,
    context: Option<SessionContext>,
}

impl LogicalSession {
    pub const fn new() -> Self {
        Self {
            state: SessionState::Created,
            context: None,
        }
    }

    pub const fn state(&self) -> SessionState {
        self.state
    }

    pub const fn context(&self) -> Option<&SessionContext> {
        self.context.as_ref()
    }

    pub fn authenticate(&mut self, activation: SessionActivation<'_>) -> Result<(), SessionError> {
        if self.state != SessionState::Created {
            return Err(SessionError::InvalidState);
        }

        self.state = SessionState::Authenticating;
        match authenticate(activation) {
            Ok(context) => {
                self.context = Some(context);
                self.state = SessionState::Active;
                Ok(())
            }
            Err(error) => {
                self.context = None;
                self.state = SessionState::Closed;
                Err(error)
            }
        }
    }

    pub fn negotiate_capabilities(
        &mut self,
        local: &[LocalCapability],
        peer: &CapabilityAdvertisement,
    ) -> Result<(), SessionError> {
        if self.state != SessionState::Active {
            return Err(SessionError::InvalidState);
        }
        let context = self.context.as_mut().ok_or(SessionError::InvalidState)?;
        context.negotiated_capabilities = negotiate_session_capabilities(local, peer);
        Ok(())
    }

    pub fn begin_close(&mut self) -> Result<(), SessionError> {
        if self.state != SessionState::Active {
            return Err(SessionError::InvalidState);
        }
        self.state = SessionState::Closing;
        Ok(())
    }

    pub fn finish_close(&mut self) -> Result<(), SessionError> {
        if !matches!(self.state, SessionState::Closing | SessionState::Revoked) {
            return Err(SessionError::InvalidState);
        }
        self.state = SessionState::Closed;
        Ok(())
    }

    pub fn transport_lost(&mut self) -> Result<(), SessionError> {
        match self.state {
            SessionState::Active | SessionState::Closing | SessionState::Revoked => {
                self.state = SessionState::Closed;
                Ok(())
            }
            SessionState::Closed => Ok(()),
            SessionState::Created | SessionState::Authenticating => Err(SessionError::InvalidState),
        }
    }

    pub fn apply_peer_revocation(&mut self, peer_trust: &TrustRecord) -> Result<(), SessionError> {
        if self.state != SessionState::Active {
            return Err(SessionError::InvalidState);
        }
        let context = self.context.as_ref().ok_or(SessionError::InvalidState)?;
        if peer_trust.state() != TrustState::Revoked {
            return Err(SessionError::PeerNotRevoked);
        }
        if peer_trust.owner_id() != context.owner_id()
            || peer_trust.device_id() != context.peer_device_id()
        {
            return Err(SessionError::PeerTrustMismatch);
        }
        if peer_trust.accepted_credential_epoch() != context.peer_credential_epoch() {
            return Err(SessionError::PeerCredentialEpochMismatch);
        }
        if peer_trust.trust_revision() <= context.peer_trust_revision() {
            return Err(SessionError::PeerTrustRevisionNotAdvanced);
        }

        self.state = SessionState::Revoked;
        Ok(())
    }

    pub fn revoke(&mut self) -> Result<(), SessionError> {
        if self.state != SessionState::Active {
            return Err(SessionError::InvalidState);
        }
        self.state = SessionState::Revoked;
        Ok(())
    }
}

impl Default for LogicalSession {
    fn default() -> Self {
        Self::new()
    }
}

fn authenticate(activation: SessionActivation<'_>) -> Result<SessionContext, SessionError> {
    activation.initiator.credential.verify(
        activation.root,
        activation.initiator.issuer,
        0,
        activation.initiator.issuer.delegation_epoch(),
    )?;
    activation.responder.credential.verify(
        activation.root,
        activation.responder.issuer,
        0,
        activation.responder.issuer.delegation_epoch(),
    )?;

    let (local, peer) = activation.local_and_peer();
    if activation.peer_trust.state() != TrustState::Trusted {
        return Err(SessionError::PeerNotTrusted);
    }
    if activation.peer_trust.owner_id() != peer.credential.owner_id()
        || activation.peer_trust.device_id() != peer.credential.device_id()
    {
        return Err(SessionError::PeerTrustMismatch);
    }
    if activation.peer_trust.accepted_credential_epoch() != peer.credential.credential_epoch() {
        return Err(SessionError::PeerCredentialEpochMismatch);
    }

    let protocol_version = negotiate_protocol_version(
        activation.initiator.protocol_ranges,
        activation.responder.protocol_ranges,
    )?;
    let negotiated_features =
        negotiate_features(activation.initiator.features, activation.responder.features)?;
    let transcript = SessionAuthTranscriptV1::new(
        activation.root.owner_id(),
        activation.initiator.credential,
        activation.initiator_nonce,
        activation.responder.credential,
        activation.responder_nonce,
        protocol_version,
        &negotiated_features,
        activation.channel_binding.profile_id().as_bytes(),
        activation.channel_binding.bytes(),
    )?;
    let session_id = transcript.derive_session_id(
        activation.initiator_proof,
        activation.initiator.credential.device_public_key(),
        activation.responder_proof,
        activation.responder.credential.device_public_key(),
    )?;

    Ok(SessionContext {
        session_id,
        local_device_id: local.credential.device_id(),
        peer_device_id: peer.credential.device_id(),
        owner_id: activation.root.owner_id(),
        peer_credential_epoch: peer.credential.credential_epoch(),
        peer_trust_revision: activation.peer_trust.trust_revision(),
        protocol_version,
        negotiated_features,
        negotiated_capabilities: Vec::new(),
        transport_security_class: activation.transport_security_class,
        next_send_sequence: 0,
        next_receive_sequence: 0,
    })
}

#[cfg(test)]
mod tests {
    use crosslab_crypto::SigningKey;
    use crosslab_policy::{TransitionId, TrustTransition};

    use super::*;

    #[test]
    fn peer_revocation_requires_revision_newer_than_authenticated_snapshot() {
        let owner_id = OwnerId::from_bytes([0xf0; 32]);
        let root_key = SigningKey::from_secret_bytes([0xf1; 32]);
        let root = OwnerRootRecord::new(owner_id, &root_key, 0);
        let peer_device_id = DeviceId::from_bytes([0xf2; 32]);
        let peer_credential_epoch = 7;
        let trusted = TrustRecord::trusted(
            owner_id,
            peer_device_id,
            peer_credential_epoch,
            TransitionId::from_bytes([0xf3; 32]),
        );
        let transition = TrustTransition::issue_root_revocation(
            &trusted,
            TransitionId::from_bytes([0xf4; 32]),
            &root,
            &root_key,
        )
        .unwrap();
        let mut revoked = trusted;
        transition.apply_root(&mut revoked, &root).unwrap();

        let context = SessionContext {
            session_id: SessionId::from_bytes([0xf5; 32]),
            local_device_id: DeviceId::from_bytes([0xf6; 32]),
            peer_device_id,
            owner_id,
            peer_credential_epoch,
            peer_trust_revision: revoked.trust_revision(),
            protocol_version: ProtocolVersion::new(1, 0),
            negotiated_features: Vec::new(),
            negotiated_capabilities: Vec::new(),
            transport_security_class: TransportSecurityClass::InProcessTest,
            next_send_sequence: 0,
            next_receive_sequence: 0,
        };
        let mut session = LogicalSession {
            state: SessionState::Active,
            context: Some(context),
        };

        assert_eq!(
            session.apply_peer_revocation(&revoked),
            Err(SessionError::PeerTrustRevisionNotAdvanced)
        );
        assert_eq!(session.state(), SessionState::Active);
    }
}
