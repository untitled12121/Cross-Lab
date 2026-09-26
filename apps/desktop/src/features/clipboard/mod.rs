use crosslab_agent::ClipboardOperationError;
use crosslab_protocol::ProtocolErrorCode;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ClipboardAction {
    Send,
    Fetch,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ClipboardResult {
    Success,
    Unavailable,
    NotConnected,
    NotNegotiated,
    Oversized,
    ResourceLimit,
    TimedOut,
    Cancelled,
    Denied,
    Failed,
}

impl From<ClipboardOperationError> for ClipboardResult {
    fn from(error: ClipboardOperationError) -> Self {
        match error {
            ClipboardOperationError::NotConnected => Self::NotConnected,
            ClipboardOperationError::NotNegotiated => Self::NotNegotiated,
            ClipboardOperationError::Oversized => Self::Oversized,
            ClipboardOperationError::ResourceLimit => Self::ResourceLimit,
            ClipboardOperationError::TimedOut => Self::TimedOut,
            ClipboardOperationError::Cancelled => Self::Cancelled,
            ClipboardOperationError::Remote(code) => match code {
                ProtocolErrorCode::AuthorizationDenied
                | ProtocolErrorCode::TrustDenied
                | ProtocolErrorCode::OperationRevoked => Self::Denied,
                ProtocolErrorCode::CapabilityUnsupported
                | ProtocolErrorCode::CapabilityVersionIncompatible => Self::Unavailable,
                ProtocolErrorCode::ResourceLimit => Self::ResourceLimit,
                ProtocolErrorCode::Cancelled => Self::Cancelled,
                _ => Self::Failed,
            },
            ClipboardOperationError::InvalidResponse
            | ClipboardOperationError::Random
            | ClipboardOperationError::Transport
            | ClipboardOperationError::Closed => Self::Failed,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ClipboardFeatureState {
    available: bool,
    busy: bool,
    action: Option<ClipboardAction>,
    result: Option<ClipboardResult>,
}

impl ClipboardFeatureState {
    pub const fn new() -> Self {
        Self {
            available: false,
            busy: false,
            action: None,
            result: None,
        }
    }

    pub const fn available(&self) -> bool {
        self.available
    }

    pub const fn busy(&self) -> bool {
        self.busy
    }

    pub const fn action(&self) -> Option<ClipboardAction> {
        self.action
    }

    pub const fn result(&self) -> Option<ClipboardResult> {
        self.result
    }

    pub fn set_available(&mut self, available: bool) {
        self.available = available;
        if !available {
            self.busy = false;
            self.action = None;
        }
    }

    pub fn begin(&mut self, action: ClipboardAction) -> bool {
        if !self.available || self.busy {
            return false;
        }
        self.busy = true;
        self.action = Some(action);
        self.result = None;
        true
    }

    pub fn finish(&mut self, action: ClipboardAction, result: ClipboardResult) {
        if self.busy && self.action == Some(action) {
            self.busy = false;
            self.result = Some(result);
        }
    }
}

impl Default for ClipboardFeatureState {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn clipboard_state_allows_only_one_in_flight_action() {
        let mut state = ClipboardFeatureState::new();
        state.set_available(true);

        assert!(state.begin(ClipboardAction::Send));
        assert!(!state.begin(ClipboardAction::Fetch));

        state.finish(ClipboardAction::Send, ClipboardResult::Success);
        assert!(!state.busy());
        assert_eq!(state.result(), Some(ClipboardResult::Success));

        assert!(state.begin(ClipboardAction::Fetch));
    }

    #[test]
    fn remote_authorization_maps_to_denied() {
        assert_eq!(
            ClipboardResult::from(ClipboardOperationError::Remote(
                ProtocolErrorCode::AuthorizationDenied,
            )),
            ClipboardResult::Denied
        );
    }
}
