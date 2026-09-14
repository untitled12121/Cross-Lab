use crosslab_policy::{OperationId, RuleId, TransitionId};

#[test]
fn policy_ids_use_secure_random_generation_api() {
    let rule = RuleId::generate().expect("rule id generation should succeed");
    let transition = TransitionId::generate().expect("transition id generation should succeed");
    let operation = OperationId::generate().expect("operation id generation should succeed");

    assert_eq!(RuleId::from_bytes(rule.to_bytes()), rule);
    assert_eq!(TransitionId::from_bytes(transition.to_bytes()), transition);
    assert_eq!(OperationId::from_bytes(operation.to_bytes()), operation);
}
