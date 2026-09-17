use crosslab_core::{LogicalSession, SessionState, TransportConnection, TransportSecurityClass};
use crosslab_identity::DeviceId;
use crosslab_policy::{CapabilityId, NetworkClass, SessionId, TrustRecord, TrustState};
use crosslab_protocol::ProtocolVersion;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConnectivityState {
    Connected,
    Disconnected,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TransportStatus {
    network_class: NetworkClass,
    security_class: TransportSecurityClass,
    metered: Option<bool>,
}

impl TransportStatus {
    pub const fn network_class(&self) -> NetworkClass {
        self.network_class
    }

    pub const fn security_class(&self) -> TransportSecurityClass {
        self.security_class
    }

    pub const fn metered(&self) -> Option<bool> {
        self.metered
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimeStatus {
    local_device_id: Option<DeviceId>,
    peer_device_id: Option<DeviceId>,
    session_id: Option<SessionId>,
    trust_state: TrustState,
    connectivity: ConnectivityState,
    session_state: SessionState,
    protocol_version: Option<ProtocolVersion>,
    negotiated_features: Vec<u16>,
    negotiated_capability_ids: Vec<CapabilityId>,
    transport: TransportStatus,
}

impl RuntimeStatus {
    pub(crate) fn from_runtime(
        session: &LogicalSession,
        transport: &dyn TransportConnection,
        peer_trust: &TrustRecord,
        network_class: NetworkClass,
    ) -> Self {
        let context = session.context();
        let trust_state = context
            .filter(|context| {
                peer_trust.owner_id() == context.owner_id()
                    && peer_trust.device_id() == context.peer_device_id()
            })
            .map_or(TrustState::Pending, |_| peer_trust.state());
        let session_id = (session.state() == SessionState::Active)
            .then(|| context.map(|context| context.session_id()))
            .flatten();
        let negotiated_capability_ids = context
            .map(|context| {
                context
                    .negotiated_capabilities()
                    .iter()
                    .map(|capability| capability.capability_id().clone())
                    .collect()
            })
            .unwrap_or_default();

        Self {
            local_device_id: context.map(|context| context.local_device_id()),
            peer_device_id: context.map(|context| context.peer_device_id()),
            session_id,
            trust_state,
            connectivity: if transport.is_closed() {
                ConnectivityState::Disconnected
            } else {
                ConnectivityState::Connected
            },
            session_state: session.state(),
            protocol_version: context.map(|context| context.protocol_version()),
            negotiated_features: context
                .map(|context| context.negotiated_features().to_vec())
                .unwrap_or_default(),
            negotiated_capability_ids,
            transport: TransportStatus {
                network_class,
                security_class: transport.security_class(),
                metered: transport.connection_metadata().metered(),
            },
        }
    }

    pub const fn local_device_id(&self) -> Option<DeviceId> {
        self.local_device_id
    }

    pub const fn peer_device_id(&self) -> Option<DeviceId> {
        self.peer_device_id
    }

    pub const fn session_id(&self) -> Option<SessionId> {
        self.session_id
    }

    pub const fn trust_state(&self) -> TrustState {
        self.trust_state
    }

    pub const fn connectivity(&self) -> ConnectivityState {
        self.connectivity
    }

    pub const fn session_state(&self) -> SessionState {
        self.session_state
    }

    pub const fn protocol_version(&self) -> Option<ProtocolVersion> {
        self.protocol_version
    }

    pub fn negotiated_features(&self) -> &[u16] {
        &self.negotiated_features
    }

    pub fn negotiated_capability_ids(&self) -> &[CapabilityId] {
        &self.negotiated_capability_ids
    }

    pub const fn transport(&self) -> &TransportStatus {
        &self.transport
    }
}
