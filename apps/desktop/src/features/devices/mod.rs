use core::fmt::Write as _;

use crosslab_agent::{PresencePhase, PresenceSnapshot};
use crosslab_core::{SessionState, TransportSecurityClass};
use crosslab_identity::{DeviceId, OwnerId};
use crosslab_policy::{NetworkClass, TrustState};
use crosslab_protocol::ProtocolVersion;
use crosslab_runtime::{ConnectivityState, RuntimeStatus};

#[cfg(target_os = "linux")]
mod linux_discovery;
#[cfg(target_os = "linux")]
mod product_presence;
mod runtime;

#[cfg(target_os = "linux")]
pub use linux_discovery::{
    LinuxTrustedSessionDiscovery, LinuxTrustedSessionDiscoveryError,
    LinuxTrustedSessionDiscoveryEvent, LinuxTrustedSessionRoute,
};
#[cfg(target_os = "linux")]
pub use product_presence::{DesktopPresenceError, DesktopProductPresenceController};
pub use runtime::{DesktopRuntimeControlError, DesktopRuntimeController};

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum PresenceDisplay {
    #[default]
    Unavailable,
    Discovering,
    Connecting,
    Online,
    Reconnecting,
    Paused,
    Failed,
}

impl PresenceDisplay {
    pub const fn label(self) -> &'static str {
        match self {
            Self::Unavailable => "Unavailable",
            Self::Discovering => "Discovering",
            Self::Connecting => "Connecting",
            Self::Online => "Online",
            Self::Reconnecting => "Reconnecting",
            Self::Paused => "Paused",
            Self::Failed => "Discovery failed",
        }
    }
}

