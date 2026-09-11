use crosslab_identity::DeviceId;
use crosslab_policy::{
    ApprovalScope, AuthorizationContext, CapabilityId, CapabilityVersion, CapabilityVersionRange,
    Constraint, DecisionEffect, DecisionReason, LocalCapability, NetworkClass, Obligation,
    OperationName, PolicyRule, PolicyState, RuleEffect, RuleId, SessionId, TrustState,
    VerifiedApproval,
};

struct Fixture {
    source: DeviceId,
    destination: DeviceId,
    session: SessionId,
    capability: CapabilityId,
    operation: OperationName,
    local_capability: LocalCapability,
}

impl Fixture {
    fn new() -> Self {
        let capability = CapabilityId::parse("files.transfer").unwrap();
        Self {
            source: DeviceId::from_bytes([1; 32]),
            destination: DeviceId::from_bytes([2; 32]),
            session: SessionId::from_bytes([3; 32]),
            local_capability: LocalCapability::new(
                capability.clone(),
                CapabilityVersionRange::new(1, 0, 2).unwrap(),
                true,
            ),
            capability,
            operation: OperationName::parse("receive").unwrap(),
        }
    }

    fn context(&self) -> AuthorizationContext {
        AuthorizationContext::new(
            self.source,
            self.destination,
            self.session,
            self.capability.clone(),
            CapabilityVersion::new(1, 1),
            self.operation.clone(),
            TrustState::Trusted,
            7,
            self.local_capability.clone(),
            NetworkClass::Local,
        )
    }

    fn rule(&self, effect: RuleEffect) -> PolicyRule {
        PolicyRule::new(
            RuleId::from_bytes([9; 32]),
            self.source,
            self.capability.clone(),
            self.operation.clone(),
            effect,
        )
    }
}

#[test]
fn no_matching_rule_denies() {
    let fixture = Fixture::new();
    let policy = PolicyState::new();

    let decision = policy.evaluate(&fixture.context());

    assert_eq!(decision.effect(), DecisionEffect::Deny);
    assert_eq!(decision.reason(), DecisionReason::NoMatchingRule);
}

#[test]
fn exact_allow_rule_allows_when_context_is_eligible() {
    let fixture = Fixture::new();
    let mut policy = PolicyState::new();
    policy.insert(fixture.rule(RuleEffect::Allow)).unwrap();

    let decision = policy.evaluate(&fixture.context());

    assert_eq!(decision.effect(), DecisionEffect::Allow);
    assert_eq!(decision.reason(), DecisionReason::Allowed);
    assert!(decision.into_grant().is_some());
}

#[test]
fn explicit_deny_rule_denies() {
    let fixture = Fixture::new();
    let mut policy = PolicyState::new();
    policy.insert(fixture.rule(RuleEffect::Deny)).unwrap();

    let decision = policy.evaluate(&fixture.context());

    assert_eq!(decision.effect(), DecisionEffect::Deny);
    assert_eq!(decision.reason(), DecisionReason::ExplicitDeny);
}

#[test]
fn pending_and_revoked_sources_fail_closed() {
    let fixture = Fixture::new();
    let mut policy = PolicyState::new();
    policy.insert(fixture.rule(RuleEffect::Allow)).unwrap();

    let mut pending = fixture.context();
    pending.set_trust_state(TrustState::Pending);
    let pending_decision = policy.evaluate(&pending);
    assert_eq!(pending_decision.effect(), DecisionEffect::Deny);
    assert_eq!(pending_decision.reason(), DecisionReason::UntrustedPeer);

    let mut revoked = fixture.context();
    revoked.set_trust_state(TrustState::Revoked);
    let revoked_decision = policy.evaluate(&revoked);
    assert_eq!(revoked_decision.effect(), DecisionEffect::Deny);
    assert_eq!(revoked_decision.reason(), DecisionReason::RevokedPeer);
}

#[test]
fn runtime_unavailable_and_incompatible_versions_deny() {
    let fixture = Fixture::new();
    let mut policy = PolicyState::new();
    policy.insert(fixture.rule(RuleEffect::Allow)).unwrap();

    let mut unavailable = fixture.context();
    unavailable.set_local_capability(LocalCapability::new(
        fixture.capability.clone(),
        CapabilityVersionRange::new(1, 0, 2).unwrap(),
        false,
    ));
    assert_eq!(
        policy.evaluate(&unavailable).reason(),
        DecisionReason::RuntimeUnavailable
    );

    let mut incompatible = fixture.context();
    incompatible.set_negotiated_version(CapabilityVersion::new(2, 0));
    assert_eq!(
        policy.evaluate(&incompatible).reason(),
        DecisionReason::IncompatibleCapabilityVersion
    );
}

#[test]
fn false_hard_constraint_denies() {
    let fixture = Fixture::new();
    let mut policy = PolicyState::new();
    policy
        .insert(fixture.rule(RuleEffect::Allow).with_constraint(Constraint::LocalOnly))
        .unwrap();

    let mut context = fixture.context();
    context.set_network_class(NetworkClass::Remote);
    let decision = policy.evaluate(&context);

    assert_eq!(decision.effect(), DecisionEffect::Deny);
    assert_eq!(decision.reason(), DecisionReason::ConstraintFailed);
}

#[test]
fn ask_requires_scoped_locally_verified_approval() {
    let fixture = Fixture::new();
    let mut policy = PolicyState::new();
    policy
        .insert(
            fixture
                .rule(RuleEffect::Ask)
                .with_obligation(Obligation::OwnerConfirmation),
        )
        .unwrap();

    let context = fixture.context();
    let decision = policy.evaluate(&context);
    assert_eq!(decision.effect(), DecisionEffect::Ask);
    assert_eq!(decision.reason(), DecisionReason::ApprovalRequired);
    assert_eq!(
        decision.required_obligations(),
        &[Obligation::OwnerConfirmation]
    );

    let scope = ApprovalScope::from_context(&context);
    let approved = context.with_verified_approval(VerifiedApproval::owner_confirmation(scope));
    let decision = policy.evaluate(&approved);
    assert_eq!(decision.effect(), DecisionEffect::Allow);
    assert!(decision.into_grant().is_some());
}

#[test]
fn wrong_scope_approval_does_not_satisfy_obligation() {
    let fixture = Fixture::new();
    let mut policy = PolicyState::new();
    policy
        .insert(
            fixture
                .rule(RuleEffect::Ask)
                .with_obligation(Obligation::OwnerConfirmation),
        )
        .unwrap();

    let context = fixture.context();
    let wrong_scope = ApprovalScope::new(
        DeviceId::from_bytes([40; 32]),
        fixture.destination,
        fixture.session,
        fixture.capability.clone(),
        fixture.operation.clone(),
    );
    let context =
        context.with_verified_approval(VerifiedApproval::owner_confirmation(wrong_scope));
    let decision = policy.evaluate(&context);

    assert_eq!(decision.effect(), DecisionEffect::Ask);
    assert_eq!(decision.reason(), DecisionReason::ApprovalRequired);
}
