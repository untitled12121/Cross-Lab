use crosslab_core::{
    ChannelBinding, ConnectionMetadata, LogicalSession, SessionActivation, SessionAuthProof,
    SessionAuthRole as CoreSessionAuthRole, SessionAuthTranscriptV1, SessionError,
    SessionHandshakeSide, SessionState, TransportConnection, TransportSecurityClass,
};
use crosslab_crypto::{SigningKey, blake3_256};
use crosslab_identity::{
    AuthorityDelegation, AuthorityRole, DeviceCredential, DeviceId, OwnerAuthorityState, OwnerId,
    OwnerRootRecord,
};
use crosslab_policy::{NetworkClass, PairingTrustTransition, TransitionId, TrustRecord};
use crosslab_protocol::{
    FeatureSet, FrameLimit, ProtocolRange, SessionAuthBootstrapMessage, SessionAuthHello,
    SessionAuthProofMessage, SessionAuthRole as WireSessionAuthRole, decode_session_auth_bootstrap,
    encode_session_auth_bootstrap, negotiate_features, negotiate_protocol_version,
};
use iroh::endpoint::{RecvStream, SendStream};

use crate::{
    candidate::{
        connection::IrohTransportConnection,
        control::{ControlBridge, ReservedControlStreams, reserve_control_stream},
        endpoint::{DirectPair, direct_pair},
        record::{read_record, write_record},
        runtime::CandidateConfig,
    },
    error::EvalError,
};

const BOOTSTRAP_RECORD_MAX: usize = FrameLimit::BootstrapHello.max_payload_len() + 4;

fn establish_trust(
    credential: &DeviceCredential,
    authority: &OwnerAuthorityState,
    issuer_key: &SigningKey,
    transition_byte: u8,
) -> TrustRecord {
    let initial_credential = DeviceCredential::issue_for_public_key(
        credential.owner_id(),
        credential.device_id(),
        credential.device_public_key(),
        0,
        authority,
        issuer_key,
    )
    .unwrap();
    let transition = PairingTrustTransition::issue(
        &initial_credential,
        TransitionId::from_bytes([transition_byte; 32]),
        [transition_byte.wrapping_add(1); 32],
        authority,
        issuer_key,
    )
    .unwrap();
    let mut trust = transition
        .establish(&initial_credential, authority)
        .unwrap();

    for epoch in 1..=credential.credential_epoch() {
        let successor = DeviceCredential::issue_for_public_key(
            credential.owner_id(),
            credential.device_id(),
            credential.device_public_key(),
            epoch,
            authority,
            issuer_key,
        )
        .unwrap();
        let mut transition_id = [transition_byte; 32];
        transition_id[..8].copy_from_slice(&epoch.to_be_bytes());
        trust
            .accept_successor_credential(
                &successor,
                authority,
                TransitionId::from_bytes(transition_id),
            )
            .unwrap();
    }

    trust
}

pub struct AuthFixture {
    owner_id: OwnerId,
    authority: OwnerAuthorityState,
    initiator_key: SigningKey,
    responder_key: SigningKey,
    initiator_credential: DeviceCredential,
    responder_credential: DeviceCredential,
    initiator_trust: TrustRecord,
    responder_trust: TrustRecord,
}

impl AuthFixture {
    pub fn new() -> Self {
        let owner_id = OwnerId::from_bytes([0x40; 32]);
        let root_key = SigningKey::from_secret_bytes([0x41; 32]);
        let root = OwnerRootRecord::new(owner_id, &root_key, 0);
        let issuer_key = SigningKey::from_secret_bytes([0x42; 32]);
        let delegation = AuthorityDelegation::issue(
            owner_id,
            AuthorityRole::DeviceSigning,
            &issuer_key,
            0,
            &root_key,
        );
        let mut authority = OwnerAuthorityState::new(root);
        authority.accept_delegation(delegation).unwrap();

        let initiator_key = SigningKey::from_secret_bytes([0x43; 32]);
        let responder_key = SigningKey::from_secret_bytes([0x44; 32]);
        let initiator_credential = DeviceCredential::issue(
            owner_id,
            DeviceId::from_bytes([0x45; 32]),
            &initiator_key,
            2,
            &authority,
            &issuer_key,
        )
        .unwrap();
        let responder_credential = DeviceCredential::issue(
            owner_id,
            DeviceId::from_bytes([0x46; 32]),
            &responder_key,
            5,
            &authority,
            &issuer_key,
        )
        .unwrap();
        let initiator_trust = establish_trust(&initiator_credential, &authority, &issuer_key, 0x47);
        let responder_trust = establish_trust(&responder_credential, &authority, &issuer_key, 0x48);

        Self {
            owner_id,
            authority,
            initiator_key,
            responder_key,
            initiator_credential,
            responder_credential,
            initiator_trust,
            responder_trust,
        }
    }

