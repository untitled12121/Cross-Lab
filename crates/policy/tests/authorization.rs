use crosslab_crypto::SigningKey;
use crosslab_identity::{
    AuthorityDelegation, AuthorityRole, DeviceId, OwnerId, OwnerRootRecord,
};
use crosslab_policy::{
    ApprovalError, ApprovalInstant, ApprovalScope, AuthorizationContext, CapabilityId,
    CapabilityVersion, CapabilityVersionRange, Constraint, DecisionEffect, DecisionReason,
    LocalCapability, NetworkClass, Obligation, OperationName, OwnerApprovalEvidence, PolicyRule,
    PolicyState, RuleEffect, RuleId, SessionId, TrustState, VerifiedApproval,
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
        self.context_at(15)
    }

    fn context_at(&self, now: u64) -> AuthorizationContext {
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
            ApprovalInstant::from_ticks(now),
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

    fn administrative_authority(
        &self,
    ) -> (
        OwnerRootRecord,
        SigningKey,
        AuthorityDelegation,
    ) {
        let owner_id = OwnerId::from_bytes([0x40; 32]);
        let root_key = SigningKey::from_secret_bytes([0x41; 32]);
        let root = OwnerRootRecord::new(owner_id, &root_key, 0);
        let administrative_key = SigningKey::from_secret_bytes([0x42; 32]);
        let delegation = AuthorityDelegation::issue(
            owner_id,
            AuthorityRole::Administrative,
            &administrative_key,
            3,
            &root_key,
        );
        (root, administrative_key, delegation)
    }

    fn verified_owner_approval(
        &self,
        scope: ApprovalScope,
        issued_at: u64,
        expires_at: u64,
    ) -> VerifiedApproval {
        let (root, administrative_key, delegation) = self.administrative_authority();
        OwnerApprovalEvidence::issue(
            scope,
            ApprovalInstant::from_ticks(issued_at),
            ApprovalInstant::from_ticks(expires_at),
            &root,
            &delegation,
            &administrative_key,
            delegation.delegation_epoch(),
        )
        .unwrap()
        .verify(&root, &delegation, delegation.delegation_epoch())
        .unwrap()
    }
}

#[test]
fn no_matching_rule_denies() {
    let fixture = Fixture::new();
    let policy = PolicyState::new();

    let decision = policy.evaluate(&fixture.context());

    assert_eq!(decision.effect(), DecisionEffect::Deny);
    assert_eq!(decision.reason(), DecisionReason::NoMatchingRule);
    assert_eq!(decision.matched_rule_id(), None);
    assert_eq!(decision.policy_revision(), 0);
}

#[test]
fn exact_allow_rule_allows_when_context_is_eligible() {
    let fixture = Fixture::new();
    let mut policy = PolicyState::new();
    policy.insert(fixture.rule(RuleEffect::Allow)).unwrap();

    let decision = policy.evaluate(&fixture.context());

    assert_eq!(decision.effect(), DecisionEffect::Allow);
    assert_eq!(decision.reason(), DecisionReason::Allowed);
    assert_eq!(
        decision.matched_rule_id(),
        Some(RuleId::from_bytes([9; 32]))
    );
    assert_eq!(decision.policy_revision(), 1);
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
        .insert(
            fixture
                .rule(RuleEffect::Allow)
                .with_constraint(Constraint::LocalOnly),
        )
        .unwrap();

    let mut context = fixture.context();
    context.set_network_class(NetworkClass::Remote);
    let decision = policy.evaluate(&context);

    assert_eq!(decision.effect(), DecisionEffect::Deny);
    assert_eq!(decision.reason(), DecisionReason::ConstraintFailed);
    assert_eq!(decision.constraints(), &[Constraint::LocalOnly]);
}

#[test]
fn ask_requires_scoped_signed_owner_approval() {
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

    let approval = fixture.verified_owner_approval(ApprovalScope::from_context(&context), 10, 20);
    let approved = context.with_verified_approval(approval);
    let decision = policy.evaluate(&approved);
    assert_eq!(decision.effect(), DecisionEffect::Allow);
    assert!(decision.into_grant().is_some());
}

#[test]
fn approval_expiry_is_reevaluated_from_local_time() {
    let fixture = Fixture::new();
    let mut policy = PolicyState::new();
    policy.insert(fixture.rule(RuleEffect::Ask)).unwrap();

    let active_context = fixture.context_at(19);
    let approval = fixture.verified_owner_approval(
        ApprovalScope::from_context(&active_context),
        10,
        20,
    );
    assert_eq!(
        policy
            .evaluate(&active_context.with_verified_approval(approval.clone()))
            .effect(),
        DecisionEffect::Allow
    );

    let expired_context = fixture.context_at(20).with_verified_approval(approval);
    assert_eq!(policy.evaluate(&expired_context).effect(), DecisionEffect::Ask);
}

#[test]
fn wrong_scope_approval_does_not_satisfy_obligation() {
    let fixture = Fixture::new();
    let mut policy = PolicyState::new();
    policy.insert(fixture.rule(RuleEffect::Ask)).unwrap();

    let context = fixture.context();
    let wrong_scope = ApprovalScope::new(
        DeviceId::from_bytes([40; 32]),
        fixture.destination,
        fixture.session,
        fixture.capability.clone(),
        fixture.operation.clone(),
    );
    let approval = fixture.verified_owner_approval(wrong_scope, 10, 20);
    let decision = policy.evaluate(&context.with_verified_approval(approval));

    assert_eq!(decision.effect(), DecisionEffect::Ask);
    assert_eq!(decision.reason(), DecisionReason::ApprovalRequired);
}

#[test]
fn non_administrative_delegation_cannot_authorize_owner_approval() {
    let fixture = Fixture::new();
    let context = fixture.context();
    let owner_id = OwnerId::from_bytes([0x50; 32]);
    let root_key = SigningKey::from_secret_bytes([0x51; 32]);
    let root = OwnerRootRecord::new(owner_id, &root_key, 0);
    let device_signing_key = SigningKey::from_secret_bytes([0x52; 32]);
    let delegation = AuthorityDelegation::issue(
        owner_id,
        AuthorityRole::DeviceSigning,
        &device_signing_key,
        0,
        &root_key,
    );

    assert_eq!(
        OwnerApprovalEvidence::issue(
            ApprovalScope::from_context(&context),
            ApprovalInstant::from_ticks(10),
            ApprovalInstant::from_ticks(20),
            &root,
            &delegation,
            &device_signing_key,
            delegation.delegation_epoch(),
        ),
        Err(ApprovalError::WrongIssuerRole)
    );
}

#[test]
fn invalid_approval_lifetime_is_rejected() {
    let fixture = Fixture::new();
    let context = fixture.context();
    let (root, administrative_key, delegation) = fixture.administrative_authority();

    assert_eq!(
        OwnerApprovalEvidence::issue(
            ApprovalScope::from_context(&context),
            ApprovalInstant::from_ticks(20),
            ApprovalInstant::from_ticks(20),
            &root,
            &delegation,
            &administrative_key,
            delegation.delegation_epoch(),
        ),
        Err(ApprovalError::InvalidLifetime)
    );
}
