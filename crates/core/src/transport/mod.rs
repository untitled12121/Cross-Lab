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

#[derive(Clone, PartialEq, Eq)]
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

impl fmt::Debug for ConnectionMetadata {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("ConnectionMetadata")
            .field(
                "local_endpoint",
                &self.local_endpoint.as_ref().map(|_| "[REDACTED]"),
            )
            .field(
                "remote_endpoint",
                &self.remote_endpoint.as_ref().map(|_| "[REDACTED]"),
            )
            .field("metered", &self.metered)
            .finish()
    }
}

#[derive(PartialEq, Eq)]
pub enum ControlSendError {
    Full(Vec<u8>),
    TooLarge(Vec<u8>),
    Closed(Vec<u8>),
}

impl fmt::Debug for ControlSendError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Full(frame) => redact_bytes(formatter, "Full", frame),
            Self::TooLarge(frame) => redact_bytes(formatter, "TooLarge", frame),
            Self::Closed(frame) => redact_bytes(formatter, "Closed", frame),
        }
    }
}

impl fmt::Display for ControlSendError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::Full(_) => "transport control queue is full",
            Self::TooLarge(_) => "transport control frame exceeds its size limit",
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

#[derive(PartialEq, Eq)]
pub enum StreamOpenError {
    Full(Vec<u8>),
    TooLarge(Vec<u8>),
    Closed(Vec<u8>),
}

impl StreamOpenError {
    pub fn into_opening_frame(self) -> Vec<u8> {
        match self {
            Self::Full(frame) | Self::TooLarge(frame) | Self::Closed(frame) => frame,
        }
    }
}

impl fmt::Debug for StreamOpenError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Full(frame) => redact_bytes(formatter, "Full", frame),
            Self::TooLarge(frame) => redact_bytes(formatter, "TooLarge", frame),
            Self::Closed(frame) => redact_bytes(formatter, "Closed", frame),
        }
    }
}

impl fmt::Display for StreamOpenError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::Full(_) => "transport data stream capacity is full",
            Self::TooLarge(_) => "transport data stream opening frame exceeds its size limit",
            Self::Closed(_) => "transport data stream direction is closed",
        })
    }
}

impl std::error::Error for StreamOpenError {}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StreamAcceptError {
    Empty,
    Closed,
}

impl fmt::Display for StreamAcceptError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::Empty => "transport has no pending data stream",
            Self::Closed => "transport data stream direction is closed",
        })
    }
}

impl std::error::Error for StreamAcceptError {}

#[derive(PartialEq, Eq)]
pub enum StreamSendError {
    Full(Vec<u8>),
    TooLarge(Vec<u8>),
    Closed(Vec<u8>),
}

impl StreamSendError {
    pub fn into_chunk(self) -> Vec<u8> {
        match self {
            Self::Full(chunk) | Self::TooLarge(chunk) | Self::Closed(chunk) => chunk,
        }
    }
}

impl fmt::Debug for StreamSendError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Full(chunk) => redact_bytes(formatter, "Full", chunk),
            Self::TooLarge(chunk) => redact_bytes(formatter, "TooLarge", chunk),
            Self::Closed(chunk) => redact_bytes(formatter, "Closed", chunk),
        }
    }
}

impl fmt::Display for StreamSendError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::Full(_) => "transport data stream queue is full",
            Self::TooLarge(_) => "transport data stream chunk exceeds its size limit",
            Self::Closed(_) => "transport data stream is closed",
        })
    }
}

impl std::error::Error for StreamSendError {}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StreamReceiveError {
    Empty,
    Finished,
    Cancelled,
}

impl fmt::Display for StreamReceiveError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::Empty => "transport data stream queue is empty",
            Self::Finished => "transport data stream finished",
            Self::Cancelled => "transport data stream was cancelled",
        })
    }
}

impl std::error::Error for StreamReceiveError {}

pub trait TransportSendStream: Send {
    fn try_send_chunk(&mut self, chunk: Vec<u8>) -> Result<(), StreamSendError>;
    fn finish(&mut self);
    fn cancel(&mut self);
}

pub trait TransportReceiveStream: Send {
    fn try_receive_chunk(&mut self) -> Result<Vec<u8>, StreamReceiveError>;
    fn cancel(&mut self);
}

pub struct IncomingUniStream {
    opening_frame: Vec<u8>,
    stream: Box<dyn TransportReceiveStream>,
}

impl IncomingUniStream {
    pub fn new(opening_frame: Vec<u8>, stream: Box<dyn TransportReceiveStream>) -> Self {
        Self {
            opening_frame,
            stream,
        }
    }

    pub fn opening_frame(&self) -> &[u8] {
        &self.opening_frame
    }

    pub fn into_parts(self) -> (Vec<u8>, Box<dyn TransportReceiveStream>) {
        (self.opening_frame, self.stream)
    }
}

impl fmt::Debug for IncomingUniStream {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("IncomingUniStream")
            .field(
                "opening_frame",
                &format_args!("[REDACTED; {} bytes]", self.opening_frame.len()),
            )
            .finish_non_exhaustive()
    }
}

pub trait TransportConnection {
    fn security_class(&self) -> TransportSecurityClass;

    fn channel_binding(&self) -> &ChannelBinding;

    fn connection_metadata(&self) -> &ConnectionMetadata;

    fn try_send_control(&self, frame: Vec<u8>) -> Result<(), ControlSendError>;

    fn try_receive_control(&self) -> Result<Vec<u8>, ControlReceiveError>;

    fn try_open_uni_stream(
        &self,
        opening_frame: Vec<u8>,
    ) -> Result<Box<dyn TransportSendStream>, StreamOpenError>;

    fn try_accept_uni_stream(&self) -> Result<IncomingUniStream, StreamAcceptError>;

    fn close(&self);

    fn is_closed(&self) -> bool;
}

fn redact_bytes(formatter: &mut fmt::Formatter<'_>, name: &str, bytes: &[u8]) -> fmt::Result {
    formatter
        .debug_tuple(name)
        .field(&format_args!("[REDACTED; {} bytes]", bytes.len()))
        .finish()
}