impl From<PresencePhase> for PresenceDisplay {
    fn from(phase: PresencePhase) -> Self {
        match phase {
            PresencePhase::Discovering => Self::Discovering,
            PresencePhase::Connecting => Self::Connecting,
            PresencePhase::Online => Self::Online,
            PresencePhase::Reconnecting => Self::Reconnecting,
            PresencePhase::Paused => Self::Paused,
            PresencePhase::Failed => Self::Failed,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TrustDisplay {
    Pending,
    Trusted,
    Revoked,
}

impl TrustDisplay {
    pub const fn label(self) -> &'static str {
        match self {
            Self::Pending => "Pending",
            Self::Trusted => "Trusted",
            Self::Revoked => "Revoked",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConnectivityDisplay {
    Connected,
    Disconnected,
}

impl ConnectivityDisplay {
    pub const fn label(self) -> &'static str {
        match self {
            Self::Connected => "Connected",
            Self::Disconnected => "Disconnected",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SessionDisplay {
    Created,
    Authenticating,
    Active,
    Closing,
    Closed,
    Revoked,
}

impl SessionDisplay {
    pub const fn label(self) -> &'static str {
        match self {
            Self::Created => "Created",
            Self::Authenticating => "Authenticating",
            Self::Active => "Active",
            Self::Closing => "Closing",
            Self::Closed => "Closed",
            Self::Revoked => "Revoked",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NetworkDisplay {
    Local,
    Trusted,
    Remote,
}

impl NetworkDisplay {
    pub const fn label(self) -> &'static str {
        match self {
            Self::Local => "Local",
            Self::Trusted => "Trusted",
            Self::Remote => "Remote",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SecurityDisplay {
    TestOnly,
    Authenticated,
}

impl SecurityDisplay {
    pub const fn label(self) -> &'static str {
        match self {
            Self::TestOnly => "Test only",
            Self::Authenticated => "Authenticated",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DevicePresentation {
    owner_id: Option<String>,
    local_device_id: Option<String>,
    peer_id: Option<String>,
    trust: TrustDisplay,
    connectivity: ConnectivityDisplay,
    session: SessionDisplay,
    protocol: Option<String>,
    network: NetworkDisplay,
    security: SecurityDisplay,
    metered: Option<bool>,
    capability_count: usize,
}

impl DevicePresentation {
    pub fn from_runtime(status: &RuntimeStatus) -> Self {
        Self::from_fields(StatusFields {
            owner_id: status.owner_id(),
            local_device_id: status.local_device_id(),
            peer_device_id: status.peer_device_id(),
            trust_state: status.trust_state(),
            connectivity: status.connectivity(),
            session_state: status.session_state(),
            protocol_version: status.protocol_version(),
            network_class: status.transport().network_class(),
            security_class: status.transport().security_class(),
            metered: status.transport().metered(),
            capability_count: status.negotiated_capability_ids().len(),
        })
    }

    pub fn owner_id(&self) -> Option<&str> {
        self.owner_id.as_deref()
    }

    pub fn local_device_id(&self) -> Option<&str> {
        self.local_device_id.as_deref()
    }

    pub fn peer_id(&self) -> Option<&str> {
        self.peer_id.as_deref()
    }

    pub const fn trust(&self) -> TrustDisplay {
        self.trust
    }

    pub const fn connectivity(&self) -> ConnectivityDisplay {
        self.connectivity
    }

    pub const fn session(&self) -> SessionDisplay {
        self.session
    }

    pub fn protocol(&self) -> Option<&str> {
        self.protocol.as_deref()
    }

    pub const fn network(&self) -> NetworkDisplay {
        self.network
    }

    pub const fn security(&self) -> SecurityDisplay {
        self.security
    }

    pub const fn metered(&self) -> Option<bool> {
        self.metered
    }

    pub const fn capability_count(&self) -> usize {
        self.capability_count
    }

    fn from_fields(fields: StatusFields) -> Self {
        Self {
            owner_id: fields.owner_id.map(short_owner_id),
            local_device_id: fields.local_device_id.map(short_device_id),
            peer_id: fields.peer_device_id.map(short_device_id),
            trust: match fields.trust_state {
                TrustState::Pending => TrustDisplay::Pending,
                TrustState::Trusted => TrustDisplay::Trusted,
                TrustState::Revoked => TrustDisplay::Revoked,
            },
            connectivity: match fields.connectivity {
                ConnectivityState::Connected => ConnectivityDisplay::Connected,
                ConnectivityState::Disconnected => ConnectivityDisplay::Disconnected,
            },
            session: match fields.session_state {
                SessionState::Created => SessionDisplay::Created,
                SessionState::Authenticating => SessionDisplay::Authenticating,
                SessionState::Active => SessionDisplay::Active,
                SessionState::Closing => SessionDisplay::Closing,
                SessionState::Closed => SessionDisplay::Closed,
                SessionState::Revoked => SessionDisplay::Revoked,
            },
            protocol: fields
                .protocol_version
                .map(|version| format!("{}.{}", version.major(), version.minor())),
            network: match fields.network_class {
                NetworkClass::Local => NetworkDisplay::Local,
                NetworkClass::Trusted => NetworkDisplay::Trusted,
                NetworkClass::Remote => NetworkDisplay::Remote,
            },
            security: match fields.security_class {
                TransportSecurityClass::InProcessTest => SecurityDisplay::TestOnly,
                TransportSecurityClass::AuthenticatedConfidentialChannel => {
                    SecurityDisplay::Authenticated
                }
            },
            metered: fields.metered,
            capability_count: fields.capability_count,
        }
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct DevicesFeatureState {
    current: Option<DevicePresentation>,
    presence: PresenceDisplay,
}

impl DevicesFeatureState {
    pub const fn empty() -> Self {
        Self {
            current: None,
            presence: PresenceDisplay::Unavailable,
        }
    }

    pub fn from_runtime(status: &RuntimeStatus) -> Self {
        Self {
            current: Some(DevicePresentation::from_runtime(status)),
            presence: PresenceDisplay::Online,
        }
    }

    pub fn update_runtime(&mut self, status: &RuntimeStatus) {
        self.current = Some(DevicePresentation::from_runtime(status));
        self.presence = PresenceDisplay::Online;
    }

    pub fn update_presence(&mut self, snapshot: &PresenceSnapshot) {
        self.presence = snapshot.phase().into();
        self.current = snapshot.runtime().map(DevicePresentation::from_runtime);
    }

    pub fn clear(&mut self) {
        self.current = None;
        self.presence = PresenceDisplay::Unavailable;
    }

    pub const fn current(&self) -> Option<&DevicePresentation> {
        self.current.as_ref()
    }

    pub const fn presence(&self) -> PresenceDisplay {
        self.presence
    }
}

#[derive(Debug, Clone, Copy)]
struct StatusFields {
    owner_id: Option<OwnerId>,
    local_device_id: Option<DeviceId>,
    peer_device_id: Option<DeviceId>,
    trust_state: TrustState,
    connectivity: ConnectivityState,
    session_state: SessionState,
    protocol_version: Option<ProtocolVersion>,
    network_class: NetworkClass,
    security_class: TransportSecurityClass,
    metered: Option<bool>,
    capability_count: usize,
}

fn short_device_id(device_id: DeviceId) -> String {
    short_id(device_id.as_bytes())
}

fn short_owner_id(owner_id: OwnerId) -> String {
    short_id(owner_id.as_bytes())
}

fn short_id(bytes: &[u8; 32]) -> String {
    let mut output = String::with_capacity(16);
    for byte in &bytes[..8] {
        write!(&mut output, "{byte:02x}").expect("writing to String cannot fail");
    }
    output
}

#[cfg(test)]
mod tests;
