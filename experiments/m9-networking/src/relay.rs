use std::net::{Ipv4Addr, SocketAddr};

use iroh::RelayUrl;
use iroh_relay::{
    defaults::DEFAULT_RELAY_QUIC_PORT,
    server::{QuicConfig, RelayConfig, Server, ServerConfig},
};

use crate::error::EvalError;

pub(crate) struct OwnerRelay {
    server: Server,
    url: RelayUrl,
}

impl OwnerRelay {
    pub(crate) async fn start() -> Result<Self, EvalError> {
        Self::start_on(SocketAddr::from((Ipv4Addr::LOCALHOST, 0))).await
    }

    pub(crate) async fn start_on(bind_addr: SocketAddr) -> Result<Self, EvalError> {
        let mut config = ServerConfig::default();
        config.relay = Some(RelayConfig::new(bind_addr));

        if !bind_addr.ip().is_loopback() {
            let (_, server_config) =
                iroh_relay::server::testing::self_signed_tls_certs_and_config();
            let mut quic =
                QuicConfig::new(SocketAddr::new(bind_addr.ip(), DEFAULT_RELAY_QUIC_PORT));
            quic.server_config = Some(server_config);
            config.quic = Some(quic);
        }

        let server = Server::spawn(config).await.map_err(|_| EvalError::Setup)?;
        let url = server.http_url().ok_or(EvalError::Setup)?;

        Ok(Self { server, url })
    }

    pub(crate) fn url(&self) -> &RelayUrl {
        &self.url
    }

    pub(crate) async fn shutdown(self) -> Result<(), EvalError> {
        self.server.shutdown().await.map_err(|_| EvalError::Setup)
    }
}

#[cfg(test)]
mod tests {
    use super::OwnerRelay;

    #[tokio::test]
    async fn routed_owner_relay_starts_quic_address_discovery() {
        let bind = "0.0.0.0:0".parse().expect("routed bind");
        let relay = OwnerRelay::start_on(bind).await.expect("owner relay");

        assert!(
            relay.server.quic_addr().is_some(),
            "routed owner relay must expose QUIC address discovery"
        );

        relay.shutdown().await.expect("owner relay shutdown");
    }
}
