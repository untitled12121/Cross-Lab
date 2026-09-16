use crosslab_core::{
    ChannelBinding, ConnectionMetadata, LogicalSession, SessionActivation,
    SessionAuthRole as CoreSessionAuthRole, SessionAuthTranscriptV1, SessionHandshakeSide,
    TransportConnection, TransportSecurityClass,
};
use crosslab_crypto::{SigningKey, blake3_256};
use crosslab_identity::{
    AuthorityDelegation, AuthorityRole, OwnerAuthorityState, OwnerId, OwnerRootRecord,
};
use crosslab_policy::NetworkClass;
use crosslab_protocol::{
    FeatureSet, FrameLimit, ProtocolRange, SessionAuthBootstrapMessage, SessionAuthHello,
    SessionAuthProofMessage, SessionAuthRole as WireSessionAuthRole, decode_session_auth_bootstrap,
    encode_session_auth_bootstrap, negotiate_features, negotiate_protocol_version,
};
use iroh::endpoint::{Connection, RecvStream, SendStream};

use crate::{
    candidate::{
        connection::IrohTransportConnection,
        control::ControlBridge,
        endpoint::{DirectPair, direct_pair},
        record::{read_record, write_record},
        runtime::CandidateConfig,
    },
    error::EvalError,
    scenarios::auth::AuthFixture,
};

const BOOTSTRAP_RECORD_MAX: usize = FrameLimit::BootstrapHello.max_payload_len() + 4;
const CONTROL_STREAM_MARKER: &[u8] = b"crosslab-m9-control-v1";

pub struct SplitAuthenticatedPair {
    direct: DirectPair,
    client_transport: IrohTransportConnection,
    server_transport: IrohTransportConnection,
    client_session: LogicalSession,
    server_session: LogicalSession,
}

impl SplitAuthenticatedPair {
    pub fn client_transport(&self) -> &dyn TransportConnection {
        &self.client_transport
    }

    pub fn server_transport(&self) -> &dyn TransportConnection {
        &self.server_transport
    }

    pub fn client_session(&self) -> &LogicalSession {
        &self.client_session
    }

    pub fn server_session(&self) -> &LogicalSession {
        &self.server_session
    }

    pub const fn network_class(&self) -> NetworkClass {
        NetworkClass::Remote
    }

    pub async fn shutdown(self) {
        tokio::join!(
            self.client_transport.shutdown(),
            self.server_transport.shutdown()
        );
        self.direct.shutdown().await;
    }
}

pub(crate) struct AuthenticatedSide {
    pub(crate) transport: IrohTransportConnection,
    pub(crate) session: LogicalSession,
}

pub async fn authenticate_split_direct_pair(
    fixture: &AuthFixture,
) -> Result<SplitAuthenticatedPair, EvalError> {
    let direct = direct_pair().await?;
    let (client_control, server_control) = tokio::join!(
        reserve_control_initiator(direct.client_connection()),
        reserve_control_responder(direct.server_connection())
    );
    let (client_send, client_recv) = client_control?;
    let (server_send, server_recv) = server_control?;

    let (client, server) = tokio::join!(
        authenticate_side(
            fixture,
            CoreSessionAuthRole::Initiator,
            direct.client_connection().clone(),
            direct.client_binding().clone(),
            client_send,
            client_recv,
        ),
        authenticate_side(
            fixture,
            CoreSessionAuthRole::Responder,
            direct.server_connection().clone(),
            direct.server_binding().clone(),
            server_send,
            server_recv,
        )
    );
    let client = client?;
    let server = server?;

    Ok(SplitAuthenticatedPair {
        direct,
        client_transport: client.transport,
        server_transport: server.transport,
        client_session: client.session,
        server_session: server.session,
    })
}

pub(crate) async fn reserve_control_initiator(
    connection: &Connection,
) -> Result<(SendStream, RecvStream), EvalError> {
    let (mut send, recv) = connection.open_bi().await.map_err(|_| EvalError::Control)?;
    write_record(
        &mut send,
        CONTROL_STREAM_MARKER,
        CONTROL_STREAM_MARKER.len(),
    )
    .await
    .map_err(|_| EvalError::Control)?;
    Ok((send, recv))
}

