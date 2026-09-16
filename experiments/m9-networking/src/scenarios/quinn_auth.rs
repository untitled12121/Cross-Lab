use std::time::Instant;

use crosslab_core::{
    ChannelBinding, LogicalSession, SessionActivation, SessionAuthProof,
    SessionAuthRole as CoreSessionAuthRole, SessionAuthTranscriptV1, SessionHandshakeSide,
    SessionState, TransportSecurityClass,
};
use crosslab_crypto::{SigningKey, blake3_256};
use crosslab_identity::{
    AuthorityDelegation, AuthorityRole, DeviceCredential, DeviceId, OwnerAuthorityState, OwnerId,
    OwnerRootRecord,
};
use crosslab_policy::{PairingTrustTransition, TransitionId, TrustRecord};
use crosslab_protocol::{
    FeatureSet, FrameLimit, ProtocolRange, SessionAuthBootstrapMessage, SessionAuthHello,
    SessionAuthProofMessage, SessionAuthRole as WireSessionAuthRole, decode_session_auth_bootstrap,
    encode_session_auth_bootstrap, negotiate_features, negotiate_protocol_version,
};
use quinn::{Connection, RecvStream, SendStream};

use crate::error::EvalError;

const BOOTSTRAP_RECORD_MAX: usize = FrameLimit::BootstrapHello.max_payload_len() + 4;
const CHANNEL_BINDING_PROFILE: &str = "quic-tls-exporter-v1";
const EXPORTER_LABEL: &[u8] = b"EXPORTER-Cross-Lab-QUIC-Channel-Binding-v1";
const EXPORTER_CONTEXT: &[u8] = b"crosslab.quic.transport.v1";
const CHANNEL_BINDING_BYTES: usize = 32;

pub(crate) async fn measure_session_auth(
    client: &Connection,
    server: &Connection,
) -> Result<u128, EvalError> {
    let client_binding = derive_channel_binding(client)?;
    let server_binding = derive_channel_binding(server)?;
    if client_binding != server_binding {
        return Err(EvalError::ChannelBinding);
    }

    let fixture = AuthFixture::new();
    let started = Instant::now();
    let (mut client_send, mut client_recv) =
        client.open_bi().await.map_err(|_| EvalError::Control)?;
    let initiator_hello = SessionAuthHello::new(
        fixture.initiator_credential,
        AuthFixture::initiator_ranges(),
        AuthFixture::initiator_features(),
        nonce_for_binding(&client_binding, b"initiator"),
    );
    send_bootstrap(
        &mut client_send,
        &SessionAuthBootstrapMessage::Hello(initiator_hello),
    )
    .await?;

    let (mut server_send, mut server_recv) =
        server.accept_bi().await.map_err(|_| EvalError::Control)?;
    let initiator_hello = expect_hello(receive_bootstrap(&mut server_recv).await?);
    let responder_hello = SessionAuthHello::new(
        fixture.responder_credential,
        AuthFixture::responder_ranges(),
        AuthFixture::responder_features(),
        nonce_for_binding(&server_binding, b"responder"),
    );
    send_bootstrap(
        &mut server_send,
        &SessionAuthBootstrapMessage::Hello(responder_hello.clone()),
    )
    .await?;
    let responder_hello = expect_hello(receive_bootstrap(&mut client_recv).await?);

    let transcript = transcript_for(
        &fixture,
        &initiator_hello,
        &responder_hello,
        &client_binding,
    );
    let initiator_proof = transcript
        .create_proof(CoreSessionAuthRole::Initiator, &fixture.initiator_key)
        .map_err(|_| EvalError::Control)?;
    let responder_proof = transcript
        .create_proof(CoreSessionAuthRole::Responder, &fixture.responder_key)
        .map_err(|_| EvalError::Control)?;
    let initiator_message = proof_message(initiator_proof);
    let responder_message = proof_message(responder_proof);

    send_bootstrap(
        &mut client_send,
        &SessionAuthBootstrapMessage::Proof(initiator_message),
    )
    .await?;
    send_bootstrap(
        &mut server_send,
        &SessionAuthBootstrapMessage::Proof(responder_message),
    )
    .await?;
    if expect_proof(receive_bootstrap(&mut server_recv).await?) != initiator_message
        || expect_proof(receive_bootstrap(&mut client_recv).await?) != responder_message
    {
        return Err(EvalError::Control);
    }

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
    let mut client_session = LogicalSession::new();
    let mut server_session = LogicalSession::new();

    client_session
        .authenticate(SessionActivation::new(
            &fixture.authority,
            initiator_side,
            responder_side,
            CoreSessionAuthRole::Initiator,
            &fixture.responder_trust,
            initiator_hello.nonce(),
            responder_hello.nonce(),
            &client_binding,
            TransportSecurityClass::AuthenticatedConfidentialChannel,
            &initiator_proof,
            &responder_proof,
        ))
        .map_err(|_| EvalError::Control)?;
    server_session
        .authenticate(SessionActivation::new(
            &fixture.authority,
            initiator_side,
            responder_side,
            CoreSessionAuthRole::Responder,
            &fixture.initiator_trust,
            initiator_hello.nonce(),
            responder_hello.nonce(),
            &server_binding,
            TransportSecurityClass::AuthenticatedConfidentialChannel,
            &initiator_proof,
            &responder_proof,
        ))
        .map_err(|_| EvalError::Control)?;

    if client_session.state() != SessionState::Active
        || server_session.state() != SessionState::Active
    {
        return Err(EvalError::Control);
    }

    client_send.finish().map_err(|_| EvalError::Control)?;
    server_send.finish().map_err(|_| EvalError::Control)?;
    Ok(started.elapsed().as_micros())
}

