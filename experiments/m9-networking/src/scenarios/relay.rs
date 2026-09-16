use std::{num::NonZeroUsize, time::Duration};

use crosslab_core::{ChannelBinding, ControlReceiveError, TransportConnection};
use crosslab_policy::{NetworkClass, PolicyState, SessionId, TrustRecord};
use crosslab_protocol::{CapabilityAdvertisement, StreamId};
use crosslab_sim::{
    node::{NodeError, NodeEvent, SimNode},
    stream::SimStreamRuntime,
};
use futures_util::StreamExt;
use iroh::{
    Endpoint, EndpointAddr, RelayMode,
    endpoint::{Connection, presets},
};
use tokio::time::{sleep, timeout};

use crate::{
    candidate::endpoint::{ConnectedPair, M9_ALPN, connect_endpoints},
    error::EvalError,
    relay::OwnerRelay,
    scenarios::{
        auth::{
            AuthFixture, AuthenticatedIrohPair, AuthenticatedIrohParts, authenticate_connected_pair,
        },
        lifecycle::{
            eventually_accept, eventually_finished, eventually_receive, prepare_stream_authority,
        },
    },
};

const RELAY_READY_TIMEOUT: Duration = Duration::from_secs(10);
const RELAY_READY_POLL: Duration = Duration::from_millis(10);

pub struct RelayObservedPair {
    relay: OwnerRelay,
    pair: AuthenticatedIrohPair,
    observed_connection: Connection,
}

impl RelayObservedPair {
    pub const fn network_class(&self) -> NetworkClass {
        self.pair.network_class()
    }

    pub fn channel_binding(&self) -> &ChannelBinding {
        self.pair.client_transport().channel_binding()
    }

    pub fn session_id(&self) -> SessionId {
        self.pair
            .client_session()
            .context()
            .expect("authenticated relay pair must have a session context")
            .session_id()
    }

    pub fn client_transport(&self) -> &dyn TransportConnection {
        self.pair.client_transport()
    }

    pub fn server_transport(&self) -> &dyn TransportConnection {
        self.pair.server_transport()
    }

    pub async fn wait_for_direct_path(&mut self, wait: Duration) -> Result<(), EvalError> {
        wait_for_direct_path_on(&self.observed_connection, wait).await
    }

    pub async fn shutdown(self) -> Result<(), EvalError> {
        let Self { relay, pair, .. } = self;
        pair.shutdown().await;
        relay.shutdown().await
    }
}

pub async fn relay_only_pair(fixture: &AuthFixture) -> Result<RelayObservedPair, EvalError> {
    relay_pair(fixture, false).await
}

pub async fn relay_then_direct_pair(fixture: &AuthFixture) -> Result<RelayObservedPair, EvalError> {
    relay_pair(fixture, true).await
}

async fn relay_pair(
    fixture: &AuthFixture,
    enable_ip_transports: bool,
) -> Result<RelayObservedPair, EvalError> {
    let relay = OwnerRelay::start().await?;
    let connected = connect_relay_pair(relay.url(), enable_ip_transports).await?;
    let observed_connection = connected.client_connection().clone();
    let pair = authenticate_connected_pair(fixture, connected)
        .await
        .map_err(|_| EvalError::Connect)?;

    Ok(RelayObservedPair {
        relay,
        pair,
        observed_connection,
    })
}

pub struct RelaySequenceEvidence {
    pub before_send_sequence: Option<u64>,
    pub before_receive_sequence: Option<u64>,
    pub after_send_sequence: Option<u64>,
    pub after_receive_sequence: Option<u64>,
}

pub async fn exercise_relay_sequence_invariant(
    fixture: &AuthFixture,
) -> Result<RelaySequenceEvidence, EvalError> {
    let RelayObservedPair {
        relay,
        pair,
        observed_connection,
    } = relay_then_direct_pair(fixture).await?;
    let AuthenticatedIrohParts {
        direct,
        client_transport,
        server_transport,
        client_session,
        server_session,
    } = pair.into_parts();
    let capacity = NonZeroUsize::new(16).expect("nonzero Task 7 state capacity");
    let mut client = SimNode::new(
        client_session,
        &client_transport,
        PolicyState::new(),
        Vec::new(),
        NetworkClass::Remote,
        capacity,
    )
    .map_err(|_| EvalError::Control)?;
    let mut server = SimNode::new(
        server_session,
        &server_transport,
        PolicyState::new(),
        Vec::new(),
        NetworkClass::Remote,
        capacity,
    )
    .map_err(|_| EvalError::Control)?;
    let peer_trust = fixture.initiator_trust();

    send_empty_capabilities(&mut client)?;
    wait_for_capability_update(&mut server, &peer_trust).await?;
    let before_send_sequence = client.next_send_sequence();
    let before_receive_sequence = server.expected_receive_sequence();

    wait_for_direct_path_on(&observed_connection, RELAY_READY_TIMEOUT).await?;

    send_empty_capabilities(&mut client)?;
    wait_for_capability_update(&mut server, &peer_trust).await?;
    let after_send_sequence = client.next_send_sequence();
    let after_receive_sequence = server.expected_receive_sequence();

    client.shutdown();
    server.shutdown();
    drop((client, server));
    tokio::join!(client_transport.shutdown(), server_transport.shutdown());
    direct.shutdown().await;
    relay.shutdown().await?;

    Ok(RelaySequenceEvidence {
        before_send_sequence,
        before_receive_sequence,
        after_send_sequence,
        after_receive_sequence,
    })
}

