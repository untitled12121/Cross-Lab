use std::{fmt, net::SocketAddr, time::Duration};

use crosslab_core::{
    ChannelBinding, ConnectionMetadata, LogicalSession, SessionActivation, SessionAuthProof,
    SessionAuthRole as CoreSessionAuthRole, SessionAuthTranscriptV1, SessionError,
    SessionHandshakeSide, SessionState, TransportSecurityClass,
};
use crosslab_crypto::{SigningProvider, random_bytes};
use crosslab_identity::{DeviceCredential, DeviceId, OwnerAuthorityState};
use crosslab_policy::TrustRecord;
use crosslab_protocol::{
    FeatureSet, FrameLimit, ProtocolRange, SessionAuthBootstrapMessage, SessionAuthHello,
    SessionAuthProofMessage, SessionAuthRole as WireSessionAuthRole, decode_session_auth_bootstrap,
    encode_session_auth_bootstrap, negotiate_features, negotiate_protocol_version,
};
use quinn::{Connection, Endpoint, RecvStream, SendStream, VarInt};
use tokio::time::timeout;

use crate::{
    QuicTransportConfig,
    binding::derive_channel_binding,
    connection::QuicTransportConnection,
    record::{read_record, write_record},
};

const BOOTSTRAP_RECORD_MAX: usize = FrameLimit::BootstrapHello.max_payload_len() + 4;
const SESSION_FAILURE_CODE: VarInt = VarInt::from_u32(3);

pub struct QuicSessionAuthConfig<'a> {
    authority: &'a OwnerAuthorityState,
    local_credential: DeviceCredential,
    local_signer: &'a dyn SigningProvider,
    peer_trusts: PeerTrusts<'a>,
    protocol_ranges: Vec<ProtocolRange>,
    features: FeatureSet,
}