    fn initiator_ranges() -> Vec<ProtocolRange> {
        vec![ProtocolRange::new(1, 0, 3).unwrap()]
    }

    fn responder_ranges() -> Vec<ProtocolRange> {
        vec![ProtocolRange::new(1, 1, 2).unwrap()]
    }

    fn initiator_features() -> FeatureSet {
        FeatureSet::new(&[1, 2, 3], &[2]).unwrap()
    }

    fn responder_features() -> FeatureSet {
        FeatureSet::new(&[2, 3, 4], &[3]).unwrap()
    }
}

impl Default for AuthFixture {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Clone, Copy)]
pub struct ReplayProof {
    proof: SessionAuthProof,
    message: SessionAuthProofMessage,
}

#[derive(Clone, Copy)]
pub enum AuthAttempt {
    Normal,
    WrongBinding,
    ReplayInitiator(ReplayProof),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BootstrapError {
    SessionNotActive,
}

pub struct BootstrapIrohPair {
    direct: DirectPair,
    client_send: SendStream,
    client_recv: RecvStream,
    server_send: SendStream,
    server_recv: RecvStream,
    client_session: LogicalSession,
    server_session: LogicalSession,
    config: CandidateConfig,
}

impl BootstrapIrohPair {
    pub fn client_session(&self) -> &LogicalSession {
        &self.client_session
    }

    pub fn server_session(&self) -> &LogicalSession {
        &self.server_session
    }

    pub fn try_promote(&self) -> Result<(), BootstrapError> {
        require_active(&self.client_session, &self.server_session)
    }

    pub async fn shutdown(self) {
        let Self { direct, .. } = self;
        direct.shutdown().await;
    }
}

pub struct AuthenticatedIrohPair {
    direct: DirectPair,
    client_transport: IrohTransportConnection,
    server_transport: IrohTransportConnection,
    client_session: LogicalSession,
    server_session: LogicalSession,
    initiator_proof: ReplayProof,
    network_class: NetworkClass,
}

impl AuthenticatedIrohPair {
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

    pub const fn initiator_replay_proof(&self) -> ReplayProof {
        self.initiator_proof
    }

    pub const fn network_class(&self) -> NetworkClass {
        self.network_class
    }

    pub async fn shutdown(self) {
        let Self {
            direct,
            client_transport,
            server_transport,
            ..
        } = self;
        tokio::join!(client_transport.shutdown(), server_transport.shutdown());
        direct.shutdown().await;
    }
}

#[derive(Debug)]
pub struct RejectedAuthentication {
    client_session: LogicalSession,
    server_session: LogicalSession,
    error: SessionError,
}

impl RejectedAuthentication {
    pub fn client_session(&self) -> &LogicalSession {
        &self.client_session
    }

    pub fn server_session(&self) -> &LogicalSession {
        &self.server_session
    }