pub async fn exercise_relay_operation_invariant(
    fixture: &AuthFixture,
) -> Result<Vec<u8>, EvalError> {
    let RelayObservedPair {
        relay,
        pair,
        observed_connection,
    } = relay_then_direct_pair(fixture).await?;
    let AuthenticatedIrohParts {
        direct,
        client_transport,
        server_transport,
        mut client_session,
        mut server_session,
    } = pair.into_parts();
    let authority = prepare_stream_authority(fixture, &mut client_session, &mut server_session);
    let capacity = NonZeroUsize::new(8).expect("nonzero Task 7 stream capacity");
    let mut sender = SimStreamRuntime::new(client_session, &client_transport, capacity)
        .map_err(|_| EvalError::Bulk)?;
    let mut receiver = SimStreamRuntime::new(server_session, &server_transport, capacity)
        .map_err(|_| EvalError::Bulk)?;
    receiver
        .register_operation(authority.operation().clone())
        .map_err(|_| EvalError::Bulk)?;

    wait_for_direct_path_on(&observed_connection, RELAY_READY_TIMEOUT).await?;

    let open = authority.open(StreamId::from_bytes([0xd1; 16]));
    let mut send = sender.open_uni(&open).map_err(|_| EvalError::Bulk)?;
    let stream_id = eventually_accept(
        &mut receiver,
        15,
        authority.peer_trust(),
        authority.policy(),
    )
    .await
    .map_err(|_| EvalError::Bulk)?;
    let expected = b"path-operation".to_vec();
    send.try_send_chunk(expected.clone())
        .map_err(|_| EvalError::Bulk)?;
    let payload = eventually_receive(&mut receiver, stream_id).await;
    send.finish();
    eventually_finished(&mut receiver, stream_id).await;

    sender.shutdown();
    receiver.shutdown();
    drop((sender, receiver));
    tokio::join!(client_transport.shutdown(), server_transport.shutdown());
    direct.shutdown().await;
    relay.shutdown().await?;

    if payload != expected {
        return Err(EvalError::Bulk);
    }
    Ok(payload)
}

fn send_empty_capabilities(node: &mut SimNode<'_>) -> Result<(), EvalError> {
    let advertisement = CapabilityAdvertisement::new(Vec::new()).map_err(|_| EvalError::Control)?;
    node.send_capability_advertisement(advertisement)
        .map_err(|_| EvalError::Control)
}

async fn wait_for_capability_update(
    node: &mut SimNode<'_>,
    peer_trust: &TrustRecord,
) -> Result<(), EvalError> {
    timeout(RELAY_READY_TIMEOUT, async {
        loop {
            match node.receive_one(peer_trust) {
                Ok(NodeEvent::CapabilitiesUpdated) => return Ok(()),
                Err(NodeError::Receive(ControlReceiveError::Empty)) => {
                    tokio::task::yield_now().await;
                }
                Ok(_) | Err(_) => return Err(EvalError::Control),
            }
        }
    })
    .await
    .map_err(|_| EvalError::Timeout)?
}

async fn wait_for_direct_path_on(connection: &Connection, wait: Duration) -> Result<(), EvalError> {
    let mut paths = connection.paths_stream();
    timeout(wait, async {
        while let Some(paths) = paths.next().await {
            if paths.iter().any(|path| path.remote_addr().is_ip()) {
                return Ok(());
            }
        }
        Err(EvalError::Connect)
    })
    .await
    .map_err(|_| EvalError::Timeout)?
}

pub(crate) async fn connect_relay_pair(
    relay_url: &iroh::RelayUrl,
    enable_ip_transports: bool,
) -> Result<ConnectedPair, EvalError> {
    let client_endpoint = relay_endpoint(relay_url, enable_ip_transports).await?;
    let server_endpoint = relay_endpoint(relay_url, enable_ip_transports).await?;
    wait_for_relay(&client_endpoint, relay_url).await?;
    wait_for_relay(&server_endpoint, relay_url).await?;

    let server_addr = EndpointAddr::new(server_endpoint.id()).with_relay_url(relay_url.clone());
    connect_endpoints(client_endpoint, server_endpoint, server_addr).await
}

async fn relay_endpoint(
    relay_url: &iroh::RelayUrl,
    enable_ip_transports: bool,
) -> Result<Endpoint, EvalError> {
    let builder = Endpoint::builder(presets::Minimal)
        .relay_mode(RelayMode::custom([relay_url.clone()]))
        .alpns(vec![M9_ALPN.to_vec()]);
    let builder = if enable_ip_transports {
        builder
            .bind_addr("127.0.0.1:0")
            .map_err(|_| EvalError::Setup)?
    } else {
        builder.clear_ip_transports()
    };

    builder.bind().await.map_err(|_| EvalError::Setup)
}

async fn wait_for_relay(endpoint: &Endpoint, relay_url: &iroh::RelayUrl) -> Result<(), EvalError> {
    timeout(RELAY_READY_TIMEOUT, async {
        loop {
            if endpoint.addr().relay_urls().any(|url| url == relay_url) {
                return;
            }
            sleep(RELAY_READY_POLL).await;
        }
    })
    .await
    .map_err(|_| EvalError::Timeout)
}