enum PeerTrusts<'a> {
    One(&'a TrustRecord),
    Many(&'a [TrustRecord]),
}

impl<'a> QuicSessionAuthConfig<'a> {
    pub fn new(
        authority: &'a OwnerAuthorityState,
        local_credential: DeviceCredential,
        local_signer: &'a dyn SigningProvider,
        peer_trust: &'a TrustRecord,
        protocol_ranges: Vec<ProtocolRange>,
        features: FeatureSet,
    ) -> Self {
        Self {
            authority,
            local_credential,
            local_signer,
            peer_trusts: PeerTrusts::One(peer_trust),
            protocol_ranges,
            features,
        }
    }

    pub fn new_with_peer_trusts(
        authority: &'a OwnerAuthorityState,
        local_credential: DeviceCredential,
        local_signer: &'a dyn SigningProvider,
        peer_trusts: &'a [TrustRecord],
        protocol_ranges: Vec<ProtocolRange>,
        features: FeatureSet,
    ) -> Self {
        Self {
            authority,
            local_credential,
            local_signer,
            peer_trusts: PeerTrusts::Many(peer_trusts),
            protocol_ranges,
            features,
        }
    }

    fn peer_trust(&self, device_id: DeviceId) -> Option<&TrustRecord> {
        match self.peer_trusts {
            PeerTrusts::One(trust) => Some(trust),
            PeerTrusts::Many(trusts) => trusts.iter().find(|trust| trust.device_id() == device_id),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct QuicSessionTimeouts {
    connect: Duration,
    bootstrap: Duration,
}

impl QuicSessionTimeouts {
    pub const fn new(connect: Duration, bootstrap: Duration) -> Self {
        Self { connect, bootstrap }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum QuicSessionError {
    Connect,
    Accept,
    Timeout,
    ChannelBinding,
    Random,
    ControlStream,
    Bootstrap,
    UnexpectedBootstrapMessage,
    Session(SessionError),
}

impl fmt::Display for QuicSessionError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Connect => formatter.write_str("QUIC connection failed"),
            Self::Accept => formatter.write_str("QUIC accept failed"),
            Self::Timeout => formatter.write_str("QUIC session setup timed out"),
            Self::ChannelBinding => formatter.write_str("QUIC channel binding failed"),
            Self::Random => formatter.write_str("secure session nonce generation failed"),
            Self::ControlStream => formatter.write_str("QUIC control stream setup failed"),
            Self::Bootstrap => formatter.write_str("QUIC session bootstrap record failed"),
            Self::UnexpectedBootstrapMessage => {
                formatter.write_str("unexpected QUIC session bootstrap message")
            }
            Self::Session(error) => fmt::Display::fmt(error, formatter),
        }
    }
}

impl std::error::Error for QuicSessionError {}

impl From<SessionError> for QuicSessionError {
    fn from(error: SessionError) -> Self {
        Self::Session(error)
    }
}

pub struct AuthenticatedQuicSession {
    session: LogicalSession,
    transport: QuicTransportConnection,
}

impl AuthenticatedQuicSession {
    pub const fn session(&self) -> &LogicalSession {
        &self.session
    }

    pub const fn transport(&self) -> &QuicTransportConnection {
        &self.transport
    }

    pub fn into_parts(self) -> (LogicalSession, QuicTransportConnection) {
        (self.session, self.transport)
    }
}

pub(crate) async fn connect_authenticated(
    endpoint: &Endpoint,
    remote_addr: SocketAddr,
    server_name: &str,
    auth: &QuicSessionAuthConfig<'_>,
    timeouts: QuicSessionTimeouts,
    transport_config: QuicTransportConfig,
) -> Result<AuthenticatedQuicSession, QuicSessionError> {
    let connecting = endpoint
        .connect(remote_addr, server_name)
        .map_err(|_| QuicSessionError::Connect)?;
    let connection = timeout(timeouts.connect, connecting)
        .await
        .map_err(|_| QuicSessionError::Timeout)?
        .map_err(|_| QuicSessionError::Connect)?;

    authenticate_with_timeout(
        endpoint.clone(),
        connection,
        auth,
        timeouts.bootstrap,
        transport_config,
        CoreSessionAuthRole::Initiator,
    )
    .await
}

pub(crate) async fn accept_authenticated(
    endpoint: &Endpoint,
    auth: &QuicSessionAuthConfig<'_>,
    timeouts: QuicSessionTimeouts,
    transport_config: QuicTransportConfig,
) -> Result<AuthenticatedQuicSession, QuicSessionError> {
    let incoming = timeout(timeouts.connect, endpoint.accept())
        .await
        .map_err(|_| QuicSessionError::Timeout)?
        .ok_or(QuicSessionError::Accept)?;
    let connection = timeout(timeouts.connect, incoming)
        .await
        .map_err(|_| QuicSessionError::Timeout)?
        .map_err(|_| QuicSessionError::Accept)?;

    authenticate_with_timeout(
        endpoint.clone(),
        connection,
        auth,
        timeouts.bootstrap,
        transport_config,
        CoreSessionAuthRole::Responder,
    )
    .await
}

async fn authenticate_with_timeout(
    endpoint: Endpoint,
    connection: Connection,
    auth: &QuicSessionAuthConfig<'_>,
    bootstrap_timeout: Duration,
    transport_config: QuicTransportConfig,
    local_role: CoreSessionAuthRole,
) -> Result<AuthenticatedQuicSession, QuicSessionError> {
    let result = match local_role {
        CoreSessionAuthRole::Initiator => {
            timeout(bootstrap_timeout, bootstrap_initiator(&connection, auth)).await
        }
        CoreSessionAuthRole::Responder => {
            timeout(bootstrap_timeout, bootstrap_responder(&connection, auth)).await
        }
    };

    let bootstrap = match result {
        Ok(Ok(bootstrap)) => bootstrap,
        Ok(Err(error)) => {
            connection.close(
                SESSION_FAILURE_CODE,
                b"crosslab session authentication failed",
            );
            return Err(error);
        }
        Err(_) => {
            connection.close(
                SESSION_FAILURE_CODE,
                b"crosslab session authentication timed out",
            );
            return Err(QuicSessionError::Timeout);
        }
    };

    let local_endpoint = endpoint
        .local_addr()
        .ok()
        .map(|address| address.to_string());
    let remote_endpoint = Some(connection.remote_address().to_string());
    let metadata = ConnectionMetadata::new(local_endpoint, remote_endpoint, None);
    let transport = QuicTransportConnection::new_owned(
        endpoint,
        connection,
        bootstrap.control_send,
        bootstrap.control_recv,
        bootstrap.binding,
        metadata,
        transport_config,
    );
    Ok(AuthenticatedQuicSession {
        session: bootstrap.session,
        transport,
    })
}

struct AuthenticatedBootstrap {
    session: LogicalSession,
    control_send: SendStream,
    control_recv: RecvStream,
    binding: ChannelBinding,
}

async fn bootstrap_initiator(
    connection: &Connection,
    auth: &QuicSessionAuthConfig<'_>,
) -> Result<AuthenticatedBootstrap, QuicSessionError> {
    let binding =
        derive_channel_binding(connection).map_err(|_| QuicSessionError::ChannelBinding)?;
    let initiator_hello = local_hello(auth)?;
    let (mut control_send, mut control_recv) = connection
        .open_bi()
        .await
        .map_err(|_| QuicSessionError::ControlStream)?;
    send_bootstrap(
        &mut control_send,
        &SessionAuthBootstrapMessage::Hello(initiator_hello.clone()),
    )
    .await?;
    let responder_hello = expect_hello(receive_bootstrap(&mut control_recv).await?)?;
    let transcript = transcript_for(auth, &initiator_hello, &responder_hello, &binding)?;
    let initiator_proof = transcript
        .create_proof_with_provider(CoreSessionAuthRole::Initiator, auth.local_signer)
        .map_err(|error| QuicSessionError::Session(SessionError::Auth(error)))?;
    send_bootstrap(
        &mut control_send,
        &SessionAuthBootstrapMessage::Proof(proof_message(initiator_proof)),
    )
    .await?;
    let responder_proof =
        proof_from_message(expect_proof(receive_bootstrap(&mut control_recv).await?)?);
    let session = authenticate_session(
        auth,
        &initiator_hello,
        &responder_hello,
        &binding,
        CoreSessionAuthRole::Initiator,
        initiator_proof,
        responder_proof,
    )?;

    Ok(AuthenticatedBootstrap {
        session,
        control_send,
        control_recv,
        binding,
    })
}

async fn bootstrap_responder(
    connection: &Connection,
    auth: &QuicSessionAuthConfig<'_>,
) -> Result<AuthenticatedBootstrap, QuicSessionError> {
    let binding =
        derive_channel_binding(connection).map_err(|_| QuicSessionError::ChannelBinding)?;
    let (mut control_send, mut control_recv) = connection
        .accept_bi()
        .await
        .map_err(|_| QuicSessionError::ControlStream)?;
    let initiator_hello = expect_hello(receive_bootstrap(&mut control_recv).await?)?;
    let responder_hello = local_hello(auth)?;
    send_bootstrap(
        &mut control_send,
        &SessionAuthBootstrapMessage::Hello(responder_hello.clone()),
    )
    .await?;
    let transcript = transcript_for(auth, &initiator_hello, &responder_hello, &binding)?;
    let initiator_proof =
        proof_from_message(expect_proof(receive_bootstrap(&mut control_recv).await?)?);
    let responder_proof = transcript
        .create_proof_with_provider(CoreSessionAuthRole::Responder, auth.local_signer)
        .map_err(|error| QuicSessionError::Session(SessionError::Auth(error)))?;
    send_bootstrap(
        &mut control_send,
        &SessionAuthBootstrapMessage::Proof(proof_message(responder_proof)),
    )
    .await?;
    let session = authenticate_session(
        auth,
        &initiator_hello,
        &responder_hello,
        &binding,
        CoreSessionAuthRole::Responder,
        initiator_proof,
        responder_proof,
    )?;

    Ok(AuthenticatedBootstrap {
        session,
        control_send,
        control_recv,
        binding,
    })
}

fn local_hello(auth: &QuicSessionAuthConfig<'_>) -> Result<SessionAuthHello, QuicSessionError> {
    let nonce = random_bytes::<32>().map_err(|_| QuicSessionError::Random)?;
    Ok(SessionAuthHello::new(
        auth.local_credential,
        auth.protocol_ranges.clone(),
        auth.features.clone(),
        nonce,
    ))
}

fn transcript_for(
    auth: &QuicSessionAuthConfig<'_>,
    initiator: &SessionAuthHello,
    responder: &SessionAuthHello,
    binding: &ChannelBinding,
) -> Result<SessionAuthTranscriptV1, QuicSessionError> {
    let initiator_credential = initiator.device_credential();
    let responder_credential = responder.device_credential();
    let protocol =
        negotiate_protocol_version(initiator.protocol_ranges(), responder.protocol_ranges())
            .map_err(SessionError::Protocol)?;
    let features = negotiate_features(initiator.features(), responder.features())
        .map_err(SessionError::Feature)?;
    SessionAuthTranscriptV1::new(
        auth.authority.root().owner_id(),
        &initiator_credential,
        initiator.nonce(),
        &responder_credential,
        responder.nonce(),
        protocol,
        &features,
        binding.profile_id().as_bytes(),
        binding.bytes(),
    )
    .map_err(|error| QuicSessionError::Session(SessionError::Auth(error)))
}

#[allow(clippy::too_many_arguments)]
fn authenticate_session(
    auth: &QuicSessionAuthConfig<'_>,
    initiator: &SessionAuthHello,
    responder: &SessionAuthHello,
    binding: &ChannelBinding,
    local_role: CoreSessionAuthRole,
    initiator_proof: SessionAuthProof,
    responder_proof: SessionAuthProof,
) -> Result<LogicalSession, QuicSessionError> {
    let initiator_credential = initiator.device_credential();
    let responder_credential = responder.device_credential();
    let initiator_side = SessionHandshakeSide::new(
        &initiator_credential,
        initiator.protocol_ranges(),
        initiator.features(),
    );
    let responder_side = SessionHandshakeSide::new(
        &responder_credential,
        responder.protocol_ranges(),
        responder.features(),
    );
    let peer_device_id = match local_role {
        CoreSessionAuthRole::Initiator => responder_credential.device_id(),
        CoreSessionAuthRole::Responder => initiator_credential.device_id(),
    };
    let peer_trust = auth
        .peer_trust(peer_device_id)
        .ok_or(QuicSessionError::Session(SessionError::PeerNotTrusted))?;
    let mut session = LogicalSession::new();
    session.authenticate(SessionActivation::new(
        auth.authority,
        initiator_side,
        responder_side,
        local_role,
        peer_trust,
        initiator.nonce(),
        responder.nonce(),
        binding,
        TransportSecurityClass::AuthenticatedConfidentialChannel,
        &initiator_proof,
        &responder_proof,
    ))?;
    if session.state() != SessionState::Active {
        return Err(QuicSessionError::Session(SessionError::InvalidState));
    }
    Ok(session)
}

fn proof_message(proof: SessionAuthProof) -> SessionAuthProofMessage {
    SessionAuthProofMessage::new(
        wire_role(proof.role()),
        proof.transcript_digest(),
        proof.signature(),
    )
}

fn proof_from_message(message: SessionAuthProofMessage) -> SessionAuthProof {
    SessionAuthProof::from_parts(
        core_role(message.role()),
        message.transcript_digest(),
        message.signature(),
    )
}

const fn wire_role(role: CoreSessionAuthRole) -> WireSessionAuthRole {
    match role {
        CoreSessionAuthRole::Initiator => WireSessionAuthRole::Initiator,
        CoreSessionAuthRole::Responder => WireSessionAuthRole::Responder,
    }
}

const fn core_role(role: WireSessionAuthRole) -> CoreSessionAuthRole {
    match role {
        WireSessionAuthRole::Initiator => CoreSessionAuthRole::Initiator,
        WireSessionAuthRole::Responder => CoreSessionAuthRole::Responder,
    }
}

async fn send_bootstrap(
    send: &mut SendStream,
    message: &SessionAuthBootstrapMessage,
) -> Result<(), QuicSessionError> {
    let frame = encode_session_auth_bootstrap(message).map_err(|_| QuicSessionError::Bootstrap)?;
    write_record(send, &frame, BOOTSTRAP_RECORD_MAX)
        .await
        .map_err(|_| QuicSessionError::Bootstrap)
}

async fn receive_bootstrap(
    recv: &mut RecvStream,
) -> Result<SessionAuthBootstrapMessage, QuicSessionError> {
    let frame = read_record(recv, BOOTSTRAP_RECORD_MAX, false)
        .await
        .map_err(|_| QuicSessionError::Bootstrap)?;
    decode_session_auth_bootstrap(&frame).map_err(|_| QuicSessionError::Bootstrap)
}

fn expect_hello(
    message: SessionAuthBootstrapMessage,
) -> Result<SessionAuthHello, QuicSessionError> {
    match message {
        SessionAuthBootstrapMessage::Hello(hello) => Ok(hello),
        SessionAuthBootstrapMessage::Proof(_) => Err(QuicSessionError::UnexpectedBootstrapMessage),
    }
}

fn expect_proof(
    message: SessionAuthBootstrapMessage,
) -> Result<SessionAuthProofMessage, QuicSessionError> {
    match message {
        SessionAuthBootstrapMessage::Proof(proof) => Ok(proof),
        SessionAuthBootstrapMessage::Hello(_) => Err(QuicSessionError::UnexpectedBootstrapMessage),
    }
}

#[cfg(test)]
mod tests;
