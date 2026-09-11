//! Trust and authorization policy domain for Cross-Lab.

mod authorization;
mod capability;
mod trust;

pub use authorization::{
    ApprovalScope, AuthorizationContext, AuthorizationGrant, Constraint, DecisionEffect,
    DecisionReason, NetworkClass, Obligation, PolicyDecision, PolicyError, PolicyRule, PolicyState,
    RuleEffect, RuleId, SessionId, VerifiedApproval,
};
pub use capability::{
    CapabilityId, CapabilityVersion, CapabilityVersionRange, IdentifierError, LocalCapability,
    OperationName, VersionRangeError,
};
pub use trust::{TransitionId, TrustError, TrustRecord, TrustState};
