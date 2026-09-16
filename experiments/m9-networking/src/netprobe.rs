use std::{
    fs,
    future,
    io::{self, Write},
    net::SocketAddr,
    path::{Path, PathBuf},
    time::Duration,
};

use crosslab_core::{
    ControlReceiveError, ControlSendError, StreamAcceptError, StreamOpenError, StreamReceiveError,
    StreamSendError, TransportConnection, TransportReceiveStream, TransportSendStream,
};
use futures_util::StreamExt;
use iroh::{Endpoint, EndpointAddr, EndpointId, RelayMode, RelayUrl, endpoint::presets};
use tokio::time::{sleep, timeout};

use crate::{
    candidate::{binding::derive_channel_binding, endpoint::M9_ALPN},
    error::EvalError,
    relay::OwnerRelay,
    scenarios::{
        auth::AuthFixture,
        split_auth::{authenticate_side, reserve_control_initiator, reserve_control_responder},
    },
};

const READY_TIMEOUT: Duration = Duration::from_secs(10);
const READY_POLL: Duration = Duration::from_millis(25);
const CONTROL_PROBE: &[u8] = b"crosslab-m9-control-probe";
const CONTROL_OK: &[u8] = b"crosslab-m9-control-ok";
const DATA_OPEN: &[u8] = b"crosslab-m9-data-v1";
const DATA_PROBE: &[u8] = b"crosslab-m9-data-probe";
const DATA_OK: &[u8] = b"crosslab-m9-data-ok";
const DIRECT_OK: &[u8] = b"crosslab-m9-direct-ok";
const RECONNECT_PROBE: &[u8] = b"crosslab-m9-reconnect-probe";
const RECONNECT_OK: &[u8] = b"crosslab-m9-reconnect-ok";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NetprobeRelayArgs {
    bind: SocketAddr,
}

impl NetprobeRelayArgs {
    pub const fn new(bind: SocketAddr) -> Self {
        Self { bind }
    }

