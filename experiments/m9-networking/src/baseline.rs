use std::{
    net::{IpAddr, Ipv4Addr, SocketAddr},
    sync::Arc,
    time::Instant,
};

use crosslab_core::{
    ControlReceiveError, StreamAcceptError, StreamReceiveError, StreamSendError,
    TransportConnection,
};
use quinn::{ClientConfig, Connection, Endpoint, ServerConfig, VarInt};
use rustls::{RootCertStore, pki_types::PrivatePkcs8KeyDer};
use tokio::{task::yield_now, time::timeout};

use crate::{
    candidate::{
        endpoint::{ConnectedPair, direct_pair},
        runtime::CandidateConfig,
    },
    command::Command,
    config::EvalConfig,
    error::EvalError,
    metrics::{Measurement, MetricKind, Report, TransportKind},
    relay::OwnerRelay,
    scenarios::{
        auth::{AuthFixture, authenticate_connected_pair},
        relay::connect_relay_pair,
    },
};

const CONTROL_BYTES: usize = 32;
const CLOSE_CODE: VarInt = VarInt::from_u32(0);
const BULK_OPENING: &[u8] = b"m9-benchmark-bulk";
const BULK_BYTE: u8 = 0x5A;

struct RawLoopbackPair {
    client_endpoint: Endpoint,
    server_endpoint: Endpoint,
    client: Connection,
    server: Connection,
}

pub async fn run_local(command: Command) -> Result<Report, EvalError> {
    match command {
        Command::LocalQuinn(config) => run_quinn_loopback_sample(config).await,
        Command::LocalIrohDirect(config) => run_iroh_direct_sample(config).await,
        Command::LocalIrohRelay(config) => run_iroh_relay_sample(config).await,
        Command::LocalAll(config) => {
            let mut report = run_quinn_loopback_sample(config).await?;
            report.append(run_iroh_direct_sample(config).await?);
            report.append(run_iroh_relay_sample(config).await?);
            Ok(report)
        }
    }
}

pub async fn run_quinn_loopback_sample(config: EvalConfig) -> Result<Report, EvalError> {
    let mut report = Report::for_run(config, Vec::with_capacity(config.samples() * 4));

    for sample in 0..config.samples() {
        let measurements = timeout(
            config.timeout(),
            run_quinn_sample(sample, config.bulk_payload_bytes()),
        )
        .await
        .map_err(|_| EvalError::Timeout)??;
        report.extend(measurements);
    }

    Ok(report)
}

pub async fn run_iroh_direct_sample(config: EvalConfig) -> Result<Report, EvalError> {
    let fixture = AuthFixture::new();
    let mut report = Report::for_run(config, Vec::with_capacity(config.samples() * 5));

    for sample in 0..config.samples() {
        let measurements = timeout(config.timeout(), async {
            let started = Instant::now();
            let connected = direct_pair().await?;
            let protected_connect_us = started.elapsed().as_micros();
            run_authenticated_iroh_sample(
                &fixture,
                connected,
                TransportKind::IrohDirect,
                sample,
                config.bulk_payload_bytes(),
                protected_connect_us,
            )
            .await
        })
        .await
        .map_err(|_| EvalError::Timeout)??;
        report.extend(measurements);
    }

    Ok(report)
}

pub async fn run_iroh_relay_sample(config: EvalConfig) -> Result<Report, EvalError> {
    let fixture = AuthFixture::new();
    let relay = OwnerRelay::start().await?;
    let result = async {
        let mut report = Report::for_run(config, Vec::with_capacity(config.samples() * 5));

        for sample in 0..config.samples() {
            let measurements = timeout(config.timeout(), async {
                let started = Instant::now();
                let connected = connect_relay_pair(relay.url(), false).await?;
                let protected_connect_us = started.elapsed().as_micros();
                run_authenticated_iroh_sample(
                    &fixture,
                    connected,
                    TransportKind::IrohRelay,
                    sample,
                    config.bulk_payload_bytes(),
                    protected_connect_us,
                )
                .await
            })
            .await
            .map_err(|_| EvalError::Timeout)??;
            report.extend(measurements);
        }

        Ok(report)
    }
    .await;
    let relay_shutdown = relay.shutdown().await;

    match result {
        Ok(report) => {
            relay_shutdown?;
            Ok(report)
        }
        Err(error) => Err(error),
    }
}

