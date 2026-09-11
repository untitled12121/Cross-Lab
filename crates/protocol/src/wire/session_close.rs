use crate::{ProtocolDiagnostic, SessionClose, SessionCloseReason};

use super::{
    codec::ProtocolWireError,
    v1::{SessionCloseReasonV1, SessionCloseV1},
};

impl From<&SessionClose> for SessionCloseV1 {
    fn from(close: &SessionClose) -> Self {
        let reason = match close.reason() {
            SessionCloseReason::Normal => SessionCloseReasonV1::Normal,
            SessionCloseReason::LocalRequest => SessionCloseReasonV1::LocalRequest,
            SessionCloseReason::ProtocolError => SessionCloseReasonV1::ProtocolError,
            SessionCloseReason::AuthenticationLost => SessionCloseReasonV1::AuthenticationLost,
            SessionCloseReason::TrustRevoked => SessionCloseReasonV1::TrustRevoked,
            SessionCloseReason::Shutdown => SessionCloseReasonV1::Shutdown,
        } as i32;

        Self {
            reason,
            diagnostic: close
                .diagnostic()
                .map(|diagnostic| diagnostic.as_str().to_owned()),
        }
    }
}

impl TryFrom<SessionCloseV1> for SessionClose {
    type Error = ProtocolWireError;

    fn try_from(wire: SessionCloseV1) -> Result<Self, Self::Error> {
        let reason = match wire.reason {
            value if value == SessionCloseReasonV1::Normal as i32 => SessionCloseReason::Normal,
            value if value == SessionCloseReasonV1::LocalRequest as i32 => {
                SessionCloseReason::LocalRequest
            }
            value if value == SessionCloseReasonV1::ProtocolError as i32 => {
                SessionCloseReason::ProtocolError
            }
            value if value == SessionCloseReasonV1::AuthenticationLost as i32 => {
                SessionCloseReason::AuthenticationLost
            }
            value if value == SessionCloseReasonV1::TrustRevoked as i32 => {
                SessionCloseReason::TrustRevoked
            }
            value if value == SessionCloseReasonV1::Shutdown as i32 => SessionCloseReason::Shutdown,
            value => return Err(ProtocolWireError::InvalidSessionCloseReason(value)),
        };
        let diagnostic = wire
            .diagnostic
            .map(|value| {
                ProtocolDiagnostic::new(&value).map_err(|_| ProtocolWireError::InvalidDiagnostic)
            })
            .transpose()?;

        Ok(SessionClose::new(reason, diagnostic))
    }
}