    pub const fn bind(&self) -> SocketAddr {
        self.bind
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NetprobePeerArgs {
    rendezvous: PathBuf,
    relay_url: RelayUrl,
}

impl NetprobePeerArgs {
    pub fn new(rendezvous: PathBuf, relay_url: RelayUrl) -> Self {
        Self {
            rendezvous,
            relay_url,
        }
    }

    pub fn rendezvous(&self) -> &Path {
        &self.rendezvous
    }

    pub const fn relay_url(&self) -> &RelayUrl {
        &self.relay_url
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Rendezvous {
    endpoint_id: EndpointId,
    relay_url: Option<RelayUrl>,
    ip: Option<SocketAddr>,
}

impl Rendezvous {
    pub fn new(
        endpoint_id: EndpointId,
        relay_url: Option<RelayUrl>,
        ip: Option<SocketAddr>,
    ) -> Self {
        Self {
            endpoint_id,
            relay_url,
            ip,
        }
    }

    pub fn parse(text: &str) -> Result<Self, EvalError> {
        let mut lines = text.lines();
        let endpoint_id = parse_required(lines.next(), "endpoint_id")?
            .parse()
            .map_err(|_| EvalError::InvalidValue)?;
        let relay_url = parse_optional(lines.next(), "relay_url")?
            .map(|value| value.parse().map_err(|_| EvalError::InvalidValue))
            .transpose()?;
        let ip = parse_optional(lines.next(), "ip")?
            .map(|value| value.parse().map_err(|_| EvalError::InvalidValue))
            .transpose()?;

        if lines.next().is_some() {
            return Err(EvalError::InvalidValue);
        }

        Ok(Self::new(endpoint_id, relay_url, ip))
    }

    pub fn to_text(&self) -> String {
        let relay_url = self
            .relay_url
            .as_ref()
            .map_or_else(|| "-".to_owned(), ToString::to_string);
        let ip = self.ip.map_or_else(|| "-".to_owned(), |ip| ip.to_string());
        format!(
            "endpoint_id={}\nrelay_url={relay_url}\nip={ip}\n",
            self.endpoint_id
        )
    }

    pub fn endpoint_addr(&self) -> EndpointAddr {
        let mut addr = EndpointAddr::new(self.endpoint_id);
        if let Some(relay_url) = &self.relay_url {
            addr = addr.with_relay_url(relay_url.clone());
        }
        if let Some(ip) = self.ip {
            addr = addr.with_ip_addr(ip);
        }
        addr
    }
}

pub async fn run_relay(args: NetprobeRelayArgs) -> Result<(), EvalError> {
    let _relay = OwnerRelay::start_on(args.bind()).await?;
    future::pending::<()>().await;
    Ok(())
}

pub async fn run_server(args: NetprobePeerArgs) -> Result<(), EvalError> {
    let endpoint = peer_endpoint(args.relay_url()).await?;
    let rendezvous = Rendezvous::new(endpoint.id(), Some(args.relay_url().clone()), None);
    fs::write(args.rendezvous(), rendezvous.to_text()).map_err(|_| EvalError::Setup)?;
    let fixture = AuthFixture::new();

    let connection = accept_connection(&endpoint).await?;
    let binding = derive_channel_binding(&connection)?;
    let (send, recv) = reserve_control_responder(&connection).await?;
    let side = authenticate_side(
        &fixture,
        crosslab_core::SessionAuthRole::Responder,
        connection,
        binding,
        send,
        recv,
    )
    .await?;
    let transport = side.transport;

    expect_control(&transport, CONTROL_PROBE).await?;
    send_control(&transport, CONTROL_OK).await?;

    let incoming = accept_uni(&transport).await?;
    if incoming.opening_frame() != DATA_OPEN {
        transport.shutdown().await;
        endpoint.close().await;
        return Err(EvalError::Bulk);
    }
    let (_, mut stream) = incoming.into_parts();
    let payload = receive_chunk(stream.as_mut()).await?;
    if payload != DATA_PROBE {
        transport.shutdown().await;
        endpoint.close().await;
        return Err(EvalError::Bulk);
    }
    expect_finished(stream.as_mut()).await?;
    send_control(&transport, DATA_OK).await?;

    expect_control(&transport, DIRECT_OK).await?;
    transport.shutdown().await;

    let connection = accept_connection(&endpoint).await?;
    let observed_connection = connection.clone();
    let binding = derive_channel_binding(&connection)?;
    let (send, recv) = reserve_control_responder(&connection).await?;
    let side = authenticate_side(
        &fixture,
        crosslab_core::SessionAuthRole::Responder,
        connection,
        binding,
        send,
        recv,
    )
    .await?;
    let transport = side.transport;

    expect_control(&transport, RECONNECT_PROBE).await?;
    send_control(&transport, RECONNECT_OK).await?;
    timeout(READY_TIMEOUT, observed_connection.closed())
        .await
        .map_err(|_| EvalError::Timeout)?;

    transport.shutdown().await;
    endpoint.close().await;
    Ok(())
}

pub async fn run_client(args: NetprobePeerArgs) -> Result<String, EvalError> {
    let mut output = String::new();
    run_client_inner(args, |key, value| {
        output.push_str(key);
        output.push('\t');
        output.push_str(value);
        output.push('\n');
        Ok(())
    })
    .await?;
    Ok(output)
}

pub async fn run_client_streaming(args: NetprobePeerArgs) -> Result<(), EvalError> {
    let mut stdout = io::stdout();
    run_client_inner(args, |key, value| {
        writeln!(stdout, "{key}\t{value}").map_err(|_| EvalError::Setup)?;
        stdout.flush().map_err(|_| EvalError::Setup)
    })
    .await
}

async fn run_client_inner<F>(args: NetprobePeerArgs, mut emit: F) -> Result<(), EvalError>
where
    F: FnMut(&str, &str) -> Result<(), EvalError>,
{
    let rendezvous = wait_for_rendezvous(args.rendezvous()).await?;
    let endpoint = peer_endpoint(args.relay_url()).await?;
    let fixture = AuthFixture::new();

    let connection = connect_connection(&endpoint, &rendezvous).await?;
    let observed_connection = connection.clone();
    let binding = derive_channel_binding(&connection)?;
    let (send, recv) = reserve_control_initiator(&connection).await?;
    let side = authenticate_side(
        &fixture,
        crosslab_core::SessionAuthRole::Initiator,
        connection,
        binding,
        send,
        recv,
    )
    .await?;
    let transport = side.transport;
    let session = side.session;
    let binding_before = transport.channel_binding().clone();
    let session_before = session
        .context()
        .ok_or(EvalError::Control)?
        .session_id();

    send_control(&transport, CONTROL_PROBE).await?;
    expect_control(&transport, CONTROL_OK).await?;

    let mut stream = open_uni(&transport, DATA_OPEN).await?;
    send_chunk(stream.as_mut(), DATA_PROBE).await?;
    stream.finish();
    expect_control(&transport, DATA_OK).await?;

    emit("phase", "relay_verified")?;
    emit("network_class", "remote")?;
    emit("control_verified", "true")?;
    emit("data_verified", "true")?;

    wait_for_direct_path(&observed_connection).await?;
    let binding_unchanged = transport.channel_binding() == &binding_before;
    let session_unchanged = session
        .context()
        .is_some_and(|context| context.session_id() == session_before);

    send_control(&transport, DIRECT_OK).await?;
    timeout(READY_TIMEOUT, observed_connection.closed())
        .await
        .map_err(|_| EvalError::Timeout)?;

    emit("phase", "direct_verified")?;
    emit(
        "binding_unchanged",
        if binding_unchanged { "true" } else { "false" },
    )?;
    emit(
        "session_unchanged",
        if session_unchanged { "true" } else { "false" },
    )?;
    transport.shutdown().await;

    let connection = connect_connection(&endpoint, &rendezvous).await?;
    let binding = derive_channel_binding(&connection)?;
    let (send, recv) = reserve_control_initiator(&connection).await?;
    let side = authenticate_side(
        &fixture,
        crosslab_core::SessionAuthRole::Initiator,
        connection,
        binding,
        send,
        recv,
    )
    .await?;
    let transport = side.transport;
    let session = side.session;
    let binding_refreshed = transport.channel_binding() != &binding_before;
    let session_refreshed = session
        .context()
        .is_some_and(|context| context.session_id() != session_before);

    send_control(&transport, RECONNECT_PROBE).await?;
    expect_control(&transport, RECONNECT_OK).await?;

    emit("phase", "reconnect_verified")?;
    emit(
        "binding_refreshed",
        if binding_refreshed { "true" } else { "false" },
    )?;
    emit(
        "session_refreshed",
        if session_refreshed { "true" } else { "false" },
    )?;
    emit("control_after_reconnect", "true")?;

    transport.shutdown().await;
    endpoint.close().await;
    Ok(())
}

async fn peer_endpoint(relay_url: &RelayUrl) -> Result<Endpoint, EvalError> {
    let endpoint = Endpoint::builder(presets::Minimal)
        .relay_mode(RelayMode::custom([relay_url.clone()]))
        .alpns(vec![M9_ALPN.to_vec()])
        .bind_addr("0.0.0.0:0")
        .map_err(|_| EvalError::Setup)?
        .bind()
        .await
        .map_err(|_| EvalError::Setup)?;
    wait_for_relay(&endpoint, relay_url).await?;
    Ok(endpoint)
}

async fn accept_connection(endpoint: &Endpoint) -> Result<iroh::endpoint::Connection, EvalError> {
    let incoming = timeout(READY_TIMEOUT, endpoint.accept())
        .await
        .map_err(|_| EvalError::Timeout)?
        .ok_or(EvalError::Connect)?;
    timeout(READY_TIMEOUT, incoming)
        .await
        .map_err(|_| EvalError::Timeout)?
        .map_err(|_| EvalError::Connect)
}

async fn connect_connection(
    endpoint: &Endpoint,
    rendezvous: &Rendezvous,
) -> Result<iroh::endpoint::Connection, EvalError> {
    timeout(
        READY_TIMEOUT,
        endpoint.connect(rendezvous.endpoint_addr(), M9_ALPN),
    )
    .await
    .map_err(|_| EvalError::Timeout)?
    .map_err(|_| EvalError::Connect)
}

async fn wait_for_relay(endpoint: &Endpoint, relay_url: &RelayUrl) -> Result<(), EvalError> {
    timeout(READY_TIMEOUT, async {
        loop {
            if endpoint.addr().relay_urls().any(|url| url == relay_url) {
                return;
            }
            sleep(READY_POLL).await;
        }
    })
    .await
    .map_err(|_| EvalError::Timeout)
}

async fn wait_for_rendezvous(path: &Path) -> Result<Rendezvous, EvalError> {
    timeout(READY_TIMEOUT, async {
        loop {
            match fs::read_to_string(path) {
                Ok(text) => return Rendezvous::parse(&text),
                Err(error) if error.kind() == io::ErrorKind::NotFound => sleep(READY_POLL).await,
                Err(_) => return Err(EvalError::Setup),
            }
        }
    })
    .await
    .map_err(|_| EvalError::Timeout)?
}

async fn send_control(
    transport: &dyn TransportConnection,
    frame: &[u8],
) -> Result<(), EvalError> {
    let mut frame = frame.to_vec();
    timeout(READY_TIMEOUT, async {
        loop {
            match transport.try_send_control(frame) {
                Ok(()) => return Ok(()),
                Err(ControlSendError::Full(returned)) => {
                    frame = returned;
                    tokio::task::yield_now().await;
                }
                Err(ControlSendError::TooLarge(_) | ControlSendError::Closed(_)) => {
                    return Err(EvalError::Control);
                }
            }
        }
    })
    .await
    .map_err(|_| EvalError::Timeout)?
}

async fn expect_control(
    transport: &dyn TransportConnection,
    expected: &[u8],
) -> Result<(), EvalError> {
    let frame = timeout(READY_TIMEOUT, async {
        loop {
            match transport.try_receive_control() {
                Ok(frame) => return Ok(frame),
                Err(ControlReceiveError::Empty) => tokio::task::yield_now().await,
                Err(ControlReceiveError::Closed) => return Err(EvalError::Control),
            }
        }
    })
    .await
    .map_err(|_| EvalError::Timeout)??;

    if frame == expected {
        Ok(())
    } else {
        Err(EvalError::Control)
    }
}

async fn open_uni(
    transport: &dyn TransportConnection,
    opening: &[u8],
) -> Result<Box<dyn TransportSendStream>, EvalError> {
    let mut opening = opening.to_vec();
    timeout(READY_TIMEOUT, async {
        loop {
            match transport.try_open_uni_stream(opening) {
                Ok(stream) => return Ok(stream),
                Err(StreamOpenError::Full(returned)) => {
                    opening = returned;
                    tokio::task::yield_now().await;
                }
                Err(StreamOpenError::TooLarge(_) | StreamOpenError::Closed(_)) => {
                    return Err(EvalError::Bulk);
                }
            }
        }
    })
    .await
    .map_err(|_| EvalError::Timeout)?
}

async fn send_chunk(
    stream: &mut dyn TransportSendStream,
    chunk: &[u8],
) -> Result<(), EvalError> {
    let mut chunk = chunk.to_vec();
    timeout(READY_TIMEOUT, async {
        loop {
            match stream.try_send_chunk(chunk) {
                Ok(()) => return Ok(()),
                Err(StreamSendError::Full(returned)) => {
                    chunk = returned;
                    tokio::task::yield_now().await;
                }
                Err(StreamSendError::TooLarge(_) | StreamSendError::Closed(_)) => {
                    return Err(EvalError::Bulk);
                }
            }
        }
    })
    .await
    .map_err(|_| EvalError::Timeout)?
}

async fn accept_uni(
    transport: &dyn TransportConnection,
) -> Result<crosslab_core::IncomingUniStream, EvalError> {
    timeout(READY_TIMEOUT, async {
        loop {
            match transport.try_accept_uni_stream() {
                Ok(stream) => return Ok(stream),
                Err(StreamAcceptError::Empty) => tokio::task::yield_now().await,
                Err(StreamAcceptError::Closed) => return Err(EvalError::Bulk),
            }
        }
    })
    .await
    .map_err(|_| EvalError::Timeout)?
}

async fn receive_chunk(stream: &mut dyn TransportReceiveStream) -> Result<Vec<u8>, EvalError> {
    timeout(READY_TIMEOUT, async {
        loop {
            match stream.try_receive_chunk() {
                Ok(chunk) => return Ok(chunk),
                Err(StreamReceiveError::Empty) => tokio::task::yield_now().await,
                Err(StreamReceiveError::Finished | StreamReceiveError::Cancelled) => {
                    return Err(EvalError::Bulk);
                }
            }
        }
    })
    .await
    .map_err(|_| EvalError::Timeout)?
}

async fn expect_finished(stream: &mut dyn TransportReceiveStream) -> Result<(), EvalError> {
    timeout(READY_TIMEOUT, async {
        loop {
            match stream.try_receive_chunk() {
                Err(StreamReceiveError::Empty) => tokio::task::yield_now().await,
                Err(StreamReceiveError::Finished) => return Ok(()),
                Ok(_) | Err(StreamReceiveError::Cancelled) => return Err(EvalError::Bulk),
            }
        }
    })
    .await
    .map_err(|_| EvalError::Timeout)?
}

async fn wait_for_direct_path(connection: &iroh::endpoint::Connection) -> Result<(), EvalError> {
    let mut paths = connection.paths_stream();
    timeout(READY_TIMEOUT, async {
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

fn parse_required<'a>(line: Option<&'a str>, key: &str) -> Result<&'a str, EvalError> {
    let line = line.ok_or(EvalError::InvalidValue)?;
    let (actual, value) = line.split_once('=').ok_or(EvalError::InvalidValue)?;
    if actual != key || value.is_empty() {
        return Err(EvalError::InvalidValue);
    }
    Ok(value)
}

fn parse_optional<'a>(line: Option<&'a str>, key: &str) -> Result<Option<&'a str>, EvalError> {
    let value = parse_required(line, key)?;
    Ok((value != "-").then_some(value))
}

#[cfg(test)]
mod tests {
    use std::{fs, process, time::Duration};

    use tokio::time::timeout;

    use super::{NetprobePeerArgs, run_client, run_server};
    use crate::relay::OwnerRelay;

    #[tokio::test]
    async fn netprobe_runtime_authenticates_control_data_and_observes_direct_path() {
        let relay = OwnerRelay::start().await.expect("owner relay");
        let rendezvous = std::env::temp_dir().join(format!(
            "crosslab-m9-netprobe-{}-{}.addr",
            process::id(),
            std::thread::current().name().unwrap_or("test")
        ));
        let _ = fs::remove_file(&rendezvous);
        let args = NetprobePeerArgs::new(rendezvous.clone(), relay.url().clone());

        let (server, client) = timeout(Duration::from_secs(25), async {
            tokio::join!(run_server(args.clone()), run_client(args))
        })
        .await
        .expect("bounded netprobe runtime");
        server.expect("netprobe server");
        let output = client.expect("netprobe client");

        for expected in [
            "phase\trelay_verified",
            "network_class\tremote",
            "control_verified\ttrue",
            "data_verified\ttrue",
            "phase\tdirect_verified",
            "binding_unchanged\ttrue",
            "session_unchanged\ttrue",
            "phase\treconnect_verified",
            "binding_refreshed\ttrue",
            "session_refreshed\ttrue",
            "control_after_reconnect\ttrue",
        ] {
            assert!(output.lines().any(|line| line == expected), "missing {expected}");
        }

        let _ = fs::remove_file(rendezvous);
        relay.shutdown().await.expect("owner relay shutdown");
    }
}
