use core::fmt;
use std::{net::SocketAddr, sync::Arc};

use quinn::{
    ClientConfig, Endpoint, ServerConfig,
    crypto::rustls::{QuicClientConfig, QuicServerConfig},
};
use rustls::pki_types::{CertificateDer, PrivatePkcs8KeyDer};

use crate::{
    QuicTransportConfig,
    endpoint::build_transport_config,
    session::{
        AuthenticatedQuicSession, QuicSessionAuthConfig, QuicSessionError, QuicSessionTimeouts,
        accept_authenticated, connect_authenticated,
    },
    tls::ChannelOnlyServerCertVerifier,
};

pub const TRUSTED_SESSION_ALPN_V1: &[u8] = b"crosslab-session-v1";
pub const TRUSTED_SESSION_SERVER_NAME: &str = "session.crosslab.local";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TrustedSessionQuicError {
    Bind,
    Tls,
}

impl fmt::Display for TrustedSessionQuicError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::Bind => "trusted-session QUIC bind failed",
            Self::Tls => "trusted-session TLS setup failed",
        })
    }
}

impl std::error::Error for TrustedSessionQuicError {}

pub struct TrustedSessionQuicServer {
    endpoint: Endpoint,
    transport_config: QuicTransportConfig,
}

impl TrustedSessionQuicServer {
    pub fn bind(
        bind_addr: SocketAddr,
        transport_config: QuicTransportConfig,
    ) -> Result<Self, TrustedSessionQuicError> {
        let certified =
            rcgen::generate_simple_self_signed(vec![TRUSTED_SESSION_SERVER_NAME.into()])
                .map_err(|_| TrustedSessionQuicError::Tls)?;
        let certificate = CertificateDer::from(certified.cert.der().as_ref().to_vec());
        let private_key = PrivatePkcs8KeyDer::from(certified.signing_key.serialize_der());

        let mut tls =
            rustls::ServerConfig::builder_with_protocol_versions(&[&rustls::version::TLS13])
                .with_no_client_auth()
                .with_single_cert(vec![certificate], private_key.into())
                .map_err(|_| TrustedSessionQuicError::Tls)?;
        tls.alpn_protocols = vec![TRUSTED_SESSION_ALPN_V1.to_vec()];
        tls.max_early_data_size = 0;

        let crypto =
            QuicServerConfig::try_from(tls).map_err(|_| TrustedSessionQuicError::Tls)?;
        let mut config = ServerConfig::with_crypto(Arc::new(crypto));
        config.transport =
            build_transport_config(transport_config).map_err(|_| TrustedSessionQuicError::Tls)?;

        let endpoint =
            Endpoint::server(config, bind_addr).map_err(|_| TrustedSessionQuicError::Bind)?;
        Ok(Self {
            endpoint,
            transport_config,
        })
    }

    pub fn local_addr(&self) -> Result<SocketAddr, TrustedSessionQuicError> {
        self.endpoint
            .local_addr()
            .map_err(|_| TrustedSessionQuicError::Bind)
    }

    pub async fn accept_authenticated(
        &self,
        auth: &QuicSessionAuthConfig<'_>,
        timeouts: QuicSessionTimeouts,
    ) -> Result<AuthenticatedQuicSession, QuicSessionError> {
        accept_authenticated(&self.endpoint, auth, timeouts, self.transport_config).await
    }

    pub fn close(&self) {
        self.endpoint.close(0_u32.into(), b"trusted-session listener closed");
    }
}

impl Drop for TrustedSessionQuicServer {
    fn drop(&mut self) {
        self.close();
    }
}

pub struct TrustedSessionQuicClient {
    endpoint: Endpoint,
    transport_config: QuicTransportConfig,
}

impl TrustedSessionQuicClient {
    pub fn bind(
        bind_addr: SocketAddr,
        transport_config: QuicTransportConfig,
    ) -> Result<Self, TrustedSessionQuicError> {
        let mut tls =
            rustls::ClientConfig::builder_with_protocol_versions(&[&rustls::version::TLS13])
                .dangerous()
                .with_custom_certificate_verifier(ChannelOnlyServerCertVerifier::new())
                .with_no_client_auth();
        tls.alpn_protocols = vec![TRUSTED_SESSION_ALPN_V1.to_vec()];
        tls.enable_early_data = false;

        let crypto =
            QuicClientConfig::try_from(tls).map_err(|_| TrustedSessionQuicError::Tls)?;
        let mut config = ClientConfig::new(Arc::new(crypto));
        config.transport_config(
            build_transport_config(transport_config).map_err(|_| TrustedSessionQuicError::Tls)?,
        );

        let mut endpoint =
            Endpoint::client(bind_addr).map_err(|_| TrustedSessionQuicError::Bind)?;
        endpoint.set_default_client_config(config);
        Ok(Self {
            endpoint,
            transport_config,
        })
    }

    pub fn local_addr(&self) -> Result<SocketAddr, TrustedSessionQuicError> {
        self.endpoint
            .local_addr()
            .map_err(|_| TrustedSessionQuicError::Bind)
    }

    pub async fn connect_authenticated(
        &self,
        remote_addr: SocketAddr,
        auth: &QuicSessionAuthConfig<'_>,
        timeouts: QuicSessionTimeouts,
    ) -> Result<AuthenticatedQuicSession, QuicSessionError> {
        connect_authenticated(
            &self.endpoint,
            remote_addr,
            TRUSTED_SESSION_SERVER_NAME,
            auth,
            timeouts,
            self.transport_config,
        )
        .await
    }

    pub fn close(&self) {
        self.endpoint.close(0_u32.into(), b"trusted-session client closed");
    }
}

impl Drop for TrustedSessionQuicClient {
    fn drop(&mut self) {
        self.close();
    }
}