struct AuthFixture {
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
    fn new() -> Self {
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

fn derive_channel_binding(connection: &Connection) -> Result<ChannelBinding, EvalError> {
    let mut bytes = [0_u8; CHANNEL_BINDING_BYTES];
    connection
        .export_keying_material(&mut bytes, EXPORTER_LABEL, EXPORTER_CONTEXT)
        .map_err(|_| EvalError::ChannelBinding)?;
    Ok(ChannelBinding::new(CHANNEL_BINDING_PROFILE, bytes.to_vec()))
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
    write_record(send, &frame).await
}

async fn receive_bootstrap(
    recv: &mut RecvStream,
) -> Result<SessionAuthBootstrapMessage, EvalError> {
    let frame = read_record(recv).await?;
    decode_session_auth_bootstrap(&frame).map_err(|_| EvalError::Control)
}

async fn write_record(send: &mut SendStream, bytes: &[u8]) -> Result<(), EvalError> {
    if bytes.is_empty() || bytes.len() > BOOTSTRAP_RECORD_MAX {
        return Err(EvalError::Control);
    }
    let length = u32::try_from(bytes.len()).map_err(|_| EvalError::Control)?;
    send.write_all(&length.to_be_bytes())
        .await
        .map_err(|_| EvalError::Control)?;
    send.write_all(bytes).await.map_err(|_| EvalError::Control)
}

async fn read_record(recv: &mut RecvStream) -> Result<Vec<u8>, EvalError> {
    let mut prefix = [0_u8; 4];
    recv.read_exact(&mut prefix)
        .await
        .map_err(|_| EvalError::Control)?;
    let declared = u32::from_be_bytes(prefix) as usize;
    if declared == 0 || declared > BOOTSTRAP_RECORD_MAX {
        return Err(EvalError::Control);
    }
    let mut bytes = vec![0_u8; declared];
    recv.read_exact(&mut bytes)
        .await
        .map_err(|_| EvalError::Control)?;
    Ok(bytes)
}

fn expect_hello(message: SessionAuthBootstrapMessage) -> SessionAuthHello {
    match message {
        SessionAuthBootstrapMessage::Hello(hello) => hello,
        SessionAuthBootstrapMessage::Proof(_) => unreachable!("expected session-auth hello"),
    }
}

fn expect_proof(message: SessionAuthBootstrapMessage) -> SessionAuthProofMessage {
    match message {
        SessionAuthBootstrapMessage::Proof(proof) => proof,
        SessionAuthBootstrapMessage::Hello(_) => unreachable!("expected session-auth proof"),
    }
}

fn nonce_for_binding(binding: &ChannelBinding, role: &[u8]) -> [u8; 32] {
    let mut input = Vec::with_capacity(binding.bytes().len() + role.len());
    input.extend_from_slice(binding.bytes());
    input.extend_from_slice(role);
    blake3_256(&input)
}
