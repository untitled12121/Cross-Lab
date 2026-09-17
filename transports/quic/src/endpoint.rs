use std::{net::SocketAddr, sync::Arc};

use quinn::{ClientConfig, Endpoint, ServerConfig, TransportConfig, VarInt};
use rustls::{
    RootCertStore,
    pki_types::{CertificateDer, PrivatePkcs8KeyDer},
};

use crate::{
    QuicTransportConfig,
    session::{
        AuthenticatedQuicSession, QuicSessionAuthConfig, QuicSessionError, QuicSessionTimeouts,
        accept_authenticated, connect_authenticated,
    },
};

pub struct QuicClientTlsConfig {
    trusted_roots_der: Vec<Vec<u8>>,
}

impl QuicClientTlsConfig {
    pub fn new(trusted_roots_der: Vec<Vec<u8>>) -> Self {
        Self { trusted_roots_der }
    }
}

pub struct QuicServerTlsConfig {
    certificate_chain_der: Vec<Vec<u8>>,
    private_key_pkcs8_der: Vec<u8>,
}

impl QuicServerTlsConfig {
    pub fn new(certificate_chain_der: Vec<Vec<u8>>, private_key_pkcs8_der: Vec<u8>) -> Self {
        Self {
            certificate_chain_der,
            private_key_pkcs8_der,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum QuicEndpointError {
    InvalidTlsMaterial,
    InvalidTransportConfig,
    Bind,
}

pub struct QuicClientEndpoint {
    endpoint: Endpoint,
    transport_config: QuicTransportConfig,
}

impl QuicClientEndpoint {
    pub fn bind(
        bind_addr: SocketAddr,
        tls: QuicClientTlsConfig,
        transport_config: QuicTransportConfig,
    ) -> Result<Self, QuicEndpointError> {
        if tls.trusted_roots_der.is_empty() {
            return Err(QuicEndpointError::InvalidTlsMaterial);
        }

        let mut roots = RootCertStore::empty();
        for certificate in tls.trusted_roots_der {
            roots
                .add(CertificateDer::from(certificate))
                .map_err(|_| QuicEndpointError::InvalidTlsMaterial)?;
        }

        let mut client_config = ClientConfig::with_root_certificates(Arc::new(roots))
            .map_err(|_| QuicEndpointError::InvalidTlsMaterial)?;
        client_config.transport_config(build_transport_config(transport_config)?);

        let mut endpoint = Endpoint::client(bind_addr).map_err(|_| QuicEndpointError::Bind)?;
        endpoint.set_default_client_config(client_config);
        Ok(Self {
            endpoint,
            transport_config,
        })
    }

    pub fn local_addr(&self) -> std::io::Result<SocketAddr> {
        self.endpoint.local_addr()
    }

    pub async fn connect_authenticated(
        &self,
        remote_addr: SocketAddr,
        server_name: &str,
        auth: &QuicSessionAuthConfig<'_>,
        timeouts: QuicSessionTimeouts,
    ) -> Result<AuthenticatedQuicSession, QuicSessionError> {
        connect_authenticated(
            &self.endpoint,
            remote_addr,
            server_name,
            auth,
            timeouts,
            self.transport_config,
        )
        .await
    }
}

pub struct QuicServerEndpoint {
    endpoint: Endpoint,
    transport_config: QuicTransportConfig,
}

impl QuicServerEndpoint {
    pub fn bind(
        bind_addr: SocketAddr,
        tls: QuicServerTlsConfig,
        transport_config: QuicTransportConfig,
    ) -> Result<Self, QuicEndpointError> {
        if tls.certificate_chain_der.is_empty() || tls.private_key_pkcs8_der.is_empty() {
            return Err(QuicEndpointError::InvalidTlsMaterial);
        }

        let certificate_chain = tls
            .certificate_chain_der
            .into_iter()
            .map(CertificateDer::from)
            .collect();
        let private_key = PrivatePkcs8KeyDer::from(tls.private_key_pkcs8_der);
        let mut server_config =
            ServerConfig::with_single_cert(certificate_chain, private_key.into())
                .map_err(|_| QuicEndpointError::InvalidTlsMaterial)?;
        server_config.transport_config(build_transport_config(transport_config)?);

        let endpoint =
            Endpoint::server(server_config, bind_addr).map_err(|_| QuicEndpointError::Bind)?;
        Ok(Self {
            endpoint,
            transport_config,
        })
    }

    pub fn local_addr(&self) -> std::io::Result<SocketAddr> {
        self.endpoint.local_addr()
    }

    pub async fn accept_authenticated(
        &self,
        auth: &QuicSessionAuthConfig<'_>,
        timeouts: QuicSessionTimeouts,
    ) -> Result<AuthenticatedQuicSession, QuicSessionError> {
        accept_authenticated(&self.endpoint, auth, timeouts, self.transport_config).await
    }
}

fn build_transport_config(
    config: QuicTransportConfig,
) -> Result<Arc<TransportConfig>, QuicEndpointError> {
    let stream_receive_window = VarInt::from_u64(config.stream_receive_window())
        .map_err(|_| QuicEndpointError::InvalidTransportConfig)?;
    let connection_receive_window = VarInt::from_u64(config.connection_receive_window())
        .map_err(|_| QuicEndpointError::InvalidTransportConfig)?;
    let idle_timeout = config
        .idle_timeout()
        .try_into()
        .map_err(|_| QuicEndpointError::InvalidTransportConfig)?;

    let mut transport = TransportConfig::default();
    transport
        .max_concurrent_uni_streams(VarInt::from_u32(config.max_concurrent_remote_uni_streams()))
        .max_concurrent_bidi_streams(VarInt::from_u32(config.max_concurrent_remote_bi_streams()))
        .stream_receive_window(stream_receive_window)
        .receive_window(connection_receive_window)
        .max_idle_timeout(Some(idle_timeout));
    Ok(Arc::new(transport))
}