async fn run_quinn_sample(
    sample: usize,
    bulk_payload_bytes: usize,
) -> Result<Vec<Measurement>, EvalError> {
    let (pair, protected_connect_us) = loopback_connection_pair().await?;
    let control_rtt_us = measure_quinn_control_rtt(&pair).await?;
    let bulk_bytes_per_second = measure_quinn_bulk_transfer(&pair, bulk_payload_bytes).await?;
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

async fn run_authenticated_iroh_sample(
    fixture: &AuthFixture,
    connected: ConnectedPair,
    transport: TransportKind,
    sample: usize,
    bulk_payload_bytes: usize,
    protected_connect_us: u128,
) -> Result<Vec<Measurement>, EvalError> {
    let auth_started = Instant::now();
    let pair = authenticate_connected_pair(fixture, connected)
        .await
        .map_err(|_| EvalError::Connect)?;
    let session_auth_us = auth_started.elapsed().as_micros();

    let activity = async {
        let control_rtt_us =
            measure_transport_control_rtt(pair.client_transport(), pair.server_transport()).await?;
        let bulk_bytes_per_second = measure_transport_bulk_transfer(
            pair.client_transport(),
            pair.server_transport(),
            bulk_payload_bytes,
        )
        .await?;
        Ok::<_, EvalError>((control_rtt_us, bulk_bytes_per_second))
    }
    .await;

    let shutdown_started = Instant::now();
    pair.shutdown().await;
    let shutdown_us = shutdown_started.elapsed().as_micros();
    let (control_rtt_us, bulk_bytes_per_second) = activity?;

    Ok(vec![
        Measurement::new(
            transport,
            MetricKind::ProtectedConnectMicros,
            sample,
            protected_connect_us,
        ),
        Measurement::new(
            transport,
            MetricKind::SessionAuthMicros,
            sample,
            session_auth_us,
        ),
        Measurement::new(
            transport,
            MetricKind::ControlRttMicros,
            sample,
            control_rtt_us,
        ),
        Measurement::new(
            transport,
            MetricKind::BulkBytesPerSecond,
            sample,
            bulk_bytes_per_second,
        ),
        Measurement::new(
            transport,
            MetricKind::ShutdownMicros,
            sample,
            shutdown_us,
        ),
    ])
}

async fn measure_transport_control_rtt(
    client: &dyn TransportConnection,
    server: &dyn TransportConnection,
) -> Result<u128, EvalError> {
    let ping = vec![0xA5; CONTROL_BYTES];
    let started = Instant::now();

    client
        .try_send_control(ping.clone())
        .map_err(|_| EvalError::Control)?;
    let request = eventually_receive_control(server).await?;
    if request != ping {
        return Err(EvalError::Control);
    }
    server
        .try_send_control(request)
        .map_err(|_| EvalError::Control)?;
    let echo = eventually_receive_control(client).await?;
    if echo != ping {
        return Err(EvalError::Control);
    }

    Ok(started.elapsed().as_micros())
}

async fn eventually_receive_control(
    transport: &dyn TransportConnection,
) -> Result<Vec<u8>, EvalError> {
    loop {
        match transport.try_receive_control() {
            Ok(frame) => return Ok(frame),
            Err(ControlReceiveError::Empty) => yield_now().await,
            Err(ControlReceiveError::Closed) => return Err(EvalError::Control),
        }
    }
}

async fn measure_transport_bulk_transfer(
    client: &dyn TransportConnection,
    server: &dyn TransportConnection,
    bulk_payload_bytes: usize,
) -> Result<u128, EvalError> {
    let chunk_bytes = CandidateConfig::default().max_chunk_bytes();
    let started = Instant::now();

    let sender = async {
        let mut send = client
            .try_open_uni_stream(BULK_OPENING.to_vec())
            .map_err(|_| EvalError::Bulk)?;
        let mut remaining = bulk_payload_bytes;
        while remaining > 0 {
            let len = remaining.min(chunk_bytes);
            let mut chunk = vec![BULK_BYTE; len];
            loop {
                match send.try_send_chunk(chunk) {
                    Ok(()) => break,
                    Err(StreamSendError::Full(returned)) => {
                        chunk = returned;
                        yield_now().await;
                    }
                    Err(StreamSendError::TooLarge(_)) | Err(StreamSendError::Closed(_)) => {
                        return Err(EvalError::Bulk);
                    }
                }
            }
            remaining -= len;
        }
        send.finish();
        Ok::<_, EvalError>(())
    };

    let receiver = async {
        let incoming = loop {
            match server.try_accept_uni_stream() {
                Ok(stream) => break stream,
                Err(StreamAcceptError::Empty) => yield_now().await,
                Err(StreamAcceptError::Closed) => return Err(EvalError::Bulk),
            }
        };
        if incoming.opening_frame() != BULK_OPENING {
            return Err(EvalError::Bulk);
        }
        let (_, mut recv) = incoming.into_parts();
        let mut received = 0_usize;
        loop {
            match recv.try_receive_chunk() {
                Ok(chunk) => {
                    if chunk.iter().any(|byte| *byte != BULK_BYTE) {
                        return Err(EvalError::Bulk);
                    }
                    received = received.checked_add(chunk.len()).ok_or(EvalError::Bulk)?;
                    if received > bulk_payload_bytes {
                        return Err(EvalError::Bulk);
                    }
                }
                Err(StreamReceiveError::Empty) => yield_now().await,
                Err(StreamReceiveError::Finished) => {
                    if received == bulk_payload_bytes {
                        return Ok::<_, EvalError>(());
                    }
                    return Err(EvalError::Bulk);
                }
                Err(StreamReceiveError::Cancelled) => return Err(EvalError::Bulk),
            }
        }
    };

    let (sender_result, receiver_result) = tokio::join!(sender, receiver);
    sender_result?;
    receiver_result?;

    let elapsed_nanos = started.elapsed().as_nanos().max(1);
    Ok((bulk_payload_bytes as u128 * 1_000_000_000) / elapsed_nanos)
}

async fn measure_quinn_control_rtt(pair: &RawLoopbackPair) -> Result<u128, EvalError> {
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

async fn measure_quinn_bulk_transfer(
    pair: &RawLoopbackPair,
    bulk_payload_bytes: usize,
) -> Result<u128, EvalError> {
    let payload = vec![BULK_BYTE; bulk_payload_bytes];
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
        if received.iter().any(|byte| *byte != BULK_BYTE) {
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
