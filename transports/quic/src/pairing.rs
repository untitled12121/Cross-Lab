use std::{
    fmt,
    net::SocketAddr,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    time::Duration,
};

use crosslab_protocol::{
    FrameLimit, ProductPairingMessage, ProtocolWireError, decode_product_pairing,
    encode_product_pairing,
};
use quinn::{
    ClientConfig, Connection, Endpoint, RecvStream, SendStream, ServerConfig, TransportConfig,
    VarInt,
    crypto::rustls::{QuicClientConfig, QuicServerConfig},
};
use rustls::{
    DigitallySignedStruct, SignatureScheme,
    client::danger::{HandshakeSignatureValid, ServerCertVerified, ServerCertVerifier},
    pki_types::{CertificateDer, PrivatePkcs8KeyDer, ServerName, UnixTime},
};
use tokio::time::timeout;

use crate::record::{RecordError, read_record, write_record};

pub const PRODUCT_PAIRING_ALPN_V1: &[u8] = b"crosslab-pairing-v1";
pub const PRODUCT_PAIRING_SERVER_NAME: &str = "pairing.crosslab.local";

const RECORD_MAX: usize = FrameLimit::BootstrapHello.max_payload_len() + 4;
const PAIRING_CLOSE_CODE: VarInt = VarInt::from_u32(4);
const MAX_REMOTE_BI_STREAMS: u32 = 1;
const DEFAULT_IDLE_TIMEOUT: Duration = Duration::from_secs(30);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ProductPairingQuicTimeouts {
    connect: Duration,
    message: Duration,
}

impl ProductPairingQuicTimeouts {
    pub const fn new(connect: Duration, message: Duration) -> Self {
        Self { connect, message }
    }

    pub const fn connect(self) -> Duration {
        self.connect
    }

    pub const fn message(self) -> Duration {
        self.message
    }
}

impl Default for ProductPairingQuicTimeouts {
    fn default() -> Self {
        Self {
            connect: Duration::from_secs(10),
            message: Duration::from_secs(15),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProductPairingQuicError {
    Bind,
    Tls,
    Connect,
    Accept,
    AlreadyAccepted,
    Stream,
    Timeout,
    Record,
    Protocol(ProtocolWireError),
}

impl fmt::Display for ProductPairingQuicError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Bind => formatter.write_str("product pairing QUIC bind failed"),
            Self::Tls => formatter.write_str("product pairing TLS setup failed"),
            Self::Connect => formatter.write_str("product pairing QUIC connection failed"),
            Self::Accept => formatter.write_str("product pairing QUIC accept failed"),
            Self::AlreadyAccepted => {
                formatter.write_str("product pairing invitation already accepted a joiner")
            }
            Self::Stream => formatter.write_str("product pairing QUIC stream setup failed"),
            Self::Timeout => formatter.write_str("product pairing QUIC operation timed out"),
            Self::Record => formatter.write_str("product pairing QUIC record failed"),
            Self::Protocol(error) => fmt::Display::fmt(error, formatter),
        }
    }
}

impl std::error::Error for ProductPairingQuicError {}

impl From<ProtocolWireError> for ProductPairingQuicError {
    fn from(error: ProtocolWireError) -> Self {
        Self::Protocol(error)
    }
}

pub struct ProductPairingQuicServer {
    endpoint: Endpoint,
    timeouts: ProductPairingQuicTimeouts,
    accepted: AtomicBool,
}

impl ProductPairingQuicServer {
    pub fn bind(
        bind_addr: SocketAddr,
        timeouts: ProductPairingQuicTimeouts,
    ) -> Result<Self, ProductPairingQuicError> {
        let certified =
            rcgen::generate_simple_self_signed(vec![PRODUCT_PAIRING_SERVER_NAME.into()])
                .map_err(|_| ProductPairingQuicError::Tls)?;
        let certificate = CertificateDer::from(certified.cert.der().as_ref().to_vec());
        let private_key = PrivatePkcs8KeyDer::from(certified.signing_key.serialize_der());

        let mut tls =
            rustls::ServerConfig::builder_with_protocol_versions(&[&rustls::version::TLS13])
                .with_no_client_auth()
                .with_single_cert(vec![certificate], private_key.into())
                .map_err(|_| ProductPairingQuicError::Tls)?;
        tls.alpn_protocols = vec![PRODUCT_PAIRING_ALPN_V1.to_vec()];
        tls.max_early_data_size = 0;

        let crypto = QuicServerConfig::try_from(tls).map_err(|_| ProductPairingQuicError::Tls)?;
        let mut config = ServerConfig::with_crypto(Arc::new(crypto));
        config.transport = pairing_transport_config()?;

        let endpoint =
            Endpoint::server(config, bind_addr).map_err(|_| ProductPairingQuicError::Bind)?;
        Ok(Self {
            endpoint,
            timeouts,
            accepted: AtomicBool::new(false),
        })
    }

