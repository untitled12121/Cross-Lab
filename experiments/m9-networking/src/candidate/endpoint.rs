use crosslab_core::ChannelBinding;
use iroh::{Endpoint, RelayMode, endpoint::Connection, endpoint::presets};

use crate::{candidate::binding::derive_channel_binding, error::EvalError};

pub const M9_ALPN: &[u8] = b"crosslab-m9-networking-eval";

pub struct DirectPair {
    client_endpoint: Endpoint,
    server_endpoint: Endpoint,
    _client_connection: Connection,
    _server_connection: Connection,
    client_binding: ChannelBinding,
    server_binding: ChannelBinding,
}

impl DirectPair {
    pub fn client_binding(&self) -> &ChannelBinding {
        &self.client_binding
    }

    pub fn server_binding(&self) -> &ChannelBinding {
        &self.server_binding
    }

    pub async fn shutdown(self) {
        let Self {
            client_endpoint,
            server_endpoint,
            ..
        } = self;
        tokio::join!(client_endpoint.close(), server_endpoint.close());
    }
}

pub struct UnconnectedDirectPair {
    client: Endpoint,
    server: Endpoint,
}

impl UnconnectedDirectPair {
    pub fn client(&self) -> &Endpoint {
        &self.client
    }

    pub fn server(&self) -> &Endpoint {
        &self.server
    }

    pub async fn shutdown(self) {
        tokio::join!(self.client.close(), self.server.close());
    }
}

pub async fn direct_pair() -> Result<DirectPair, EvalError> {
    let client_endpoint = direct_endpoint().await?;
    let server_endpoint = direct_endpoint().await?;
    let server_addr = server_endpoint.addr();

    let client_connect = client_endpoint.connect(server_addr, M9_ALPN);
    let server_accept = async {
        let incoming = server_endpoint.accept().await.ok_or(EvalError::Connect)?;
        incoming.await.map_err(|_| EvalError::Connect)
    };
    let (client_connection, server_connection) = tokio::join!(client_connect, server_accept);
    let client_connection = client_connection.map_err(|_| EvalError::Connect)?;
    let server_connection = server_connection?;
    let client_binding = derive_channel_binding(&client_connection)?;
    let server_binding = derive_channel_binding(&server_connection)?;

    Ok(DirectPair {
        client_endpoint,
        server_endpoint,
        _client_connection: client_connection,
        _server_connection: server_connection,
        client_binding,
        server_binding,
    })
}

pub async fn unconnected_direct_endpoints() -> Result<UnconnectedDirectPair, EvalError> {
    Ok(UnconnectedDirectPair {
        client: direct_endpoint().await?,
        server: direct_endpoint().await?,
    })
}

async fn direct_endpoint() -> Result<Endpoint, EvalError> {
    Endpoint::builder(presets::Minimal)
        .relay_mode(RelayMode::Disabled)
        .alpns(vec![M9_ALPN.to_vec()])
        .clear_ip_transports()
        .bind_addr("127.0.0.1:0")
        .map_err(|_| EvalError::Setup)?
        .bind()
        .await
        .map_err(|_| EvalError::Setup)
}