    pub const fn error(&self) -> SessionError {
        self.error
    }
}

pub async fn bootstrap_direct_pair(fixture: &AuthFixture) -> Result<BootstrapIrohPair, EvalError> {
    let direct = direct_pair().await?;
    let ReservedControlStreams {
        mut client_send,
        client_recv,
        server_send,
        server_recv,
    } = reserve_control_stream(&direct).await?;
    let client_binding = direct.client_binding();
    let initiator_hello = SessionAuthHello::new(
        fixture.initiator_credential,
        AuthFixture::initiator_ranges(),
        AuthFixture::initiator_features(),
        nonce_for_binding(client_binding, b"initiator"),
    );
    send_bootstrap(
        &mut client_send,
        &SessionAuthBootstrapMessage::Hello(initiator_hello),
    )
    .await?;

    Ok(BootstrapIrohPair {
        direct,
        client_send,
        client_recv,
        server_send,
        server_recv,
        client_session: LogicalSession::new(),
        server_session: LogicalSession::new(),
        config: CandidateConfig::default(),
    })
}

pub async fn authenticate_direct_pair(
    fixture: &AuthFixture,
    attempt: AuthAttempt,
) -> Result<AuthenticatedIrohPair, RejectedAuthentication> {
    let mut bootstrap = bootstrap_direct_pair(fixture)
        .await
        .expect("deterministic Iroh authentication bootstrap");

    let initiator_hello = expect_hello(
        receive_bootstrap(&mut bootstrap.server_recv)
            .await
            .expect("initiator hello"),
    );
    let responder_hello = SessionAuthHello::new(
        fixture.responder_credential,
        AuthFixture::responder_ranges(),
        AuthFixture::responder_features(),
        nonce_for_binding(bootstrap.direct.server_binding(), b"responder"),
    );
    send_bootstrap(
        &mut bootstrap.server_send,
        &SessionAuthBootstrapMessage::Hello(responder_hello.clone()),
    )
    .await
    .expect("responder hello");
    let responder_hello = expect_hello(
        receive_bootstrap(&mut bootstrap.client_recv)
            .await
            .expect("responder hello"),
    );

    let actual_transcript = transcript_for(
        fixture,
        &initiator_hello,
        &responder_hello,
        bootstrap.direct.client_binding(),
    );
    let proof_transcript = match attempt {
        AuthAttempt::WrongBinding => {
            let binding = bootstrap.direct.client_binding();
            let mut bytes = binding.bytes().to_vec();
            bytes[0] ^= 0xFF;
            let wrong_binding = ChannelBinding::new(binding.profile_id(), bytes);
            transcript_for(fixture, &initiator_hello, &responder_hello, &wrong_binding)
        }
        AuthAttempt::Normal | AuthAttempt::ReplayInitiator(_) => actual_transcript.clone(),
    };

    let generated_initiator_proof = proof_transcript
        .create_proof(CoreSessionAuthRole::Initiator, &fixture.initiator_key)
        .unwrap();
    let responder_proof = proof_transcript
        .create_proof(CoreSessionAuthRole::Responder, &fixture.responder_key)
        .unwrap();
    let generated_replay = ReplayProof {
        proof: generated_initiator_proof,
        message: proof_message(generated_initiator_proof),
    };
    let initiator_replay = match attempt {
        AuthAttempt::ReplayInitiator(replay) => replay,
        AuthAttempt::Normal | AuthAttempt::WrongBinding => generated_replay,
    };
    let responder_message = proof_message(responder_proof);

    send_bootstrap(
        &mut bootstrap.client_send,
        &SessionAuthBootstrapMessage::Proof(initiator_replay.message),
    )
    .await
    .expect("initiator proof");
    send_bootstrap(
        &mut bootstrap.server_send,
        &SessionAuthBootstrapMessage::Proof(responder_message),
    )
    .await
    .expect("responder proof");
    assert_eq!(
        expect_proof(
            receive_bootstrap(&mut bootstrap.server_recv)
                .await
                .expect("initiator proof receipt"),
        ),
        initiator_replay.message
    );
    assert_eq!(
        expect_proof(
            receive_bootstrap(&mut bootstrap.client_recv)
                .await
                .expect("responder proof receipt"),
        ),
        responder_message
    );

    let initiator_credential = initiator_hello.device_credential();
    let responder_credential = responder_hello.device_credential();
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

    let client_result = bootstrap
        .client_session
        .authenticate(SessionActivation::new(
            &fixture.authority,
            initiator_side,
            responder_side,
            CoreSessionAuthRole::Initiator,
            &fixture.responder_trust,
            initiator_hello.nonce(),
            responder_hello.nonce(),
            bootstrap.direct.client_binding(),
            TransportSecurityClass::AuthenticatedConfidentialChannel,
            &initiator_replay.proof,
            &responder_proof,
        ));
    let server_result = bootstrap
        .server_session
        .authenticate(SessionActivation::new(
            &fixture.authority,
            initiator_side,
            responder_side,
            CoreSessionAuthRole::Responder,
            &fixture.initiator_trust,
            initiator_hello.nonce(),
            responder_hello.nonce(),
            bootstrap.direct.server_binding(),
            TransportSecurityClass::AuthenticatedConfidentialChannel,
            &initiator_replay.proof,
            &responder_proof,
        ));

    match (client_result, server_result) {
        (Ok(()), Ok(())) => Ok(promote_authenticated(bootstrap, initiator_replay)
            .expect("active authenticated sessions must be promotable")),
        (Err(client_error), Err(server_error)) => {
            assert_eq!(client_error, server_error);
            let BootstrapIrohPair {
                direct,
                client_session,
                server_session,
                ..
            } = bootstrap;
            direct.shutdown().await;
            Err(RejectedAuthentication {
                client_session,
                server_session,
                error: client_error,
            })
        }
        _ => {
            bootstrap.shutdown().await;
            panic!("peers disagreed on session authentication outcome")
        }
    }
}

fn promote_authenticated(
    bootstrap: BootstrapIrohPair,
    initiator_proof: ReplayProof,
) -> Result<AuthenticatedIrohPair, BootstrapError> {
    require_active(&bootstrap.client_session, &bootstrap.server_session)?;
    let BootstrapIrohPair {
        direct,
        client_send,
        client_recv,
        server_send,
        server_recv,
        client_session,
        server_session,
        config,
    } = bootstrap;
    let client_binding = direct.client_binding().clone();
    let server_binding = direct.server_binding().clone();
    let client_control = ControlBridge::new(
        direct.client_connection().clone(),
        client_send,
        client_recv,
        config,
    );
    let server_control = ControlBridge::new(
        direct.server_connection().clone(),
        server_send,
        server_recv,
        config,
    );
    let client_transport = IrohTransportConnection::new(
        client_control,
        client_binding,
        ConnectionMetadata::new(None, None, None),
        config,
    );
    let server_transport = IrohTransportConnection::new(
        server_control,
        server_binding,
        ConnectionMetadata::new(None, None, None),
        config,
    );

    Ok(AuthenticatedIrohPair {
        direct,
        client_transport,
        server_transport,
        client_session,
        server_session,
        initiator_proof,
        network_class: NetworkClass::Remote,
    })
}

fn require_active(client: &LogicalSession, server: &LogicalSession) -> Result<(), BootstrapError> {
    if client.state() == SessionState::Active && server.state() == SessionState::Active {
        Ok(())
    } else {
        Err(BootstrapError::SessionNotActive)
    }
}

fn transcript_for(
    fixture: &AuthFixture,
    initiator: &SessionAuthHello,
    responder: &SessionAuthHello,
    binding: &ChannelBinding,
) -> SessionAuthTranscriptV1 {
    let initiator_credential = initiator.device_credential();
    let responder_credential = responder.device_credential();
    let protocol =
        negotiate_protocol_version(initiator.protocol_ranges(), responder.protocol_ranges())
            .unwrap();
    let features = negotiate_features(initiator.features(), responder.features()).unwrap();

    SessionAuthTranscriptV1::new(
        fixture.owner_id,
        &initiator_credential,
        initiator.nonce(),
        &responder_credential,
        responder.nonce(),
        protocol,
        &features,
        binding.profile_id().as_bytes(),
        binding.bytes(),
    )
    .unwrap()
}

fn proof_message(proof: SessionAuthProof) -> SessionAuthProofMessage {
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

fn expect_hello(message: SessionAuthBootstrapMessage) -> SessionAuthHello {
    match message {
        SessionAuthBootstrapMessage::Hello(hello) => hello,
        SessionAuthBootstrapMessage::Proof(_) => panic!("expected session-auth hello"),
    }
}

fn expect_proof(message: SessionAuthBootstrapMessage) -> SessionAuthProofMessage {
    match message {
        SessionAuthBootstrapMessage::Proof(proof) => proof,
        SessionAuthBootstrapMessage::Hello(_) => panic!("expected session-auth proof"),
    }
}

fn nonce_for_binding(binding: &ChannelBinding, role: &[u8]) -> [u8; 32] {
    let mut input = Vec::with_capacity(binding.bytes().len() + role.len());
    input.extend_from_slice(binding.bytes());
    input.extend_from_slice(role);
    blake3_256(&input)
}