    pub fn local_addr(&self) -> Result<SocketAddr, ProductPairingQuicError> {
        self.endpoint
            .local_addr()
            .map_err(|_| ProductPairingQuicError::Bind)
    }

    pub async fn accept(&self) -> Result<ProductPairingQuicChannel, ProductPairingQuicError> {
        if self
            .accepted
            .compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
            .is_err()
        {
            return Err(ProductPairingQuicError::AlreadyAccepted);
        }

        let result = self.accept_inner().await;
        if result.is_err() {
            self.accepted.store(false, Ordering::Release);
        }
        result
    }

    async fn accept_inner(&self) -> Result<ProductPairingQuicChannel, ProductPairingQuicError> {
        let incoming = timeout(self.timeouts.connect, self.endpoint.accept())
            .await
            .map_err(|_| ProductPairingQuicError::Timeout)?
            .ok_or(ProductPairingQuicError::Accept)?;
        let connection = timeout(self.timeouts.connect, incoming)
            .await
            .map_err(|_| ProductPairingQuicError::Timeout)?
            .map_err(|_| ProductPairingQuicError::Accept)?;

        let (send, recv) = timeout(self.timeouts.connect, connection.accept_bi())
            .await
            .map_err(|_| ProductPairingQuicError::Timeout)?
            .map_err(|_| ProductPairingQuicError::Stream)?;

        Ok(ProductPairingQuicChannel {
            _endpoint: Some(self.endpoint.clone()),
            connection,
            send,
            recv,
            message_timeout: self.timeouts.message,
        })
    }

    pub fn close(&self) {
        self.endpoint
            .close(PAIRING_CLOSE_CODE, b"product pairing listener closed");
    }
}

impl Drop for ProductPairingQuicServer {
    fn drop(&mut self) {
        self.close();
    }
}

pub struct ProductPairingQuicClient;

impl ProductPairingQuicClient {
    pub async fn connect(
        bind_addr: SocketAddr,
        remote_addr: SocketAddr,
        timeouts: ProductPairingQuicTimeouts,
    ) -> Result<ProductPairingQuicChannel, ProductPairingQuicError> {
        let mut tls =
            rustls::ClientConfig::builder_with_protocol_versions(&[&rustls::version::TLS13])
                .dangerous()
                .with_custom_certificate_verifier(PairingServerCertVerifier::new())
                .with_no_client_auth();
        tls.alpn_protocols = vec![PRODUCT_PAIRING_ALPN_V1.to_vec()];
        tls.enable_early_data = false;

        let crypto = QuicClientConfig::try_from(tls).map_err(|_| ProductPairingQuicError::Tls)?;
        let mut config = ClientConfig::new(Arc::new(crypto));
        config.transport_config(pairing_transport_config()?);

        let mut endpoint =
            Endpoint::client(bind_addr).map_err(|_| ProductPairingQuicError::Bind)?;
        endpoint.set_default_client_config(config);
        let connecting = endpoint
            .connect(remote_addr, PRODUCT_PAIRING_SERVER_NAME)
            .map_err(|_| ProductPairingQuicError::Connect)?;
        let connection = timeout(timeouts.connect, connecting)
            .await
            .map_err(|_| ProductPairingQuicError::Timeout)?
            .map_err(|_| ProductPairingQuicError::Connect)?;
        let (send, recv) = timeout(timeouts.connect, connection.open_bi())
            .await
            .map_err(|_| ProductPairingQuicError::Timeout)?
            .map_err(|_| ProductPairingQuicError::Stream)?;

        Ok(ProductPairingQuicChannel {
            _endpoint: Some(endpoint),
            connection,
            send,
            recv,
            message_timeout: timeouts.message,
        })
    }
}

