//! Trust and authorization policy domain for Cross-Lab.

mod capability;
mod trust;

pub use capability::{
    CapabilityId, CapabilityVersion, CapabilityVersionRange, IdentifierError, OperationName,
    VersionRangeError,
};
pub use trust::{TransitionId, TrustError, TrustRecord, TrustState};
