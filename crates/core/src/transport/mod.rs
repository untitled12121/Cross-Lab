use core::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TransportSecurityClass {
    InProcessTest,
    AuthenticatedConfidentialChannel,
}

#[derive(Clone, PartialEq, Eq)]
pub struct ChannelBinding {
    profile_id: &'static str,
    bytes: Vec<u8>,
}

impl ChannelBinding {
    pub fn new(profile_id: &'static str, bytes: Vec<u8>) -> Self {
        Self { profile_id, bytes }
    }

    pub const fn profile_id(&self) -> &'static str {
        self.profile_id
    }

    pub fn bytes(&self) -> &[u8] {
        &self.bytes
    }
}

impl fmt::Debug for ChannelBinding {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("ChannelBinding")
            .field("profile_id", &self.profile_id)
            .field(
                "bytes",
                &format_args!("[REDACTED; {} bytes]", self.bytes.len()),
            )
            .finish()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConnectionMetadata {
    local_endpoint: Option<String>,
    remote_endpoint: Option<String>,
    metered: Option<bool>,
}

impl ConnectionMetadata {
    pub fn new(
        local_endpoint: Option<String>,
        remote_endpoint: Option<String>,
        metered: Option<bool>,
    ) -> Self {
        Self {
            local_endpoint,
            remote_endpoint,
            metered,
        }
    }

    pub fn local_endpoint(&self) -> Option<&str> {
        self.local_endpoint.as_deref()
    }

    pub fn remote_endpoint(&self) -> Option<&str> {
        self.remote_endpoint.as_deref()
    }

    pub const fn metered(&self) -> Option<bool> {
        self.metered
    }
}

#[derive(PartialEq, Eq)]
pub enum ControlSendError {
    Full(Vec<u8>),
    Closed(Vec<u8>),
}

impl fmt::Debug for ControlSendError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Full(frame) => formatter
                .debug_tuple("Full")
                .field(&format_args!("[REDACTED; {} bytes]", frame.len()))
                .finish(),
            Self::Closed(frame) => formatter
                .debug_tuple("Closed")
                .field(&format_args!("[REDACTED; {} bytes]", frame.len()))
                .finish(),
        }
    }
}

impl fmt::Display for ControlSendError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::Full(_) => "transport control queue is full",
            Self::Closed(_) => "transport control channel is closed",
        })
    }
}

impl std::error::Error for ControlSendError {}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ControlReceiveError {
    Empty,
    Closed,
}

impl fmt::Display for ControlReceiveError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::Empty => "transport control queue is empty",
            Self::Closed => "transport control channel is closed",
        })
    }
}

impl std::error::Error for ControlReceiveError {}

pub trait TransportConnection {
    fn security_class(&self) -> TransportSecurityClass;

    fn channel_binding(&self) -> &ChannelBinding;

    fn connection_metadata(&self) -> &ConnectionMetadata;

    fn try_send_control(&self, frame: Vec<u8>) -> Result<(), ControlSendError>;

    fn try_receive_control(&self) -> Result<Vec<u8>, ControlReceiveError>;

    fn close(&self);

    fn is_closed(&self) -> bool;
}
