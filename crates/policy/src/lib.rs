//! Trust and authorization policy domain for Cross-Lab.

mod authorization;
mod capability;
mod id_generation;
mod operation;
mod trust;

pub use authorization::{
    ApprovalError, ApprovalInstant, ApprovalScope, AuthorizationContext, AuthorizationGrant,
    Constraint, DecisionEffect, DecisionReason, NetworkClass, Obligation, OwnerApprovalEvidence,
    PolicyDecision, PolicyError, PolicyRule, PolicyState, RuleEffect, RuleId, SessionId,
    VerifiedApproval,
};
pub use capability::{
    CapabilityId, CapabilityVersion, CapabilityVersionRange, IdentifierError, LocalCapability,
    OperationName, VersionRangeError,
};
pub use id_generation::PolicyIdGenerationError;
pub use operation::{
    AuthorizedOperation, OperationError, OperationId, OperationState, OperationUseContext,
    UsePolicy,
};
pub use trust::{
    CredentialRotationError, PairingTrustTransition, PairingTrustTransitionError, TransitionId,
    TrustError, TrustRecord, TrustState, TrustTransition, TrustTransitionError,
};
