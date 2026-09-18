use core::fmt::Write as _;

use crosslab_core::{SessionState, TransportSecurityClass};
use crosslab_identity::DeviceId;
use crosslab_policy::{NetworkClass, SessionId, TrustState};
use crosslab_protocol::ProtocolVersion;
use crosslab_runtime::{ConnectivityState, RuntimeStatus};

#[derive(Debug, Clone, Copy, PartialEq, Eq, uniffi::Enum)]
pub enum MobileLifecycleState {
    Stopped,
    Running,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, uniffi::Enum)]
pub enum MobileTrustState {
    Unavailable,
    Pending,
    Trusted,
    Revoked,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, uniffi::Enum)]
pub enum MobileConnectivityState {
    Connected,
    Disconnected,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, uniffi::Enum)]
pub enum MobileSessionState {
    Unavailable,
    Created,
    Authenticating,
    Active,
    Closing,
    Closed,
    Revoked,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, uniffi::Enum)]
pub enum MobileNetworkClass {
    Unavailable,
    Local,
    Trusted,
    Remote,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, uniffi::Enum)]
pub enum MobileTransportSecurity {
    Unavailable,
    TestOnly,
    Authenticated,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, uniffi::Record)]
pub struct MobileProtocolVersion {
    pub major: u16,
    pub minor: u16,
}

#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct MobileRuntimeSnapshot {
    pub revision: u64,
    pub lifecycle: MobileLifecycleState,
    pub local_device_id: Option<String>,
    pub peer_device_id: Option<String>,
    pub session_id: Option<String>,
    pub trust: MobileTrustState,
    pub connectivity: MobileConnectivityState,
    pub session: MobileSessionState,
    pub protocol: Option<MobileProtocolVersion>,
    pub network: MobileNetworkClass,
    pub transport_security: MobileTransportSecurity,
    pub metered: Option<bool>,
    pub capability_count: u64,
}

impl MobileRuntimeSnapshot {
    pub(crate) fn from_runtime(
        status: &RuntimeStatus,
        lifecycle: MobileLifecycleState,
        revision: u64,
    ) -> Self {
        Self::from_fields(
            lifecycle,
            revision,
            RuntimeStatusFields {
                local_device_id: status.local_device_id(),
                peer_device_id: status.peer_device_id(),
                session_id: status.session_id(),
                trust_state: status.trust_state(),
                connectivity: status.connectivity(),
                session_state: status.session_state(),
                protocol_version: status.protocol_version(),
                network_class: status.transport().network_class(),
                security_class: status.transport().security_class(),
                metered: status.transport().metered(),
                capability_count: status.negotiated_capability_ids().len(),
            },
        )
    }

    pub(crate) fn disconnected(lifecycle: MobileLifecycleState, revision: u64) -> Self {
        Self {
            revision,
            lifecycle,
            local_device_id: None,
            peer_device_id: None,
            session_id: None,
            trust: MobileTrustState::Unavailable,
            connectivity: MobileConnectivityState::Disconnected,
            session: MobileSessionState::Unavailable,
            protocol: None,
            network: MobileNetworkClass::Unavailable,
            transport_security: MobileTransportSecurity::Unavailable,
            metered: None,
            capability_count: 0,
        }
    }

    fn from_fields(
        lifecycle: MobileLifecycleState,
        revision: u64,
        fields: RuntimeStatusFields,
    ) -> Self {
        let session_id = if fields.session_state == SessionState::Active {
            fields.session_id.map(|id| hex_id(&id.to_bytes()))
        } else {
            None
        };

        Self {
            revision,
            lifecycle,
            local_device_id: fields.local_device_id.map(|id| hex_id(id.as_bytes())),
            peer_device_id: fields.peer_device_id.map(|id| hex_id(id.as_bytes())),
            session_id,
            trust: if fields.peer_device_id.is_none() {
                MobileTrustState::Unavailable
            } else {
                match fields.trust_state {
                    TrustState::Pending => MobileTrustState::Pending,
                    TrustState::Trusted => MobileTrustState::Trusted,
                    TrustState::Revoked => MobileTrustState::Revoked,
                }
            },
            connectivity: match fields.connectivity {
                ConnectivityState::Connected => MobileConnectivityState::Connected,
                ConnectivityState::Disconnected => MobileConnectivityState::Disconnected,
            },
            session: match fields.session_state {
                SessionState::Created => MobileSessionState::Created,
                SessionState::Authenticating => MobileSessionState::Authenticating,
                SessionState::Active => MobileSessionState::Active,
                SessionState::Closing => MobileSessionState::Closing,
                SessionState::Closed => MobileSessionState::Closed,
                SessionState::Revoked => MobileSessionState::Revoked,
            },
            protocol: fields
                .protocol_version
                .map(|version| MobileProtocolVersion {
                    major: version.major(),
                    minor: version.minor(),
                }),
            network: match fields.network_class {
                NetworkClass::Local => MobileNetworkClass::Local,
                NetworkClass::Trusted => MobileNetworkClass::Trusted,
                NetworkClass::Remote => MobileNetworkClass::Remote,
            },
            transport_security: match fields.security_class {
                TransportSecurityClass::InProcessTest => MobileTransportSecurity::TestOnly,
                TransportSecurityClass::AuthenticatedConfidentialChannel => {
                    MobileTransportSecurity::Authenticated
                }
            },
            metered: fields.metered,
            capability_count: fields.capability_count as u64,
        }
    }

    #[cfg(test)]
    pub(crate) fn from_test_fields(
        lifecycle: MobileLifecycleState,
        revision: u64,
        fields: RuntimeStatusFields,
    ) -> Self {
        Self::from_fields(lifecycle, revision, fields)
    }
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct RuntimeStatusFields {
    pub local_device_id: Option<DeviceId>,
    pub peer_device_id: Option<DeviceId>,
    pub session_id: Option<SessionId>,
    pub trust_state: TrustState,
    pub connectivity: ConnectivityState,
    pub session_state: SessionState,
    pub protocol_version: Option<ProtocolVersion>,
    pub network_class: NetworkClass,
    pub security_class: TransportSecurityClass,
    pub metered: Option<bool>,
    pub capability_count: usize,
}

fn hex_id(bytes: &[u8; 32]) -> String {
    let mut output = String::with_capacity(64);
    for byte in bytes {
        write!(&mut output, "{byte:02x}").expect("writing to String cannot fail");
    }
    output
}
