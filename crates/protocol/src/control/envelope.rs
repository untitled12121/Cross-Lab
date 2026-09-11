use crosslab_policy::SessionId;

use crate::{CapabilityAdvertisement, ProtocolVersion};

use super::{CancelRequest, ControlRequest, ControlResponse, ProtocolFailure};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EnvelopeBody {
    CapabilityAdvertisement(CapabilityAdvertisement),
    ControlRequest(ControlRequest),
    ControlResponse(ControlResponse),
    CancelRequest(CancelRequest),
    ProtocolError(ProtocolFailure),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ControlEnvelope {
    protocol_version: ProtocolVersion,
    session_id: SessionId,
    message_seq: u64,
    body: EnvelopeBody,
}

impl ControlEnvelope {
    pub fn new(
        protocol_version: ProtocolVersion,
        session_id: SessionId,
        message_seq: u64,
        body: EnvelopeBody,
    ) -> Self {
        Self {
            protocol_version,
            session_id,
            message_seq,
            body,
        }
    }

    pub const fn protocol_version(&self) -> ProtocolVersion {
        self.protocol_version
    }

    pub const fn session_id(&self) -> SessionId {
        self.session_id
    }

    pub const fn message_seq(&self) -> u64 {
        self.message_seq
    }

    pub const fn body(&self) -> &EnvelopeBody {
        &self.body
    }
}
