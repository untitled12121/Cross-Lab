use std::time::Duration;

use crosslab_core::{ChannelBinding, TransportConnection};
use crosslab_policy::{NetworkClass, SessionId};
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
    scenarios::auth::{AuthFixture, AuthenticatedIrohPair, authenticate_connected_pair},
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
        let mut paths = self.observed_connection.paths_stream();
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

async fn connect_relay_pair(
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
