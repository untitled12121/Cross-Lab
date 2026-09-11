use crosslab_policy::CapabilityId;

use crate::{Event, EventId, EventScope, EventType};

use super::{
    codec::{ProtocolWireError, copy_16},
    v1::EventV1,
};

impl From<&Event> for EventV1 {
    fn from(event: &Event) -> Self {
        let capability_id = match event.scope() {
            EventScope::Capability(capability_id) => Some(capability_id.as_str().to_owned()),
            EventScope::System => None,
        };

        Self {
            event_id: event.event_id().to_bytes().to_vec(),
            capability_id,
            event_type: event.event_type().as_str().to_owned(),
            body: event.body().to_vec(),
        }
    }
}

impl TryFrom<EventV1> for Event {
    type Error = ProtocolWireError;

    fn try_from(wire: EventV1) -> Result<Self, Self::Error> {
        let event_id = EventId::from_bytes(copy_16(
            wire.event_id,
            ProtocolWireError::InvalidEventIdLength,
        )?);
        let event_type = EventType::parse(&wire.event_type)
            .map_err(|_| ProtocolWireError::InvalidEventType)?;

        match wire.capability_id {
            Some(value) => {
                let capability_id = CapabilityId::parse(&value)
                    .map_err(|_| ProtocolWireError::InvalidCapabilityId)?;
                Event::capability(event_id, capability_id, event_type, wire.body)
                    .map_err(|_| ProtocolWireError::InvalidEventScope)
            }
            None => Event::system(event_id, event_type, wire.body)
                .map_err(|_| ProtocolWireError::InvalidEventScope),
        }
    }
}
