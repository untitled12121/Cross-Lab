use crosslab_core::{SessionState, TransportSecurityClass};
use crosslab_identity::DeviceId;
use crosslab_policy::{NetworkClass, TrustState};
use crosslab_protocol::ProtocolVersion;
use crosslab_runtime::{ConnectivityState, RuntimeStatus};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TrustDisplay {
    Pending,
    Trusted,
    Revoked,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConnectivityDisplay {
    Connected,
    Disconnected,
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NetworkDisplay {
    Local,
    Trusted,
    Remote,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SecurityDisplay {
    TestOnly,
    Authenticated,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DevicePresentation {
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

    fn from_fields(_fields: StatusFields) -> Self {
        unimplemented!("RED: implement safe runtime status projection")
    }
}

#[derive(Debug, Clone, Copy)]
struct StatusFields {
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

#[cfg(test)]
mod tests;
