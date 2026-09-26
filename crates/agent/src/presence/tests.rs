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

use super::{
    PresenceAgentError, PresencePhase, PresenceSnapshot, TrustedPresenceAgent, TrustedSessionRoute,
};
use crate::{ClipboardAvailability, ClipboardOperationError, ClipboardRequest};

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
        .set_rule_effect(right_device_id, capability, operation, RuleEffect::Deny)
        .unwrap();
    left.replace_policy(policy).unwrap();
    tokio::time::timeout(WAIT, permissions.changed())
        .await
        .expect("permission state should update")
        .expect("permission channel should remain open");
    assert_eq!(permissions.borrow().policy_revision(), 2);
    assert_eq!(permissions.borrow().rules()[0].effect(), RuleEffect::Deny);
    assert_eq!(
        left.replace_policy(PolicyState::new()),
        Err(PresenceAgentError::StalePolicy)
    );

    left.disconnect().unwrap();
    wait_phase(&mut left_status, PresencePhase::Paused).await;
    left.reconnect().unwrap();
    left.candidate_available(route_for(&right)).unwrap();
    right.candidate_available(route_for(&left)).unwrap();

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

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn fail_closed_policy_advances_revision_and_removes_rules() {
    let (left_state, right_state) = reciprocal_identities();
    let right_device_id = right_state.local_credential().device_id();
    let signer = Arc::new(SigningKey::from_secret_bytes([0x74; 32]));
    let mut policy = PolicyState::new();
    policy
        .set_rule_effect(
            right_device_id,
            CapabilityId::parse("clipboard.read").unwrap(),
            OperationName::parse("get").unwrap(),
            RuleEffect::Allow,
        )
        .unwrap();

    let agent = TrustedPresenceAgent::spawn_with_policy(left_state, signer, policy).unwrap();
    let mut permissions = agent.subscribe_permissions();
    agent.fail_closed_policy().await.unwrap();
    tokio::time::timeout(WAIT, permissions.changed())
        .await
        .expect("fail-closed policy should publish")
        .expect("permission channel should remain open");

    assert_eq!(permissions.borrow().policy_revision(), 2);
    assert!(permissions.borrow().rules().is_empty());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn clipboard_v1_round_trips_explicit_write_and_read() {
    let (left_state, right_state) = reciprocal_identities();
    let left_device_id = left_state.local_credential().device_id();
    let left_signer = Arc::new(SigningKey::from_secret_bytes([0x74; 32]));
    let right_signer = Arc::new(SigningKey::from_secret_bytes([0x75; 32]));
    let mut right_policy = PolicyState::new();
    right_policy
        .set_rule_effect(
            left_device_id,
            CapabilityId::parse("clipboard.write").unwrap(),
            OperationName::parse("set").unwrap(),
            RuleEffect::Allow,
        )
        .unwrap();
    right_policy
        .set_rule_effect(
            left_device_id,
            CapabilityId::parse("clipboard.read").unwrap(),
            OperationName::parse("get").unwrap(),
            RuleEffect::Allow,
        )
        .unwrap();

    let availability = ClipboardAvailability::new(true, true);
    let left = TrustedPresenceAgent::spawn_with_policy_and_clipboard(
        left_state,
        left_signer,
        PolicyState::new(),
        availability,
    )
    .unwrap();
    let right = TrustedPresenceAgent::spawn_with_policy_and_clipboard(
        right_state,
        right_signer,
        right_policy,
        availability,
    )
    .unwrap();
    let mut right_clipboard = right.take_clipboard_requests().unwrap();

    left.candidate_available(route_for(&right)).unwrap();
    right.candidate_available(route_for(&left)).unwrap();
    let mut left_status = left.subscribe_status();
    let mut right_status = right.subscribe_status();
    wait_clipboard_negotiated(&mut left_status).await;
    wait_clipboard_negotiated(&mut right_status).await;

    let send = left.send_clipboard_text("hello from left".into());
    tokio::pin!(send);
    let write = tokio::time::timeout(WAIT, async {
        tokio::select! {
            result = &mut send => panic!("clipboard send completed before peer request: {result:?}"),
            request = right_clipboard.recv() => request,
        }
    })
    .await
    .expect("write request should arrive")
    .expect("clipboard channel should remain open");
    let request_id = match write {
        ClipboardRequest::Write { request_id, text } => {
            assert_eq!(text, "hello from left");
            request_id
        }
        ClipboardRequest::Read { .. } => panic!("expected clipboard write"),
    };
    right
        .complete_clipboard_write(request_id, Ok(()))
        .await
        .unwrap();
    send.await.unwrap();

    let fetch = left.fetch_clipboard_text();
    tokio::pin!(fetch);
    let read = tokio::time::timeout(WAIT, async {
        tokio::select! {
            result = &mut fetch => panic!("clipboard fetch completed before peer request: {result:?}"),
            request = right_clipboard.recv() => request,
        }
    })
    .await
    .expect("read request should arrive")
    .expect("clipboard channel should remain open");
    let request_id = match read {
        ClipboardRequest::Read { request_id } => request_id,
        ClipboardRequest::Write { .. } => panic!("expected clipboard read"),
    };
    right
        .complete_clipboard_read(request_id, Ok("hello from right".into()))
        .await
        .unwrap();
    assert_eq!(fetch.await.unwrap(), "hello from right");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn clipboard_pending_work_is_cancelled_on_disconnect() {
    let (left_state, right_state) = reciprocal_identities();
    let left_device_id = left_state.local_credential().device_id();
    let left_signer = Arc::new(SigningKey::from_secret_bytes([0x74; 32]));
    let right_signer = Arc::new(SigningKey::from_secret_bytes([0x75; 32]));
    let mut right_policy = PolicyState::new();
    right_policy
        .set_rule_effect(
            left_device_id,
            CapabilityId::parse("clipboard.write").unwrap(),
            OperationName::parse("set").unwrap(),
            RuleEffect::Allow,
        )
        .unwrap();

    let availability = ClipboardAvailability::new(true, true);
    let left = TrustedPresenceAgent::spawn_with_policy_and_clipboard(
        left_state,
        left_signer,
        PolicyState::new(),
        availability,
    )
    .unwrap();
    let right = TrustedPresenceAgent::spawn_with_policy_and_clipboard(
        right_state,
        right_signer,
        right_policy,
        availability,
    )
    .unwrap();
    let mut right_clipboard = right.take_clipboard_requests().unwrap();

    left.candidate_available(route_for(&right)).unwrap();
    right.candidate_available(route_for(&left)).unwrap();
    let mut left_status = left.subscribe_status();
    wait_clipboard_negotiated(&mut left_status).await;

    let send = left.send_clipboard_text("ephemeral".into());
    tokio::pin!(send);
    let _request = tokio::time::timeout(WAIT, async {
        tokio::select! {
            result = &mut send => panic!("clipboard send completed before peer request: {result:?}"),
            request = right_clipboard.recv() => request,
        }
    })
    .await
    .expect("write request should arrive")
    .expect("clipboard channel should remain open");

    left.disconnect().unwrap();
    assert_eq!(send.await, Err(ClipboardOperationError::Cancelled));
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

async fn wait_clipboard_negotiated(
    status: &mut watch::Receiver<PresenceSnapshot>,
) -> PresenceSnapshot {
    wait_for(status, |snapshot| {
        let Some(runtime) = snapshot.runtime() else {
            return false;
        };
        let capabilities = runtime.negotiated_capability_ids();
        capabilities
            .iter()
            .any(|capability| capability.as_str() == "clipboard.read")
            && capabilities
                .iter()
                .any(|capability| capability.as_str() == "clipboard.write")
    })
    .await
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