pub struct ProductPairingQuicChannel {
    _endpoint: Option<Endpoint>,
    connection: Connection,
    send: SendStream,
    recv: RecvStream,
    message_timeout: Duration,
}

impl ProductPairingQuicChannel {
    pub fn remote_addr(&self) -> SocketAddr {
        self.connection.remote_address()
    }

    pub async fn send(
        &mut self,
        message: &ProductPairingMessage,
    ) -> Result<(), ProductPairingQuicError> {
        let frame = encode_product_pairing(message)?;
        timeout(
            self.message_timeout,
            write_record(&mut self.send, &frame, RECORD_MAX),
        )
        .await
        .map_err(|_| ProductPairingQuicError::Timeout)?
        .map_err(record_error)
    }

    pub async fn receive(&mut self) -> Result<ProductPairingMessage, ProductPairingQuicError> {
        let frame = timeout(
            self.message_timeout,
            read_record(&mut self.recv, RECORD_MAX, false),
        )
        .await
        .map_err(|_| ProductPairingQuicError::Timeout)?
        .map_err(record_error)?;
        decode_product_pairing(&frame).map_err(Into::into)
    }

    pub fn close(&self) {
        self.connection
            .close(PAIRING_CLOSE_CODE, b"product pairing channel closed");
    }
}

impl Drop for ProductPairingQuicChannel {
    fn drop(&mut self) {
        self.close();
    }
}

#[derive(Debug)]
struct PairingServerCertVerifier(Arc<rustls::crypto::CryptoProvider>);

impl PairingServerCertVerifier {
    fn new() -> Arc<Self> {
        Arc::new(Self(Arc::new(rustls::crypto::ring::default_provider())))
    }
}

impl ServerCertVerifier for PairingServerCertVerifier {
    fn verify_server_cert(
        &self,
        _end_entity: &CertificateDer<'_>,
        _intermediates: &[CertificateDer<'_>],
        _server_name: &ServerName<'_>,
        _ocsp: &[u8],
        _now: UnixTime,
    ) -> Result<ServerCertVerified, rustls::Error> {
        // ADR-0016: the invitation secret authenticates Cross-Lab pairing.
        // This certificate is ephemeral channel protection, not device identity.
        Ok(ServerCertVerified::assertion())
    }

    fn verify_tls12_signature(
        &self,
        message: &[u8],
        cert: &CertificateDer<'_>,
        signature: &DigitallySignedStruct,
    ) -> Result<HandshakeSignatureValid, rustls::Error> {
        rustls::crypto::verify_tls12_signature(
            message,
            cert,
            signature,
            &self.0.signature_verification_algorithms,
        )
    }

    fn verify_tls13_signature(
        &self,
        message: &[u8],
        cert: &CertificateDer<'_>,
        signature: &DigitallySignedStruct,
    ) -> Result<HandshakeSignatureValid, rustls::Error> {
        rustls::crypto::verify_tls13_signature(
            message,
            cert,
            signature,
            &self.0.signature_verification_algorithms,
        )
    }

    fn supported_verify_schemes(&self) -> Vec<SignatureScheme> {
        self.0.signature_verification_algorithms.supported_schemes()
    }
}

fn pairing_transport_config() -> Result<Arc<TransportConfig>, ProductPairingQuicError> {
    let mut transport = TransportConfig::default();
    let idle_timeout = DEFAULT_IDLE_TIMEOUT
        .try_into()
        .map_err(|_| ProductPairingQuicError::Tls)?;
    transport
        .max_concurrent_uni_streams(VarInt::from_u32(0))
        .max_concurrent_bidi_streams(VarInt::from_u32(MAX_REMOTE_BI_STREAMS))
        .max_idle_timeout(Some(idle_timeout));
    Ok(Arc::new(transport))
}

fn record_error(_: RecordError) -> ProductPairingQuicError {
    ProductPairingQuicError::Record
}