pub(crate) async fn reserve_control_responder(
    connection: &Connection,
) -> Result<(SendStream, RecvStream), EvalError> {
    let (send, mut recv) = connection
        .accept_bi()
        .await
        .map_err(|_| EvalError::Control)?;
    let marker = read_record(&mut recv, CONTROL_STREAM_MARKER.len(), false)
        .await
        .map_err(|_| EvalError::Control)?;
    if marker != CONTROL_STREAM_MARKER {
        return Err(EvalError::Control);
    }
    Ok((send, recv))
}

pub(crate) async fn authenticate_side(
    fixture: &AuthFixture,
    role: CoreSessionAuthRole,
    connection: Connection,
    binding: ChannelBinding,
    mut send: SendStream,
    mut recv: RecvStream,
) -> Result<AuthenticatedSide, EvalError> {
    let local_hello = hello_for(fixture, role, &binding);
    let (initiator_hello, responder_hello) = match role {
        CoreSessionAuthRole::Initiator => {
            send_bootstrap(
                &mut send,
                &SessionAuthBootstrapMessage::Hello(local_hello.clone()),
            )
            .await?;
            let responder = expect_hello(receive_bootstrap(&mut recv).await?)?;
            (local_hello, responder)
        }
        CoreSessionAuthRole::Responder => {
            let initiator = expect_hello(receive_bootstrap(&mut recv).await?)?;
            send_bootstrap(
                &mut send,
                &SessionAuthBootstrapMessage::Hello(local_hello.clone()),
            )
            .await?;
            (initiator, local_hello)
        }
    };

    let initiator_credential = initiator_hello.device_credential();
    let responder_credential = responder_hello.device_credential();
    let protocol = negotiate_protocol_version(
        initiator_hello.protocol_ranges(),
        responder_hello.protocol_ranges(),
    )
    .map_err(|_| EvalError::Control)?;
    let features = negotiate_features(initiator_hello.features(), responder_hello.features())
        .map_err(|_| EvalError::Control)?;
    let transcript = SessionAuthTranscriptV1::new(
        initiator_credential.owner_id(),
        &initiator_credential,
        initiator_hello.nonce(),
        &responder_credential,
        responder_hello.nonce(),
        protocol,
        &features,
        binding.profile_id().as_bytes(),
        binding.bytes(),
    )
    .map_err(|_| EvalError::Control)?;

    // The deterministic M9 fixture recreates peer proof objects only to bridge the
    // existing wire/core proof types without widening a production core API.
    let initiator_key = SigningKey::from_secret_bytes([0x43; 32]);
    let responder_key = SigningKey::from_secret_bytes([0x44; 32]);
    let initiator_proof = transcript
        .create_proof(CoreSessionAuthRole::Initiator, &initiator_key)
        .map_err(|_| EvalError::Control)?;
    let responder_proof = transcript
        .create_proof(CoreSessionAuthRole::Responder, &responder_key)
        .map_err(|_| EvalError::Control)?;
    let expected_initiator = proof_message(initiator_proof);
    let expected_responder = proof_message(responder_proof);

    match role {
        CoreSessionAuthRole::Initiator => {
            send_bootstrap(
                &mut send,
                &SessionAuthBootstrapMessage::Proof(expected_initiator),
            )
            .await?;
            if expect_proof(receive_bootstrap(&mut recv).await?)? != expected_responder {
                return Err(EvalError::Control);
            }
        }
        CoreSessionAuthRole::Responder => {
            if expect_proof(receive_bootstrap(&mut recv).await?)? != expected_initiator {
                return Err(EvalError::Control);
            }
            send_bootstrap(
                &mut send,
                &SessionAuthBootstrapMessage::Proof(expected_responder),
            )
            .await?;
        }
    }

    let authority = fixture_authority()?;
    let peer_trust = match role {
        CoreSessionAuthRole::Initiator => fixture.responder_trust(),
        CoreSessionAuthRole::Responder => fixture.initiator_trust(),
    };
    let initiator_side = SessionHandshakeSide::new(
        &initiator_credential,
        initiator_hello.protocol_ranges(),
        initiator_hello.features(),
    );
    let responder_side = SessionHandshakeSide::new(
        &responder_credential,
        responder_hello.protocol_ranges(),
        responder_hello.features(),
    );
    let mut session = LogicalSession::new();
    session
        .authenticate(SessionActivation::new(
            &authority,
            initiator_side,
            responder_side,
            role,
            &peer_trust,
            initiator_hello.nonce(),
            responder_hello.nonce(),
            &binding,
            TransportSecurityClass::AuthenticatedConfidentialChannel,
            &initiator_proof,
            &responder_proof,
        ))
        .map_err(|_| EvalError::Control)?;

    let config = CandidateConfig::default();
    let control = ControlBridge::new(connection, send, recv, config);
    let transport = IrohTransportConnection::new(
        control,
        binding,
        ConnectionMetadata::new(None, None, None),
        config,
    );

    Ok(AuthenticatedSide { transport, session })
}

