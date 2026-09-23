use std::{net::SocketAddr, sync::Arc, time::Duration};

use crosslab_crypto::SigningKey;
use crosslab_identity::{
    AuthorityDelegation, AuthorityRole, DeviceCredential, DeviceId, OwnerAuthorityState, OwnerId,
    OwnerRootRecord,
};
use crosslab_identity_store::ProductIdentityState;
use crosslab_policy::{
    CapabilityId, OperationName, PairingTrustTransition, PolicyState, RuleEffect, TransitionId,
};
use tokio::sync::watch;

use super::{PresencePhase, PresenceSnapshot, TrustedPresenceAgent, TrustedSessionRoute};

const WAIT: Duration = Duration::from_secs(8);

#[test]
fn discovery_rotation_changes_instance_without_changing_listener_port() {
    let (state, _) = reciprocal_identities();
    let signer = Arc::new(SigningKey::from_secret_bytes([0x74; 32]));
    let agent = TrustedPresenceAgent::spawn(state, signer).unwrap();

    let before = agent.discovery();
    let after = agent.rotate_discovery().unwrap();

    assert_ne!(before.instance(), after.instance());
    assert_eq!(before.listen_port(), after.listen_port());
    assert!(crosslab_core::is_session_dns_sd_instance(after.instance()));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn trusted_agents_connect_and_reconnect_with_fresh_session_authority() {
    let (left_state, right_state) = reciprocal_identities();
    let right_device_id = right_state.local_credential().device_id();
    let left_signer = Arc::new(SigningKey::from_secret_bytes([0x74; 32]));
    let right_signer = Arc::new(SigningKey::from_secret_bytes([0x75; 32]));
    let capability = CapabilityId::parse("files.transfer").unwrap();
    let operation = OperationName::parse("receive").unwrap();
    let mut policy = PolicyState::new();
    policy
        .set_rule_effect(
            right_device_id,
            capability.clone(),
            operation.clone(),
            RuleEffect::Allow,
        )
        .unwrap();

    let left =
        TrustedPresenceAgent::spawn_with_policy(left_state, left_signer, policy.clone()).unwrap();
    let right = TrustedPresenceAgent::spawn(right_state, right_signer).unwrap();
    assert_eq!(left.permission_snapshot().policy_revision(), 1);
    assert_eq!(left.permission_snapshot().rules().len(), 1);

    let left_route = route_for(&left);
    let right_route = route_for(&right);
    left.candidate_available(right_route).unwrap();
    right.candidate_available(left_route).unwrap();

    let mut left_status = left.subscribe_status();
    let mut right_status = right.subscribe_status();
    let first_left = wait_online(&mut left_status).await;
    let first_right = wait_online(&mut right_status).await;

    let first_session = first_left
        .runtime()
        .and_then(|status| status.session_id())
        .expect("left session should have active session id");
    assert_eq!(
        Some(first_session),
        first_right.runtime().and_then(|status| status.session_id())
    );

    let mut permissions = left.subscribe_permissions();
    policy
        .set_rule_effect(
            right_device_id,
            capability,
            operation,
            RuleEffect::Deny,
        )
        .unwrap();
    left.replace_policy(policy).unwrap();
    tokio::time::timeout(WAIT, permissions.changed())
        .await
        .expect("permission state should update")
        .expect("permission channel should remain open");
    assert_eq!(permissions.borrow().policy_revision(), 2);
    assert_eq!(permissions.borrow().rules()[0].effect(), RuleEffect::Deny);

    left.disconnect().unwrap();
    wait_phase(&mut left_status, PresencePhase::Paused).await;
    left.reconnect().unwrap();

    let second_left = wait_online_with_new_session(&mut left_status, first_session).await;
    let second_right = wait_online_with_new_session(&mut right_status, first_session).await;

    let second_session = second_left
        .runtime()
        .and_then(|status| status.session_id())
        .expect("reconnected session should have active session id");
    assert_ne!(first_session, second_session);
    assert_eq!(
        Some(second_session),
        second_right
            .runtime()
            .and_then(|status| status.session_id())
    );
    assert_eq!(left.permission_snapshot().policy_revision(), 2);
    assert_eq!(
        left.permission_snapshot().rules()[0].effect(),
        RuleEffect::Deny
    );
}

fn reciprocal_identities() -> (ProductIdentityState, ProductIdentityState) {
    let owner_id = OwnerId::from_bytes([0x70; 32]);
    let root_key = SigningKey::from_secret_bytes([0x71; 32]);
    let issuer = SigningKey::from_secret_bytes([0x72; 32]);
    let root = OwnerRootRecord::new(owner_id, &root_key, 0);
    let delegation = AuthorityDelegation::issue(
        owner_id,
        AuthorityRole::DeviceSigning,
        &issuer,
        0,
        &root_key,
    );
    let mut authority = OwnerAuthorityState::new(root);
    authority.accept_delegation(delegation).unwrap();

    let left_key = SigningKey::from_secret_bytes([0x74; 32]);
    let right_key = SigningKey::from_secret_bytes([0x75; 32]);
    let left_credential = DeviceCredential::issue(
        owner_id,
        DeviceId::from_bytes([0x76; 32]),
        &left_key,
        0,
        &authority,
        &issuer,
    )
    .unwrap();
    let right_credential = DeviceCredential::issue(
        owner_id,
        DeviceId::from_bytes([0x77; 32]),
        &right_key,
        0,
        &authority,
        &issuer,
    )
    .unwrap();

    let left_transition = PairingTrustTransition::issue(
        &left_credential,
        TransitionId::from_bytes([0x78; 32]),
        [0x79; 32],
        &authority,
        &issuer,
    )
    .unwrap();
    let right_transition = PairingTrustTransition::issue(
        &right_credential,
        TransitionId::from_bytes([0x7a; 32]),
        [0x7b; 32],
        &authority,
        &issuer,
    )
    .unwrap();

    let mut left =
        ProductIdentityState::join_owner_domain(root, delegation, left_credential, &left_key)
            .unwrap();
    left.add_paired_peer(right_credential, right_transition)
        .unwrap();

    let mut right =
        ProductIdentityState::join_owner_domain(root, delegation, right_credential, &right_key)
            .unwrap();
    right
        .add_paired_peer(left_credential, left_transition)
        .unwrap();

    (left, right)
}

fn route_for(agent: &TrustedPresenceAgent) -> TrustedSessionRoute {
    TrustedSessionRoute::new(
        agent.discovery().instance().to_owned(),
        SocketAddr::from(([127, 0, 0, 1], agent.discovery().listen_port())),
    )
    .unwrap()
}

async fn wait_online(status: &mut watch::Receiver<PresenceSnapshot>) -> PresenceSnapshot {
    wait_for(status, |snapshot| snapshot.phase() == PresencePhase::Online).await
}

async fn wait_online_with_new_session(
    status: &mut watch::Receiver<PresenceSnapshot>,
    previous: crosslab_policy::SessionId,
) -> PresenceSnapshot {
    wait_for(status, |snapshot| {
        snapshot.phase() == PresencePhase::Online
            && snapshot
                .runtime()
                .and_then(|runtime| runtime.session_id())
                .is_some_and(|session| session != previous)
    })
    .await
}

async fn wait_phase(
    status: &mut watch::Receiver<PresenceSnapshot>,
    phase: PresencePhase,
) -> PresenceSnapshot {
    wait_for(status, |snapshot| snapshot.phase() == phase).await
}

async fn wait_for(
    status: &mut watch::Receiver<PresenceSnapshot>,
    predicate: impl Fn(&PresenceSnapshot) -> bool,
) -> PresenceSnapshot {
    tokio::time::timeout(WAIT, async {
        loop {
            let snapshot = status.borrow_and_update().clone();
            if predicate(&snapshot) {
                return snapshot;
            }
            status
                .changed()
                .await
                .expect("presence agent should remain available");
        }
    })
    .await
    .expect("presence state should converge before timeout")
}
