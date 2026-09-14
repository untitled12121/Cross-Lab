use std::{
    net::{IpAddr, Ipv4Addr, SocketAddr},
    sync::Arc,
    time::Instant,
};

use quinn::{ClientConfig, Connection, Endpoint, ServerConfig, VarInt};
use rustls::{RootCertStore, pki_types::PrivatePkcs8KeyDer};
use tokio::time::timeout;

use crate::{
    config::EvalConfig,
    error::EvalError,
    metrics::{Measurement, MetricKind, Report, TransportKind},
};

const CONTROL_BYTES: usize = 32;
const CLOSE_CODE: VarInt = VarInt::from_u32(0);

struct RawLoopbackPair {
    client_endpoint: Endpoint,
    server_endpoint: Endpoint,
    client: Connection,
    server: Connection,
}

pub async fn run_quinn_loopback_sample(config: EvalConfig) -> Result<Report, EvalError> {
    let mut report = Report::new(Vec::with_capacity(config.samples() * 4));

    for sample in 0..config.samples() {
        let measurements = timeout(
            config.timeout(),
            run_sample(sample, config.bulk_payload_bytes()),
        )
        .await
        .map_err(|_| EvalError::Timeout)??;
        report.extend(measurements);
    }

    Ok(report)
}

async fn run_sample(
    sample: usize,
    bulk_payload_bytes: usize,
) -> Result<Vec<Measurement>, EvalError> {
    let (pair, protected_connect_us) = loopback_connection_pair().await?;
    let control_rtt_us = measure_control_rtt(&pair).await?;
    let bulk_bytes_per_second = measure_bulk_transfer(&pair, bulk_payload_bytes).await?;
    let shutdown_us = pair.shutdown().await;

    Ok(vec![
        Measurement::new(
            TransportKind::Quinn,
            MetricKind::ProtectedConnectMicros,
            sample,
            protected_connect_us,
        ),
        Measurement::new(
            TransportKind::Quinn,
            MetricKind::ControlRttMicros,
            sample,
            control_rtt_us,
        ),
        Measurement::new(
            TransportKind::Quinn,
            MetricKind::BulkBytesPerSecond,
            sample,
            bulk_bytes_per_second,
        ),
        Measurement::new(
            TransportKind::Quinn,
            MetricKind::ShutdownMicros,
            sample,
            shutdown_us,
        ),
    ])
}

async fn measure_control_rtt(pair: &RawLoopbackPair) -> Result<u128, EvalError> {
    let ping = [0xA5; CONTROL_BYTES];
    let started = Instant::now();

    let client = async {
        let (mut send, mut recv) = pair
            .client
            .open_bi()
            .await
            .map_err(|_| EvalError::Control)?;
        send.write_all(&ping)
            .await
            .map_err(|_| EvalError::Control)?;
        send.finish().map_err(|_| EvalError::Control)?;

        let mut echo = [0; CONTROL_BYTES];
        recv.read_exact(&mut echo)
            .await
            .map_err(|_| EvalError::Control)?;
        if echo != ping {
            return Err(EvalError::Control);
        }
        Ok(())
    };

    let server = async {
        let (mut send, mut recv) = pair
            .server
            .accept_bi()
            .await
            .map_err(|_| EvalError::Control)?;
        let mut request = [0; CONTROL_BYTES];
        recv.read_exact(&mut request)
            .await
            .map_err(|_| EvalError::Control)?;
        send.write_all(&request)
            .await
            .map_err(|_| EvalError::Control)?;
        send.finish().map_err(|_| EvalError::Control)?;
        Ok(())
    };

    let (client_result, server_result) = tokio::join!(client, server);
    client_result?;
    server_result?;
    Ok(started.elapsed().as_micros())
}

async fn measure_bulk_transfer(
    pair: &RawLoopbackPair,
    bulk_payload_bytes: usize,
) -> Result<u128, EvalError> {
    let payload = vec![0x5A; bulk_payload_bytes];
    let started = Instant::now();

    let client = async {
        let mut send = pair.client.open_uni().await.map_err(|_| EvalError::Bulk)?;
        send.write_all(&payload)
            .await
            .map_err(|_| EvalError::Bulk)?;
        send.finish().map_err(|_| EvalError::Bulk)?;
        Ok(())
    };

    let server = async {
        let mut recv = pair
            .server
            .accept_uni()
            .await
            .map_err(|_| EvalError::Bulk)?;
        let mut received = vec![0; bulk_payload_bytes];
        recv.read_exact(&mut received)
            .await
            .map_err(|_| EvalError::Bulk)?;
        if received.iter().any(|byte| *byte != 0x5A) {
            return Err(EvalError::Bulk);
        }
        Ok(())
    };

    let (client_result, server_result) = tokio::join!(client, server);
    client_result?;
    server_result?;

    let elapsed_nanos = started.elapsed().as_nanos().max(1);
    Ok((bulk_payload_bytes as u128 * 1_000_000_000) / elapsed_nanos)
}

async fn loopback_connection_pair() -> Result<(RawLoopbackPair, u128), EvalError> {
    let certified = rcgen::generate_simple_self_signed(vec!["localhost".to_owned()])
        .map_err(|_| EvalError::Setup)?;
    let certificate = certified.cert.der().clone();
    let key = PrivatePkcs8KeyDer::from(certified.signing_key.serialize_der());
    let server_config = ServerConfig::with_single_cert(vec![certificate.clone()], key.into())
        .map_err(|_| EvalError::Setup)?;
    let server_endpoint = Endpoint::server(
        server_config,
        SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), 0),
    )
    .map_err(|_| EvalError::Setup)?;
    let server_addr = server_endpoint.local_addr().map_err(|_| EvalError::Setup)?;

    let mut roots = RootCertStore::empty();
    roots.add(certificate).map_err(|_| EvalError::Setup)?;
    let mut client_endpoint = Endpoint::client(SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), 0))
        .map_err(|_| EvalError::Setup)?;
    let client_config =
        ClientConfig::with_root_certificates(Arc::new(roots)).map_err(|_| EvalError::Setup)?;
    client_endpoint.set_default_client_config(client_config);

    let started = Instant::now();
    let client_connecting = client_endpoint
        .connect(server_addr, "localhost")
        .map_err(|_| EvalError::Connect)?;
    let server_incoming = server_endpoint.accept().await.ok_or(EvalError::Connect)?;
    let (client, server) = tokio::join!(client_connecting, server_incoming);
    let client = client.map_err(|_| EvalError::Connect)?;
    let server = server.map_err(|_| EvalError::Connect)?;
    let protected_connect_us = started.elapsed().as_micros();

    Ok((
        RawLoopbackPair {
            client_endpoint,
            server_endpoint,
            client,
            server,
        },
        protected_connect_us,
    ))
}

impl RawLoopbackPair {
    async fn shutdown(self) -> u128 {
        let started = Instant::now();
        self.client.close(CLOSE_CODE, b"M9 Quinn baseline complete");
        self.server.close(CLOSE_CODE, b"M9 Quinn baseline complete");
        let _ = tokio::join!(self.client.closed(), self.server.closed());
        self.client_endpoint.wait_idle().await;
        self.server_endpoint.wait_idle().await;
        started.elapsed().as_micros()
    }
}