fn hello_for(
    fixture: &AuthFixture,
    role: CoreSessionAuthRole,
    binding: &ChannelBinding,
) -> SessionAuthHello {
    match role {
        CoreSessionAuthRole::Initiator => SessionAuthHello::new(
            *fixture.initiator_credential(),
            vec![ProtocolRange::new(1, 0, 3).expect("static protocol range")],
            FeatureSet::new(&[1, 2, 3], &[2]).expect("static feature set"),
            nonce_for_binding(binding, b"initiator"),
        ),
        CoreSessionAuthRole::Responder => SessionAuthHello::new(
            *fixture.responder_credential(),
            vec![ProtocolRange::new(1, 1, 2).expect("static protocol range")],
            FeatureSet::new(&[2, 3, 4], &[3]).expect("static feature set"),
            nonce_for_binding(binding, b"responder"),
        ),
    }
}

fn fixture_authority() -> Result<OwnerAuthorityState, EvalError> {
    let owner_id = OwnerId::from_bytes([0x40; 32]);
    let root_key = SigningKey::from_secret_bytes([0x41; 32]);
    let issuer_key = SigningKey::from_secret_bytes([0x42; 32]);
    let root = OwnerRootRecord::new(owner_id, &root_key, 0);
    let delegation = AuthorityDelegation::issue(
        owner_id,
        AuthorityRole::DeviceSigning,
        &issuer_key,
        0,
        &root_key,
    );
    let mut authority = OwnerAuthorityState::new(root);
    authority
        .accept_delegation(delegation)
        .map_err(|_| EvalError::Control)?;
    Ok(authority)
}

fn proof_message(proof: crosslab_core::SessionAuthProof) -> SessionAuthProofMessage {
    let role = match proof.role() {
        CoreSessionAuthRole::Initiator => WireSessionAuthRole::Initiator,
        CoreSessionAuthRole::Responder => WireSessionAuthRole::Responder,
    };
    SessionAuthProofMessage::new(role, proof.transcript_digest(), proof.signature())
}

async fn send_bootstrap(
    send: &mut SendStream,
    message: &SessionAuthBootstrapMessage,
) -> Result<(), EvalError> {
    let frame = encode_session_auth_bootstrap(message).map_err(|_| EvalError::Control)?;
    write_record(send, &frame, BOOTSTRAP_RECORD_MAX)
        .await
        .map_err(|_| EvalError::Control)
}

async fn receive_bootstrap(
    recv: &mut RecvStream,
) -> Result<SessionAuthBootstrapMessage, EvalError> {
    let frame = read_record(recv, BOOTSTRAP_RECORD_MAX, false)
        .await
        .map_err(|_| EvalError::Control)?;
    decode_session_auth_bootstrap(&frame).map_err(|_| EvalError::Control)
}

fn expect_hello(message: SessionAuthBootstrapMessage) -> Result<SessionAuthHello, EvalError> {
    match message {
        SessionAuthBootstrapMessage::Hello(hello) => Ok(hello),
        SessionAuthBootstrapMessage::Proof(_) => Err(EvalError::Control),
    }
}

fn expect_proof(
    message: SessionAuthBootstrapMessage,
) -> Result<SessionAuthProofMessage, EvalError> {
    match message {
        SessionAuthBootstrapMessage::Proof(proof) => Ok(proof),
        SessionAuthBootstrapMessage::Hello(_) => Err(EvalError::Control),
    }
}

fn nonce_for_binding(binding: &ChannelBinding, role: &[u8]) -> [u8; 32] {
    let mut input = Vec::with_capacity(binding.bytes().len() + role.len());
    input.extend_from_slice(binding.bytes());
    input.extend_from_slice(role);
    blake3_256(&input)
}
