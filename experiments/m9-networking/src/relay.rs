use std::net::{Ipv4Addr, SocketAddr};

use iroh::RelayUrl;
use iroh_relay::server::{RelayConfig, Server, ServerConfig};

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
