use std::collections::BTreeMap;

use crosslab_identity::DeviceId;

use crate::{CapabilityId, CapabilityVersion, LocalCapability, OperationName, TrustState};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct SessionId([u8; 32]);

impl SessionId {
    pub const fn from_bytes(bytes: [u8; 32]) -> Self {
        Self(bytes)
    }

    pub const fn to_bytes(self) -> [u8; 32] {
        self.0
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct RuleId([u8; 32]);

impl RuleId {
    pub const fn from_bytes(bytes: [u8; 32]) -> Self {
        Self(bytes)
    }

    pub const fn to_bytes(self) -> [u8; 32] {
        self.0
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RuleEffect {
    Allow,
    Deny,
    Ask,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NetworkClass {
    Local,
    Trusted,
    Remote,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Constraint {
    LocalOnly,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Obligation {
    OwnerConfirmation,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ApprovalScope {
    source_device_id: DeviceId,
    destination_device_id: DeviceId,
    session_id: SessionId,
    capability_id: CapabilityId,
    operation: OperationName,
}

impl ApprovalScope {
    pub fn new(
        source_device_id: DeviceId,
        destination_device_id: DeviceId,
        session_id: SessionId,
        capability_id: CapabilityId,
        operation: OperationName,
    ) -> Self {
        Self {
            source_device_id,
            destination_device_id,
            session_id,
            capability_id,
            operation,
        }
    }

    pub fn from_context(context: &AuthorizationContext) -> Self {
        Self::new(
            context.source_device_id,
            context.destination_device_id,
            context.session_id,
            context.capability_id.clone(),
            context.operation.clone(),
        )
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VerifiedApproval {
    obligation: Obligation,
    scope: ApprovalScope,
}

impl VerifiedApproval {
    pub fn owner_confirmation(scope: ApprovalScope) -> Self {
        Self {
            obligation: Obligation::OwnerConfirmation,
            scope,
        }
    }

    fn satisfies(&self, obligation: Obligation, scope: &ApprovalScope) -> bool {
        self.obligation == obligation && self.scope == *scope
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AuthorizationContext {
    source_device_id: DeviceId,
    destination_device_id: DeviceId,
    session_id: SessionId,
    capability_id: CapabilityId,
    negotiated_version: CapabilityVersion,
    operation: OperationName,
    trust_state: TrustState,
    trust_revision: u64,
    local_capability: LocalCapability,
    network_class: NetworkClass,
    verified_approvals: Vec<VerifiedApproval>,
}

impl AuthorizationContext {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        source_device_id: DeviceId,
        destination_device_id: DeviceId,
        session_id: SessionId,
        capability_id: CapabilityId,
        negotiated_version: CapabilityVersion,
        operation: OperationName,
        trust_state: TrustState,
        trust_revision: u64,
        local_capability: LocalCapability,
        network_class: NetworkClass,
    ) -> Self {
        Self {
            source_device_id,
            destination_device_id,
            session_id,
            capability_id,
            negotiated_version,
            operation,
            trust_state,
            trust_revision,
            local_capability,
            network_class,
            verified_approvals: Vec::new(),
        }
    }

    pub fn with_verified_approval(mut self, approval: VerifiedApproval) -> Self {
        self.verified_approvals.push(approval);
        self
    }

    pub fn set_trust_state(&mut self, state: TrustState) {
        self.trust_state = state;
    }

    pub fn set_local_capability(&mut self, capability: LocalCapability) {
        self.local_capability = capability;
    }

    pub fn set_negotiated_version(&mut self, version: CapabilityVersion) {
        self.negotiated_version = version;
    }

    pub fn set_network_class(&mut self, network_class: NetworkClass) {
        self.network_class = network_class;
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PolicyRule {
    rule_id: RuleId,
    source_device_id: DeviceId,
    capability_id: CapabilityId,
    operation: OperationName,
    effect: RuleEffect,
    constraints: Vec<Constraint>,
    obligations: Vec<Obligation>,
}

impl PolicyRule {
    pub fn new(
        rule_id: RuleId,
        source_device_id: DeviceId,
        capability_id: CapabilityId,
        operation: OperationName,
        effect: RuleEffect,
    ) -> Self {
        Self {
            rule_id,
            source_device_id,
            capability_id,
            operation,
            effect,
            constraints: Vec::new(),
            obligations: Vec::new(),
        }
    }

    pub fn with_constraint(mut self, constraint: Constraint) -> Self {
        if !self.constraints.contains(&constraint) {
            self.constraints.push(constraint);
        }
        self
    }

    pub fn with_obligation(mut self, obligation: Obligation) -> Self {
        if !self.obligations.contains(&obligation) {
            self.obligations.push(obligation);
        }
        self
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PolicyError {
    DuplicateRule,
    RevisionOverflow,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
struct PolicyKey {
    source_device_id: DeviceId,
    capability_id: CapabilityId,
    operation: OperationName,
}

impl PolicyKey {
    fn new(
        source_device_id: DeviceId,
        capability_id: CapabilityId,
        operation: OperationName,
    ) -> Self {
        Self {
            source_device_id,
            capability_id,
            operation,
        }
    }
}

#[derive(Debug, Default)]
pub struct PolicyState {
    revision: u64,
    rules: BTreeMap<PolicyKey, PolicyRule>,
}

impl PolicyState {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn insert(&mut self, rule: PolicyRule) -> Result<(), PolicyError> {
        let key = PolicyKey::new(
            rule.source_device_id,
            rule.capability_id.clone(),
            rule.operation.clone(),
        );
        if self.rules.contains_key(&key) {
            return Err(PolicyError::DuplicateRule);
        }
        let next_revision = self
            .revision
            .checked_add(1)
            .ok_or(PolicyError::RevisionOverflow)?;
        self.rules.insert(key, rule);
        self.revision = next_revision;
        Ok(())
    }

    pub const fn revision(&self) -> u64 {
        self.revision
    }

    pub fn evaluate(&self, context: &AuthorizationContext) -> PolicyDecision {
        match context.trust_state {
            TrustState::Pending => return PolicyDecision::deny(DecisionReason::UntrustedPeer),
            TrustState::Revoked => return PolicyDecision::deny(DecisionReason::RevokedPeer),
            TrustState::Trusted => {}
        }

        if context.local_capability.capability_id() != &context.capability_id {
            return PolicyDecision::deny(DecisionReason::UnsupportedCapability);
        }
        if !context.local_capability.runtime_available() {
            return PolicyDecision::deny(DecisionReason::RuntimeUnavailable);
        }
        if !context
            .local_capability
            .supports(context.negotiated_version)
        {
            return PolicyDecision::deny(DecisionReason::IncompatibleCapabilityVersion);
        }

        let key = PolicyKey::new(
            context.source_device_id,
            context.capability_id.clone(),
            context.operation.clone(),
        );
        let Some(rule) = self.rules.get(&key) else {
            return PolicyDecision::deny(DecisionReason::NoMatchingRule);
        };

        if rule.effect == RuleEffect::Deny {
            return PolicyDecision::deny(DecisionReason::ExplicitDeny);
        }

        if rule.constraints.iter().any(|constraint| match constraint {
            Constraint::LocalOnly => context.network_class != NetworkClass::Local,
        }) {
            return PolicyDecision::deny(DecisionReason::ConstraintFailed);
        }

        let scope = ApprovalScope::from_context(context);
        let required = if rule.effect == RuleEffect::Ask && rule.obligations.is_empty() {
            vec![Obligation::OwnerConfirmation]
        } else {
            rule.obligations.clone()
        };
        let missing = required
            .into_iter()
            .filter(|obligation| {
                !context
                    .verified_approvals
                    .iter()
                    .any(|approval| approval.satisfies(*obligation, &scope))
            })
            .collect::<Vec<_>>();

        if !missing.is_empty() {
            return PolicyDecision::ask(missing);
        }

        PolicyDecision::allow(AuthorizationGrant {
            rule_id: rule.rule_id,
            source_device_id: context.source_device_id,
            destination_device_id: context.destination_device_id,
            session_id: context.session_id,
            capability_id: context.capability_id.clone(),
            capability_version: context.negotiated_version,
            operation: context.operation.clone(),
            trust_revision: context.trust_revision,
            policy_revision: self.revision,
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DecisionEffect {
    Allow,
    Deny,
    Ask,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DecisionReason {
    NoMatchingRule,
    ExplicitDeny,
    UntrustedPeer,
    RevokedPeer,
    UnsupportedCapability,
    IncompatibleCapabilityVersion,
    RuntimeUnavailable,
    ConstraintFailed,
    ApprovalRequired,
    Allowed,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PolicyDecision {
    effect: DecisionEffect,
    reason: DecisionReason,
    required_obligations: Vec<Obligation>,
    grant: Option<AuthorizationGrant>,
}

impl PolicyDecision {
    fn deny(reason: DecisionReason) -> Self {
        Self {
            effect: DecisionEffect::Deny,
            reason,
            required_obligations: Vec::new(),
            grant: None,
        }
    }

    fn ask(required_obligations: Vec<Obligation>) -> Self {
        Self {
            effect: DecisionEffect::Ask,
            reason: DecisionReason::ApprovalRequired,
            required_obligations,
            grant: None,
        }
    }

    fn allow(grant: AuthorizationGrant) -> Self {
        Self {
            effect: DecisionEffect::Allow,
            reason: DecisionReason::Allowed,
            required_obligations: Vec::new(),
            grant: Some(grant),
        }
    }

    pub const fn effect(&self) -> DecisionEffect {
        self.effect
    }

    pub const fn reason(&self) -> DecisionReason {
        self.reason
    }

    pub fn required_obligations(&self) -> &[Obligation] {
        &self.required_obligations
    }

    pub fn into_grant(self) -> Option<AuthorizationGrant> {
        self.grant
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AuthorizationGrant {
    pub(crate) rule_id: RuleId,
    pub(crate) source_device_id: DeviceId,
    pub(crate) destination_device_id: DeviceId,
    pub(crate) session_id: SessionId,
    pub(crate) capability_id: CapabilityId,
    pub(crate) capability_version: CapabilityVersion,
    pub(crate) operation: OperationName,
    pub(crate) trust_revision: u64,
    pub(crate) policy_revision: u64,
}
