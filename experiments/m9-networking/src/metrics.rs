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

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Report {
    measurements: Vec<Measurement>,
}

impl Report {
    pub fn new(measurements: Vec<Measurement>) -> Self {
        Self { measurements }
    }

    pub fn contains(&self, metric: MetricKind) -> bool {
        self.measurements
            .iter()
            .any(|measurement| measurement.metric() == metric)
    }

    pub fn to_tsv(&self) -> String {
        let mut output = String::from("transport\tmetric\tsample\tvalue\n");
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
}
