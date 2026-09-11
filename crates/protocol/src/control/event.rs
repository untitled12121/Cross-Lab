use core::fmt;

use crosslab_policy::CapabilityId;

use super::EventId;

pub const MAX_EVENT_TYPE_BYTES: usize = 128;
pub const SYSTEM_EVENT_TYPE_PREFIX: &str = "crosslab.system.";

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct EventType(String);

impl EventType {
    pub fn parse(value: &str) -> Result<Self, EventTypeError> {
        if value.len() > MAX_EVENT_TYPE_BYTES || !value.is_ascii() {
            return Err(EventTypeError);
        }

        let mut segments = value.split('.');
        let first = segments.next().ok_or(EventTypeError)?;
        let second = segments.next().ok_or(EventTypeError)?;
        if !valid_segment(first) || !valid_segment(second) || !segments.all(valid_segment) {
            return Err(EventTypeError);
        }

        Ok(Self(value.to_owned()))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }

    pub fn is_system(&self) -> bool {
        self.0.starts_with(SYSTEM_EVENT_TYPE_PREFIX)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EventTypeError;

impl fmt::Display for EventTypeError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("invalid canonical event type")
    }
}

impl std::error::Error for EventTypeError {}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EventScope {
    Capability(CapabilityId),
    System,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EventError {
    ReservedSystemType,
    SystemTypeRequired,
}

impl fmt::Display for EventError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::ReservedSystemType => {
                "reserved system event type cannot be used for a capability event"
            }
            Self::SystemTypeRequired => "system event requires the reserved system event namespace",
        })
    }
}

impl std::error::Error for EventError {}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Event {
    event_id: EventId,
    scope: EventScope,
    event_type: EventType,
    body: Vec<u8>,
}

impl Event {
    pub fn capability(
        event_id: EventId,
        capability_id: CapabilityId,
        event_type: EventType,
        body: Vec<u8>,
    ) -> Result<Self, EventError> {
        if event_type.is_system() {
            return Err(EventError::ReservedSystemType);
        }
        Ok(Self {
            event_id,
            scope: EventScope::Capability(capability_id),
            event_type,
            body,
        })
    }

    pub fn system(
        event_id: EventId,
        event_type: EventType,
        body: Vec<u8>,
    ) -> Result<Self, EventError> {
        if !event_type.is_system() {
            return Err(EventError::SystemTypeRequired);
        }
        Ok(Self {
            event_id,
            scope: EventScope::System,
            event_type,
            body,
        })
    }

    pub const fn event_id(&self) -> EventId {
        self.event_id
    }

    pub const fn scope(&self) -> &EventScope {
        &self.scope
    }

    pub const fn event_type(&self) -> &EventType {
        &self.event_type
    }

    pub fn body(&self) -> &[u8] {
        &self.body
    }
}

fn valid_segment(value: &str) -> bool {
    let bytes = value.as_bytes();
    if bytes.is_empty() || !bytes[0].is_ascii_lowercase() || bytes.last() == Some(&b'-') {
        return false;
    }

    bytes
        .iter()
        .skip(1)
        .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || *byte == b'-')
}
