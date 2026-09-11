use crosslab_policy::SessionId;
use prost::Message;

use crate::{
    ControlEnvelope, EnvelopeBody, FrameLimit, ProtocolVersion, decode_frame, encode_frame,
};

use super::{
    codec::{ProtocolWireError, copy_32},
    v1::{
        CancelRequestV1, CapabilityAdvertisementV1, ControlRequestV1, ControlResponseV1,
        EnvelopeV1, ProtocolErrorV1, envelope_v1,
    },
};

pub fn encode_control_envelope(envelope: &ControlEnvelope) -> Result<Vec<u8>, ProtocolWireError> {
    let wire = EnvelopeV1::from(envelope);
    encode_frame(&wire.encode_to_vec(), FrameLimit::NormalControl).map_err(Into::into)
}

pub fn decode_control_envelope(frame: &[u8]) -> Result<ControlEnvelope, ProtocolWireError> {
    let payload = decode_frame(frame, FrameLimit::NormalControl)?;
    let wire = EnvelopeV1::decode(payload).map_err(|_| ProtocolWireError::MalformedProtobuf)?;
    wire.try_into()
}

impl From<&ControlEnvelope> for EnvelopeV1 {
    fn from(envelope: &ControlEnvelope) -> Self {
        let body = match envelope.body() {
            EnvelopeBody::CapabilityAdvertisement(advertisement) => {
                envelope_v1::Body::CapabilityAdvertisement(CapabilityAdvertisementV1::from(
                    advertisement,
                ))
            }
            EnvelopeBody::ControlRequest(request) => {
                envelope_v1::Body::ControlRequest(ControlRequestV1::from(request))
            }
            EnvelopeBody::ControlResponse(response) => {
                envelope_v1::Body::ControlResponse(ControlResponseV1::from(response))
            }
            EnvelopeBody::CancelRequest(cancel) => {
                envelope_v1::Body::CancelRequest(CancelRequestV1::from(cancel))
            }
            EnvelopeBody::ProtocolError(error) => {
                envelope_v1::Body::ProtocolError(ProtocolErrorV1::from(error))
            }
        };
        let version = envelope.protocol_version();

        Self {
            protocol_major: version.major().into(),
            protocol_minor: version.minor().into(),
            session_id: envelope.session_id().to_bytes().to_vec(),
            message_seq: envelope.message_seq(),
            body: Some(body),
        }
    }
}

impl TryFrom<EnvelopeV1> for ControlEnvelope {
    type Error = ProtocolWireError;

    fn try_from(wire: EnvelopeV1) -> Result<Self, Self::Error> {
        let protocol_version = protocol_version_from_wire(wire.protocol_major, wire.protocol_minor)?;
        let session_id = SessionId::from_bytes(copy_32(
            wire.session_id,
            ProtocolWireError::InvalidSessionIdLength,
        )?);
        let body = match wire.body.ok_or(ProtocolWireError::MissingEnvelopeBody)? {
            envelope_v1::Body::CapabilityAdvertisement(advertisement) => {
                EnvelopeBody::CapabilityAdvertisement(advertisement.try_into()?)
            }
            envelope_v1::Body::ControlRequest(request) => {
                EnvelopeBody::ControlRequest(request.try_into()?)
            }
            envelope_v1::Body::ControlResponse(response) => {
                EnvelopeBody::ControlResponse(response.try_into()?)
            }
            envelope_v1::Body::CancelRequest(cancel) => {
                EnvelopeBody::CancelRequest(cancel.try_into()?)
            }
            envelope_v1::Body::ProtocolError(error) => {
                EnvelopeBody::ProtocolError(error.try_into()?)
            }
        };

        Ok(ControlEnvelope::new(
            protocol_version,
            session_id,
            wire.message_seq,
            body,
        ))
    }
}

fn protocol_version_from_wire(major: u32, minor: u32) -> Result<ProtocolVersion, ProtocolWireError> {
    let major = u16::try_from(major).map_err(|_| ProtocolWireError::InvalidProtocolVersion)?;
    let minor = u16::try_from(minor).map_err(|_| ProtocolWireError::InvalidProtocolVersion)?;
    Ok(ProtocolVersion::new(major, minor))
}
