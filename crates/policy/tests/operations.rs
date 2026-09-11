use crosslab_identity::DeviceId;
use crosslab_policy::{
    AuthorizationGrant, AuthorizedOperation, CapabilityId, CapabilityVersion, CapabilityVersionRange,
    LocalCapability, NetworkClass, OperationError, OperationName, OperationState,
    OperationUseContext, PolicyRule, PolicyState, RuleEffect, RuleId, SessionId, TrustState,
    UsePolicy,
};

struct Fixture {
    source: DeviceId,
    destination: DeviceId,
    session: SessionId,
    capability: CapabilityId,
    operation: OperationName,
    version: CapabilityVersion,
    trust_revision: u64,
    policy_revision: u64,
    grant: AuthorizationGrant,
}

impl Fixture {
    fn new() -> Self {
        let source = DeviceId::from_bytes([1; 32]);
        let destination = DeviceId::from_bytes([2; 32]);
        let session = SessionId::from_bytes([3; 32]);
        let capability = CapabilityId::parse("files.transfer").unwrap();
        let operation = OperationName::parse("receive").unwrap();
        let version = CapabilityVersion::new(1, 0);
        let local = LocalCapability::new(
            capability.clone(),
            CapabilityVersionRange::new(1, 0, 0).unwrap(),
            true,
        );
        let mut policy = PolicyState::new();
        policy
            .insert(PolicyRule::new(
                RuleId::from_bytes([4; 32]),
                source,
                capability.clone(),
                operation.clone(),
                RuleEffect::Allow,
            ))
            .unwrap();
        let context = crosslab_policy::AuthorizationContext::new(
            source,
            destination,
            session,
            capability.clone(),
            version,
            operation.clone(),
            TrustState::Trusted,
            5,
            local,
            NetworkClass::Local,
        );
        let grant = policy.evaluate(&context).into_grant().unwrap();

        Self {
            source,
            destination,
            session,
            capability,
            operation,
            version,
            trust_revision: 5,
            policy_revision: policy.revision(),
            grant,
        }
    }

    fn use_context(&self) -> OperationUseContext {
        OperationUseContext::new(
            self.source,
            self.destination,
            self.session,
            self.capability.clone(),
            self.version,
            self.operation.clone(),
        )
    }
}

#[test]
fn allow_grant_creates_active_operation_with_nonreused_random_id() {
    let fixture = Fixture::new();
    let first = AuthorizedOperation::issue(fixture.grant.clone(), 10, 20, UsePolicy::SingleStream)
        .unwrap();
    let second = AuthorizedOperation::issue(fixture.grant, 10, 20, UsePolicy::SingleStream).unwrap();

    assert_eq!(first.state(), OperationState::Active);
    assert_eq!(first.id().as_bytes().len(), 32);
    assert_ne!(first.id(), second.id());
}

#[test]
fn matching_binding_and_revisions_validate() {
    let fixture = Fixture::new();
    let mut operation =
        AuthorizedOperation::issue(fixture.grant, 10, 20, UsePolicy::SingleStream).unwrap();

    operation
        .validate(
            &fixture.use_context(),
            15,
            fixture.trust_revision,
            fixture.policy_revision,
        )
        .unwrap();
}

#[test]
fn operation_id_does_not_authorize_wrong_source_or_session() {
    let fixture = Fixture::new();
    let mut operation =
        AuthorizedOperation::issue(fixture.grant, 10, 20, UsePolicy::SingleStream).unwrap();

    let wrong_source = OperationUseContext::new(
        DeviceId::from_bytes([40; 32]),
        fixture.destination,
        fixture.session,
        fixture.capability.clone(),
        fixture.version,
        fixture.operation.clone(),
    );
    assert_eq!(
        operation.validate(
            &wrong_source,
            15,
            fixture.trust_revision,
            fixture.policy_revision,
        ),
        Err(OperationError::BindingMismatch)
    );

    let wrong_session = OperationUseContext::new(
        fixture.source,
        fixture.destination,
        SessionId::from_bytes([41; 32]),
        fixture.capability.clone(),
        fixture.version,
        fixture.operation.clone(),
    );
    assert_eq!(
        operation.validate(
            &wrong_session,
            15,
            fixture.trust_revision,
            fixture.policy_revision,
        ),
        Err(OperationError::BindingMismatch)
    );
}

#[test]
fn expiry_transitions_operation_to_terminal_expired_state() {
    let fixture = Fixture::new();
    let mut operation =
        AuthorizedOperation::issue(fixture.grant, 10, 20, UsePolicy::SingleStream).unwrap();

    assert_eq!(
        operation.validate(
            &fixture.use_context(),
            20,
            fixture.trust_revision,
            fixture.policy_revision,
        ),
        Err(OperationError::Inactive(OperationState::Expired))
    );
    assert_eq!(operation.state(), OperationState::Expired);
}

#[test]
fn trust_or_policy_revision_change_revokes_operation() {
    let fixture = Fixture::new();
    let mut trust_changed =
        AuthorizedOperation::issue(fixture.grant.clone(), 10, 20, UsePolicy::SingleStream).unwrap();
    assert_eq!(
        trust_changed.validate(
            &fixture.use_context(),
            15,
            fixture.trust_revision + 1,
            fixture.policy_revision,
        ),
        Err(OperationError::TrustRevisionChanged)
    );
    assert_eq!(trust_changed.state(), OperationState::Revoked);

    let mut policy_changed =
        AuthorizedOperation::issue(fixture.grant, 10, 20, UsePolicy::SingleStream).unwrap();
    assert_eq!(
        policy_changed.validate(
            &fixture.use_context(),
            15,
            fixture.trust_revision,
            fixture.policy_revision + 1,
        ),
        Err(OperationError::PolicyRevisionChanged)
    );
    assert_eq!(policy_changed.state(), OperationState::Revoked);
}

#[test]
fn explicit_terminal_transitions_prevent_further_use() {
    let fixture = Fixture::new();
    let mut cancelled =
        AuthorizedOperation::issue(fixture.grant.clone(), 10, 20, UsePolicy::SingleAction).unwrap();
    cancelled.cancel();
    assert_eq!(cancelled.state(), OperationState::Cancelled);
    assert_eq!(
        cancelled.validate(
            &fixture.use_context(),
            15,
            fixture.trust_revision,
            fixture.policy_revision,
        ),
        Err(OperationError::Inactive(OperationState::Cancelled))
    );

    let mut consumed =
        AuthorizedOperation::issue(fixture.grant, 10, 20, UsePolicy::SingleAction).unwrap();
    consumed.consume();
    assert_eq!(consumed.state(), OperationState::Consumed);
}
