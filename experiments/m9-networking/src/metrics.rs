use crate::config::EvalConfig;

const RUST_VERSION: &str = "1.98.1";
const QUINN_VERSION: &str = "0.11.11";
const IROH_VERSION: &str = "1.2.0";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TransportKind {
    Quinn,
    IrohDirect,
    IrohRelay,
    IrohRelayThenDirect,
}

impl TransportKind {
    const fn as_str(self) -> &'static str {
        match self {
            Self::Quinn => "quinn",
            Self::IrohDirect => "iroh_direct",
            Self::IrohRelay => "iroh_relay",
            Self::IrohRelayThenDirect => "iroh_relay_then_direct",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MetricKind {
    ProtectedConnectMicros,
    SessionAuthMicros,
    ControlRttMicros,
    BulkBytesPerSecond,
    RelayUpgradeMicros,
    ShutdownMicros,
}

impl MetricKind {
    const fn as_str(self) -> &'static str {
        match self {
            Self::ProtectedConnectMicros => "protected_connect_us",
            Self::SessionAuthMicros => "session_auth_us",
            Self::ControlRttMicros => "control_rtt_us",
            Self::BulkBytesPerSecond => "bulk_bytes_per_second",
            Self::RelayUpgradeMicros => "relay_upgrade_us",
            Self::ShutdownMicros => "shutdown_us",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Measurement {
    transport: TransportKind,
    metric: MetricKind,
    sample: usize,
    value: u128,
}

impl Measurement {
    pub const fn new(
        transport: TransportKind,
        metric: MetricKind,
        sample: usize,
        value: u128,
    ) -> Self {
        Self {
            transport,
            metric,
            sample,
            value,
        }
    }

    pub const fn metric(self) -> MetricKind {
        self.metric
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
struct ResourceObservations {
    rss_kib: Option<u64>,
    fd_count: Option<usize>,
}

impl ResourceObservations {
    #[cfg(target_os = "linux")]
    fn capture() -> Self {
        Self {
            rss_kib: linux_rss_kib(),
            fd_count: std::fs::read_dir("/proc/self/fd")
                .ok()
                .map(|entries| entries.filter_map(Result::ok).count()),
        }
    }

    #[cfg(not(target_os = "linux"))]
    const fn capture() -> Self {
        Self {
            rss_kib: None,
            fd_count: None,
        }
    }
}

#[cfg(target_os = "linux")]
fn linux_rss_kib() -> Option<u64> {
    let status = std::fs::read_to_string("/proc/self/status").ok()?;
    let value = status
        .lines()
        .find_map(|line| line.strip_prefix("VmRSS:"))?
        .split_whitespace()
        .next()?;
    value.parse().ok()
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct RunMetadata {
    samples: usize,
    payload_bytes: usize,
    resources: ResourceObservations,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Report {
    metadata: Option<RunMetadata>,
    measurements: Vec<Measurement>,
}

impl Report {
    pub fn new(measurements: Vec<Measurement>) -> Self {
        Self {
            metadata: None,
            measurements,
        }
    }

    pub fn for_run(config: EvalConfig, measurements: Vec<Measurement>) -> Self {
        Self {
            metadata: Some(RunMetadata {
                samples: config.samples(),
                payload_bytes: config.bulk_payload_bytes(),
                resources: ResourceObservations::default(),
            }),
            measurements,
        }
    }

    pub fn contains(&self, metric: MetricKind) -> bool {
        self.measurements
            .iter()
            .any(|measurement| measurement.metric() == metric)
    }

    pub fn contains_transport(&self, transport: TransportKind) -> bool {
        self.measurements
            .iter()
            .any(|measurement| measurement.transport == transport)
    }

    pub fn to_tsv(&self) -> String {
        let mut output = String::new();
        if let Some(metadata) = self.metadata {
            output.push_str("# os=");
            output.push_str(std::env::consts::OS);
            output.push('\n');
            output.push_str("# arch=");
            output.push_str(std::env::consts::ARCH);
            output.push('\n');
            output.push_str("# rust=");
            output.push_str(RUST_VERSION);
            output.push('\n');
            output.push_str("# samples=");
            output.push_str(&metadata.samples.to_string());
            output.push('\n');
            output.push_str("# payload_bytes=");
            output.push_str(&metadata.payload_bytes.to_string());
            output.push('\n');
            output.push_str("# quinn=");
            output.push_str(QUINN_VERSION);
            output.push('\n');
            output.push_str("# iroh=");
            output.push_str(IROH_VERSION);
            output.push('\n');
            push_optional_header(&mut output, "rss_kib", metadata.resources.rss_kib);
            push_optional_header(&mut output, "fd_count", metadata.resources.fd_count);
        }

        output.push_str("transport\tmetric\tsample\tvalue\n");
        for measurement in &self.measurements {
            output.push_str(measurement.transport.as_str());
            output.push('\t');
            output.push_str(measurement.metric.as_str());
            output.push('\t');
            output.push_str(&measurement.sample.to_string());
            output.push('\t');
            output.push_str(&measurement.value.to_string());
            output.push('\n');
        }
        output
    }

    pub fn extend(&mut self, measurements: impl IntoIterator<Item = Measurement>) {
        self.measurements.extend(measurements);
    }

    pub(crate) fn append(&mut self, mut other: Self) {
        self.measurements.append(&mut other.measurements);
    }

    pub(crate) fn capture_resources(&mut self) {
        if let Some(metadata) = &mut self.metadata {
            metadata.resources = ResourceObservations::capture();
        }
    }
}

fn push_optional_header<T>(output: &mut String, name: &str, value: Option<T>)
where
    T: std::fmt::Display,
{
    output.push_str("# ");
    output.push_str(name);
    output.push('=');
    match value {
        Some(value) => output.push_str(&value.to_string()),
        None => output.push('-'),
    }
    output.push('\n');
}
