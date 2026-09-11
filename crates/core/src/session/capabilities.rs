use std::collections::BTreeMap;

use crosslab_policy::{CapabilityId, CapabilityVersion, LocalCapability};
use crosslab_protocol::CapabilityAdvertisement;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NegotiatedCapability {
    capability_id: CapabilityId,
    version: CapabilityVersion,
}

impl NegotiatedCapability {
    fn new(capability_id: CapabilityId, version: CapabilityVersion) -> Self {
        Self {
            capability_id,
            version,
        }
    }

    pub fn capability_id(&self) -> &CapabilityId {
        &self.capability_id
    }

    pub const fn version(&self) -> CapabilityVersion {
        self.version
    }
}

pub(crate) fn negotiate_session_capabilities(
    local: &[LocalCapability],
    peer: &CapabilityAdvertisement,
) -> Vec<NegotiatedCapability> {
    let mut negotiated = BTreeMap::<CapabilityId, CapabilityVersion>::new();

    for local_capability in local {
        if !local_capability.runtime_available() {
            continue;
        }
        let local_range = local_capability.supported_versions();

        for peer_capability in peer.entries() {
            if !peer_capability.runtime_available()
                || peer_capability.capability_id() != local_capability.capability_id()
                || peer_capability.min_version().major() != local_range.major()
            {
                continue;
            }

            let min_minor = local_range.min_minor().max(peer_capability.min_version().minor());
            let max_minor = local_range.max_minor().min(peer_capability.max_version().minor());
            if min_minor > max_minor {
                continue;
            }

            let candidate = CapabilityVersion::new(local_range.major(), max_minor);
            negotiated
                .entry(local_capability.capability_id().clone())
                .and_modify(|current| *current = (*current).max(candidate))
                .or_insert(candidate);
        }
    }

    negotiated
        .into_iter()
        .map(|(capability_id, version)| NegotiatedCapability::new(capability_id, version))
        .collect()
}
