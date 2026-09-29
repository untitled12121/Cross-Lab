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
use crate::{
    ClipboardAvailability, ClipboardOperationError, ClipboardRequest, FileTransferAvailability,
    FileTransferCancellation, FileTransferOperationError,
};

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
async fn file_transfer_ready_mints_fresh_runtime_authority() {
    let (left_state, right_state) = reciprocal_identities();
    let left_device_id = left_state.local_credential().device_id();
    let left_signer = Arc::new(SigningKey::from_secret_bytes([0x74; 32]));
    let right_signer = Arc::new(SigningKey::from_secret_bytes([0x75; 32]));
    let mut right_policy = PolicyState::new();
    right_policy
        .set_rule_effect(
            left_device_id,
            CapabilityId::parse("files.transfer").unwrap(),
            OperationName::parse("receive").unwrap(),
            RuleEffect::Allow,
        )
        .unwrap();

    let left = TrustedPresenceAgent::spawn_with_policy_and_capabilities(
        left_state,
        left_signer,
        PolicyState::new(),
        ClipboardAvailability::default(),
        FileTransferAvailability::new(true),
    )
    .unwrap();
    let right = TrustedPresenceAgent::spawn_with_policy_and_capabilities(
        right_state,
        right_signer,
        right_policy,
        ClipboardAvailability::default(),
        FileTransferAvailability::new(true),
    )
    .unwrap();
    let mut right_requests = right.take_file_transfer_requests().unwrap();

    left.candidate_available(route_for(&right)).unwrap();
    right.candidate_available(route_for(&left)).unwrap();
    let mut left_status = left.subscribe_status();
    wait_capability_negotiated(&mut left_status, "files.transfer").await;

    let offer = crosslab_protocol::FileTransferOffer::new(
        crosslab_protocol::TransferId::from_bytes([0xb1; 32]),
        "ready.bin".into(),
        crosslab_protocol::FILE_TRANSFER_CHECKPOINT_BYTES + 3,
        crosslab_protocol::FileTransferDigest::from_bytes([0xb2; 32]),
    )
    .unwrap();

    let first = left.send_file_offer(offer.clone());
    tokio::pin!(first);
    let first_request = tokio::time::timeout(WAIT, async {
        tokio::select! {
            result = &mut first => panic!("first file offer completed before peer request: {result:?}"),
            request = right_requests.recv() => request,
        }
    })
    .await
    .expect("first offer should arrive")
    .expect("file transfer request channel should remain open");
    right
        .complete_file_transfer_ready(
            first_request.request_id(),
            crosslab_protocol::FILE_TRANSFER_CHECKPOINT_BYTES,
        )
        .await
        .unwrap();
    let first_operation = match first.await.unwrap() {
        crosslab_protocol::FileTransferAcceptance::Ready {
            resume_offset,
            operation_id,
            ..
        } => {
            assert_eq!(
                resume_offset,
                crosslab_protocol::FILE_TRANSFER_CHECKPOINT_BYTES
            );
            operation_id
        }
        crosslab_protocol::FileTransferAcceptance::AlreadyComplete { .. } => {
            panic!("expected Ready acceptance")
        }
    };

    let second = left.send_file_offer(offer);
    tokio::pin!(second);
    let second_request = tokio::time::timeout(WAIT, async {
        tokio::select! {
            result = &mut second => panic!("second file offer completed before peer request: {result:?}"),
            request = right_requests.recv() => request,
        }
    })
    .await
    .expect("second offer should arrive")
    .expect("file transfer request channel should remain open");
    right
        .complete_file_transfer_ready(second_request.request_id(), 0)
        .await
        .unwrap();
    let second_operation = match second.await.unwrap() {
        crosslab_protocol::FileTransferAcceptance::Ready { operation_id, .. } => operation_id,
        crosslab_protocol::FileTransferAcceptance::AlreadyComplete { .. } => {
            panic!("expected Ready acceptance")
        }
    };
    assert_ne!(first_operation, second_operation);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn file_transfer_data_plane_correlates_bytes_and_terminal_result() {
    let (left_state, right_state) = reciprocal_identities();
    let left_device_id = left_state.local_credential().device_id();
    let left_signer = Arc::new(SigningKey::from_secret_bytes([0x74; 32]));
    let right_signer = Arc::new(SigningKey::from_secret_bytes([0x75; 32]));
    let mut right_policy = PolicyState::new();
    right_policy
        .set_rule_effect(
            left_device_id,
            CapabilityId::parse("files.transfer").unwrap(),
            OperationName::parse("receive").unwrap(),
            RuleEffect::Allow,
        )
        .unwrap();

    let left = TrustedPresenceAgent::spawn_with_policy_and_capabilities(
        left_state,
        left_signer,
        PolicyState::new(),
        ClipboardAvailability::default(),
        FileTransferAvailability::new(true),
    )
    .unwrap();
    let right = TrustedPresenceAgent::spawn_with_policy_and_capabilities(
        right_state,
        right_signer,
        right_policy,
        ClipboardAvailability::default(),
        FileTransferAvailability::new(true),
    )
    .unwrap();
    let mut right_requests = right.take_file_transfer_requests().unwrap();
    let mut right_data = right.take_file_transfer_data().unwrap();

    left.candidate_available(route_for(&right)).unwrap();
    right.candidate_available(route_for(&left)).unwrap();
    let mut left_status = left.subscribe_status();
    wait_capability_negotiated(&mut left_status, "files.transfer").await;

    let offer = crosslab_protocol::FileTransferOffer::new(
        crosslab_protocol::TransferId::from_bytes([0xb8; 32]),
        "stream.bin".into(),
        18,
        crosslab_protocol::FileTransferDigest::from_bytes([0xb9; 32]),
    )
    .unwrap();
    let transfer_id = offer.transfer_id();

    let offer_result = left.send_file_offer(offer);
    tokio::pin!(offer_result);
    let request = tokio::time::timeout(WAIT, async {
        tokio::select! {
            result = &mut offer_result => panic!("offer completed before destination request: {result:?}"),
            request = right_requests.recv() => request,
        }
    })
    .await
    .expect("file offer should reach destination")
    .expect("file request channel should remain open");
    right
        .complete_file_transfer_ready(request.request_id(), 0)
        .await
        .unwrap();
    assert!(matches!(
        offer_result.await.unwrap(),
        crosslab_protocol::FileTransferAcceptance::Ready {
            transfer_id: received,
            resume_offset: 0,
            ..
        } if received == transfer_id
    ));

    let stream = left.open_file_transfer_stream(transfer_id).await.unwrap();
    let opened = tokio::time::timeout(WAIT, right_data.recv())
        .await
        .expect("destination should observe stream open")
        .expect("file data channel should remain open");
    assert!(matches!(
        opened,
        crate::FileTransferDataEvent::Opened {
            transfer_id: received,
            stream_id,
            resume_offset: 0,
        } if received == transfer_id && stream_id == stream.stream_id()
    ));

    let payload = b"private-file-bytes".to_vec();
    left.send_file_transfer_chunk(stream, payload.clone())
        .await
        .unwrap();
    let chunk = tokio::time::timeout(WAIT, right_data.recv())
        .await
        .expect("destination should receive file chunk")
        .expect("file data channel should remain open");
    assert!(matches!(
        chunk,
        crate::FileTransferDataEvent::Chunk(ref chunk)
            if chunk.transfer_id() == transfer_id
                && chunk.stream_id() == stream.stream_id()
                && chunk.bytes() == payload
    ));
    assert!(!format!("{chunk:?}").contains("private-file-bytes"));

    let finish = left.finish_file_transfer_stream(stream);
    tokio::pin!(finish);
    let finished = tokio::time::timeout(WAIT, async {
        tokio::select! {
            result = &mut finish => panic!("source completed before destination terminal result: {result:?}"),
            event = right_data.recv() => event,
        }
    })
    .await
    .expect("destination should observe stream finish")
    .expect("file data channel should remain open");
    assert!(matches!(
        finished,
        crate::FileTransferDataEvent::Finished {
            transfer_id: received,
            stream_id,
        } if received == transfer_id && stream_id == stream.stream_id()
    ));

    right
        .complete_file_transfer_result(
            transfer_id,
            crosslab_protocol::FileTransferTerminalOutcome::Completed,
        )
        .await
        .unwrap();
    let result = tokio::time::timeout(WAIT, &mut finish)
        .await
        .expect("source should receive terminal result")
        .unwrap();
    assert_eq!(result.transfer_id(), transfer_id);
    assert_eq!(
        result.outcome(),
        crosslab_protocol::FileTransferTerminalOutcome::Completed
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn file_transfer_unused_ready_authority_shares_inbound_capacity() {
    let (left_state, right_state) = reciprocal_identities();
    let left_device_id = left_state.local_credential().device_id();
    let left_signer = Arc::new(SigningKey::from_secret_bytes([0x74; 32]));
    let right_signer = Arc::new(SigningKey::from_secret_bytes([0x75; 32]));
    let mut right_policy = PolicyState::new();
    right_policy
        .set_rule_effect(
            left_device_id,
            CapabilityId::parse("files.transfer").unwrap(),
            OperationName::parse("receive").unwrap(),
            RuleEffect::Allow,
        )
        .unwrap();

    let left = TrustedPresenceAgent::spawn_with_policy_and_capabilities(
        left_state,
        left_signer,
        PolicyState::new(),
        ClipboardAvailability::default(),
        FileTransferAvailability::new(true),
    )
    .unwrap();
    let right = TrustedPresenceAgent::spawn_with_policy_and_capabilities(
        right_state,
        right_signer,
        right_policy,
        ClipboardAvailability::default(),
        FileTransferAvailability::new(true),
    )
    .unwrap();
    let mut right_requests = right.take_file_transfer_requests().unwrap();

    left.candidate_available(route_for(&right)).unwrap();
    right.candidate_available(route_for(&left)).unwrap();
    let mut left_status = left.subscribe_status();
    wait_capability_negotiated(&mut left_status, "files.transfer").await;

    for byte in 0xc0..0xc8 {
        let offer = crosslab_protocol::FileTransferOffer::new(
            crosslab_protocol::TransferId::from_bytes([byte; 32]),
            "bounded.bin".into(),
            1,
            crosslab_protocol::FileTransferDigest::from_bytes([byte.wrapping_add(1); 32]),
        )
        .unwrap();
        let send = left.send_file_offer(offer);
        tokio::pin!(send);
        let request = tokio::time::timeout(WAIT, async {
            tokio::select! {
                result = &mut send => panic!("bounded file offer completed before platform dispatch: {result:?}"),
                request = right_requests.recv() => request,
            }
        })
        .await
        .expect("bounded offer should reach platform")
        .expect("file transfer request channel should remain open");
        right
            .complete_file_transfer_ready(request.request_id(), 0)
            .await
            .unwrap();
        assert!(matches!(
            send.await.unwrap(),
            crosslab_protocol::FileTransferAcceptance::Ready { .. }
        ));
    }

    let overflow = crosslab_protocol::FileTransferOffer::new(
        crosslab_protocol::TransferId::from_bytes([0xd0; 32]),
        "overflow.bin".into(),
        1,
        crosslab_protocol::FileTransferDigest::from_bytes([0xd1; 32]),
    )
    .unwrap();

    assert_eq!(
        left.send_file_offer(overflow).await,
        Err(FileTransferOperationError::Remote(
            crosslab_protocol::ProtocolErrorCode::ResourceLimit
        ))
    );
    assert!(
        tokio::time::timeout(Duration::from_millis(100), right_requests.recv())
            .await
            .is_err(),
        "overflow offer must be rejected before platform dispatch"
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn file_transfer_offer_is_bounded_correlated_and_cancelled_on_disconnect() {
    let (left_state, right_state) = reciprocal_identities();
    let left_device_id = left_state.local_credential().device_id();
    let left_signer = Arc::new(SigningKey::from_secret_bytes([0x74; 32]));
    let right_signer = Arc::new(SigningKey::from_secret_bytes([0x75; 32]));
    let mut right_policy = PolicyState::new();
    right_policy
        .set_rule_effect(
            left_device_id,
            CapabilityId::parse("files.transfer").unwrap(),
            OperationName::parse("receive").unwrap(),
            RuleEffect::Allow,
        )
        .unwrap();

    let left = TrustedPresenceAgent::spawn_with_policy_and_capabilities(
        left_state,
        left_signer,
        PolicyState::new(),
        ClipboardAvailability::default(),
        FileTransferAvailability::new(true),
    )
    .unwrap();
    let right = TrustedPresenceAgent::spawn_with_policy_and_capabilities(
        right_state,
        right_signer,
        right_policy,
        ClipboardAvailability::default(),
        FileTransferAvailability::new(true),
    )
    .unwrap();
    let mut right_requests = right.take_file_transfer_requests().unwrap();
    let mut right_cancellations = right
        .take_file_transfer_cancellations()
        .unwrap();

    left.candidate_available(route_for(&right)).unwrap();
    right.candidate_available(route_for(&left)).unwrap();
    let mut left_status = left.subscribe_status();
    wait_capability_negotiated(&mut left_status, "files.transfer").await;

    let offer = crosslab_protocol::FileTransferOffer::new(
        crosslab_protocol::TransferId::from_bytes([0xa1; 32]),
        "example.txt".into(),
        7,
        crosslab_protocol::FileTransferDigest::from_bytes([0xa2; 32]),
    )
    .unwrap();
    let send = left.send_file_offer(offer.clone());
    tokio::pin!(send);
    let inbound = tokio::time::timeout(WAIT, async {
        tokio::select! {
            result = &mut send => panic!("file offer completed before peer request: {result:?}"),
            request = right_requests.recv() => request,
        }
    })
    .await
    .expect("file offer should arrive")
    .expect("file transfer request channel should remain open");

    assert_eq!(inbound.source_device_id(), left_device_id);
    assert_eq!(inbound.offer(), &offer);
    right
        .complete_file_transfer_already_complete(inbound.request_id())
        .await
        .unwrap();
    assert!(matches!(
        send.await.unwrap(),
        crosslab_protocol::FileTransferAcceptance::AlreadyComplete { transfer_id }
            if transfer_id == offer.transfer_id()
    ));

    let second = left.send_file_offer(offer);
    tokio::pin!(second);
    let second_inbound = tokio::time::timeout(WAIT, async {
        tokio::select! {
            result = &mut second => panic!("second file offer completed before peer request: {result:?}"),
            request = right_requests.recv() => request,
        }
    })
    .await
    .expect("second file offer should arrive")
    .expect("file transfer request channel should remain open");
    let second_request_id = second_inbound.request_id();
    let second_transfer_id = second_inbound.offer().transfer_id();

    left.disconnect().unwrap();
    assert_eq!(second.await, Err(FileTransferOperationError::Cancelled));

    let cancellation = tokio::time::timeout(WAIT, right_cancellations.recv())
        .await
        .expect("destination should clear the pending offer when the session closes")
        .expect("file transfer cancellation channel should remain open");
    assert_eq!(
        cancellation,
        FileTransferCancellation::Request {
            request_id: second_request_id,
            transfer_id: second_transfer_id,
        }
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn file_transfer_explicit_offer_cancel_releases_retry_identity() {
    let (left_state, right_state) = reciprocal_identities();
    let left_device_id = left_state.local_credential().device_id();
    let left_signer = Arc::new(SigningKey::from_secret_bytes([0x74; 32]));
    let right_signer = Arc::new(SigningKey::from_secret_bytes([0x75; 32]));
    let mut right_policy = PolicyState::new();
    right_policy
        .set_rule_effect(
            left_device_id,
            CapabilityId::parse("files.transfer").unwrap(),
            OperationName::parse("receive").unwrap(),
            RuleEffect::Allow,
        )
        .unwrap();

    let left = TrustedPresenceAgent::spawn_with_policy_and_capabilities(
        left_state,
        left_signer,
        PolicyState::new(),
        ClipboardAvailability::default(),
        FileTransferAvailability::new(true),
    )
    .unwrap();
    let right = TrustedPresenceAgent::spawn_with_policy_and_capabilities(
        right_state,
        right_signer,
        right_policy,
        ClipboardAvailability::default(),
        FileTransferAvailability::new(true),
    )
    .unwrap();
    let mut right_requests = right.take_file_transfer_requests().unwrap();
    let mut right_cancellations = right
        .take_file_transfer_cancellations()
        .unwrap();

    left.candidate_available(route_for(&right)).unwrap();
    right.candidate_available(route_for(&left)).unwrap();
    let mut left_status = left.subscribe_status();
    wait_capability_negotiated(&mut left_status, "files.transfer").await;

    let offer = crosslab_protocol::FileTransferOffer::new(
        crosslab_protocol::TransferId::from_bytes([0xe1; 32]),
        "cancel-retry.bin".into(),
        7,
        crosslab_protocol::FileTransferDigest::from_bytes([0xe2; 32]),
    )
    .unwrap();
    let transfer_id = offer.transfer_id();

    let first = left.send_file_offer(offer.clone());
    tokio::pin!(first);
    let first_request = tokio::time::timeout(WAIT, async {
        tokio::select! {
            result = &mut first => panic!("file offer completed before peer request: {result:?}"),
            request = right_requests.recv() => request,
        }
    })
    .await
    .expect("first file offer should arrive")
    .expect("file transfer request channel should remain open");

    left.cancel_file_transfer_offer(transfer_id).await.unwrap();
    assert_eq!(first.await, Err(FileTransferOperationError::Cancelled));

    let cancellation = tokio::time::timeout(WAIT, right_cancellations.recv())
        .await
        .expect("destination should observe explicit offer cancellation")
        .expect("file transfer cancellation channel should remain open");
    assert_eq!(
        cancellation,
        FileTransferCancellation::Request {
            request_id: first_request.request_id(),
            transfer_id,
        }
    );
    assert_eq!(
        right
            .complete_file_transfer_ready(first_request.request_id(), 0)
            .await,
        Err(FileTransferOperationError::Cancelled)
    );

    let retry = left.send_file_offer(offer);
    tokio::pin!(retry);
    let retry_request = tokio::time::timeout(WAIT, async {
        tokio::select! {
            result = &mut retry => panic!("retry completed before peer request: {result:?}"),
            request = right_requests.recv() => request,
        }
    })
    .await
    .expect("retry should arrive without waiting for offer timeout")
    .expect("file transfer request channel should remain open");
    assert_ne!(retry_request.request_id(), first_request.request_id());

    right
        .complete_file_transfer_already_complete(retry_request.request_id())
        .await
        .unwrap();
    assert!(matches!(
        retry.await.unwrap(),
        crosslab_protocol::FileTransferAcceptance::AlreadyComplete { transfer_id: received }
            if received == transfer_id
    ));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn file_transfer_session_loss_interrupts_ready_destination_transfer() {
    let (left_state, right_state) = reciprocal_identities();
    let left_device_id = left_state.local_credential().device_id();
    let left_signer = Arc::new(SigningKey::from_secret_bytes([0x74; 32]));
    let right_signer = Arc::new(SigningKey::from_secret_bytes([0x75; 32]));
    let mut right_policy = PolicyState::new();
    right_policy
        .set_rule_effect(
            left_device_id,
            CapabilityId::parse("files.transfer").unwrap(),
            OperationName::parse("receive").unwrap(),
            RuleEffect::Allow,
        )
        .unwrap();

    let left = TrustedPresenceAgent::spawn_with_policy_and_capabilities(
        left_state,
        left_signer,
        PolicyState::new(),
        ClipboardAvailability::default(),
        FileTransferAvailability::new(true),
    )
    .unwrap();
    let right = TrustedPresenceAgent::spawn_with_policy_and_capabilities(
        right_state,
        right_signer,
        right_policy,
        ClipboardAvailability::default(),
        FileTransferAvailability::new(true),
    )
    .unwrap();
    let mut right_requests = right.take_file_transfer_requests().unwrap();
    let mut right_cancellations = right.take_file_transfer_cancellations().unwrap();

    left.candidate_available(route_for(&right)).unwrap();
    right.candidate_available(route_for(&left)).unwrap();
    let mut left_status = left.subscribe_status();
    wait_capability_negotiated(&mut left_status, "files.transfer").await;

    let offer = crosslab_protocol::FileTransferOffer::new(
        crosslab_protocol::TransferId::from_bytes([0xe5; 32]),
        "ready-disconnect.bin".into(),
        13,
        crosslab_protocol::FileTransferDigest::from_bytes([0xe6; 32]),
    )
    .unwrap();
    let transfer_id = offer.transfer_id();

    let send = left.send_file_offer(offer);
    tokio::pin!(send);
    let request = tokio::time::timeout(WAIT, async {
        tokio::select! {
            result = &mut send => panic!("file offer completed before peer request: {result:?}"),
            request = right_requests.recv() => request,
        }
    })
    .await
    .expect("file offer should arrive")
    .expect("file transfer request channel should remain open");
    right
        .complete_file_transfer_ready(request.request_id(), 0)
        .await
        .unwrap();
    assert!(matches!(
        send.await.unwrap(),
        crosslab_protocol::FileTransferAcceptance::Ready {
            transfer_id: received,
            ..
        } if received == transfer_id
    ));

    left.disconnect().unwrap();

    let cancellation = tokio::time::timeout(WAIT, right_cancellations.recv())
        .await
        .expect("session loss should interrupt accepted destination transfer state")
        .expect("file transfer cancellation channel should remain open");
    assert_eq!(
        cancellation,
        FileTransferCancellation::Transfer { transfer_id }
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn file_transfer_cancel_after_ready_consumes_remote_authority() {
    let (left_state, right_state) = reciprocal_identities();
    let left_device_id = left_state.local_credential().device_id();
    let left_signer = Arc::new(SigningKey::from_secret_bytes([0x74; 32]));
    let right_signer = Arc::new(SigningKey::from_secret_bytes([0x75; 32]));
    let mut right_policy = PolicyState::new();
    right_policy
        .set_rule_effect(
            left_device_id,
            CapabilityId::parse("files.transfer").unwrap(),
            OperationName::parse("receive").unwrap(),
            RuleEffect::Allow,
        )
        .unwrap();

    let left = TrustedPresenceAgent::spawn_with_policy_and_capabilities(
        left_state,
        left_signer,
        PolicyState::new(),
        ClipboardAvailability::default(),
        FileTransferAvailability::new(true),
    )
    .unwrap();
    let right = TrustedPresenceAgent::spawn_with_policy_and_capabilities(
        right_state,
        right_signer,
        right_policy,
        ClipboardAvailability::default(),
        FileTransferAvailability::new(true),
    )
    .unwrap();
    let mut right_requests = right.take_file_transfer_requests().unwrap();
    let mut right_data = right.take_file_transfer_data().unwrap();

    left.candidate_available(route_for(&right)).unwrap();
    right.candidate_available(route_for(&left)).unwrap();
    let mut left_status = left.subscribe_status();
    wait_capability_negotiated(&mut left_status, "files.transfer").await;

    let offer = crosslab_protocol::FileTransferOffer::new(
        crosslab_protocol::TransferId::from_bytes([0xe3; 32]),
        "cancel-ready.bin".into(),
        11,
        crosslab_protocol::FileTransferDigest::from_bytes([0xe4; 32]),
    )
    .unwrap();
    let transfer_id = offer.transfer_id();

    let send = left.send_file_offer(offer.clone());
    tokio::pin!(send);
    let request = tokio::time::timeout(WAIT, async {
        tokio::select! {
            result = &mut send => panic!("file offer completed before peer request: {result:?}"),
            request = right_requests.recv() => request,
        }
    })
    .await
    .expect("file offer should arrive")
    .expect("file transfer request channel should remain open");
    right
        .complete_file_transfer_ready(request.request_id(), 0)
        .await
        .unwrap();
    assert!(matches!(
        send.await.unwrap(),
        crosslab_protocol::FileTransferAcceptance::Ready {
            transfer_id: received,
            ..
        } if received == transfer_id
    ));

    left.cancel_file_transfer_offer(transfer_id).await.unwrap();

    let opened = tokio::time::timeout(WAIT, right_data.recv())
        .await
        .expect("ready cancellation should consume the issued authority")
        .expect("file transfer data channel should remain open");
    let stream_id = match opened {
        crate::FileTransferDataEvent::Opened {
            transfer_id: received,
            stream_id,
            resume_offset: 0,
        } if received == transfer_id => stream_id,
        other => panic!("expected cancellation stream open, got {other:?}"),
    };
    let cancelled = tokio::time::timeout(WAIT, right_data.recv())
        .await
        .expect("ready cancellation should cancel the opened stream")
        .expect("file transfer data channel should remain open");
    assert!(matches!(
        cancelled,
        crate::FileTransferDataEvent::Cancelled {
            transfer_id: received,
            stream_id: cancelled_stream,
        } if received == transfer_id && cancelled_stream == stream_id
    ));
    right
        .complete_file_transfer_result(
            transfer_id,
            crosslab_protocol::FileTransferTerminalOutcome::Cancelled,
        )
        .await
        .unwrap();

    let retry = left.send_file_offer(offer);
    tokio::pin!(retry);
    let retry_request = tokio::time::timeout(WAIT, async {
        tokio::select! {
            result = &mut retry => panic!("retry completed before peer request: {result:?}"),
            request = right_requests.recv() => request,
        }
    })
    .await
    .expect("retry should arrive after ready cancellation")
    .expect("file transfer request channel should remain open");
    right
        .complete_file_transfer_already_complete(retry_request.request_id())
        .await
        .unwrap();
    assert!(matches!(
        retry.await.unwrap(),
        crosslab_protocol::FileTransferAcceptance::AlreadyComplete { transfer_id: received }
            if received == transfer_id
    ));
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

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn clipboard_pending_work_is_cancelled_on_policy_replacement() {
    let (left_state, right_state) = reciprocal_identities();
    let left_device_id = left_state.local_credential().device_id();
    let right_device_id = right_state.local_credential().device_id();
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

    let send = left.send_clipboard_text("ephemeral policy work".into());
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

    let mut replacement = PolicyState::new();
    replacement
        .set_rule_effect(
            right_device_id,
            CapabilityId::parse("clipboard.read").unwrap(),
            OperationName::parse("get").unwrap(),
            RuleEffect::Deny,
        )
        .unwrap();
    left.replace_policy(replacement).unwrap();

    assert_eq!(
        tokio::time::timeout(WAIT, &mut send)
            .await
            .expect("policy replacement should cancel pending clipboard work"),
        Err(ClipboardOperationError::Cancelled)
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn clipboard_pending_work_does_not_cross_reconnect() {
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
    let mut right_status = right.subscribe_status();
    let first = wait_clipboard_negotiated(&mut left_status).await;
    wait_clipboard_negotiated(&mut right_status).await;
    let first_session = first
        .runtime()
        .and_then(|runtime| runtime.session_id())
        .expect("clipboard session should be active");

    let send = left.send_clipboard_text("old session".into());
    tokio::pin!(send);
    let _old_request = tokio::time::timeout(WAIT, async {
        tokio::select! {
            result = &mut send => panic!("clipboard send completed before peer request: {result:?}"),
            request = right_clipboard.recv() => request,
        }
    })
    .await
    .expect("write request should arrive")
    .expect("clipboard channel should remain open");

    left.network_lost().unwrap();
    right.network_lost().unwrap();
    assert_eq!(
        tokio::time::timeout(WAIT, &mut send)
            .await
            .expect("network loss should cancel pending clipboard work"),
        Err(ClipboardOperationError::Cancelled)
    );

    left.network_available().unwrap();
    right.network_available().unwrap();
    left.candidate_available(route_for(&right)).unwrap();
    right.candidate_available(route_for(&left)).unwrap();

    wait_online_with_new_session(&mut left_status, first_session).await;
    wait_online_with_new_session(&mut right_status, first_session).await;
    wait_clipboard_negotiated(&mut left_status).await;
    wait_clipboard_negotiated(&mut right_status).await;

    let fresh = left.send_clipboard_text("fresh session".into());
    tokio::pin!(fresh);
    let request = tokio::time::timeout(WAIT, async {
        tokio::select! {
            result = &mut fresh => panic!("fresh clipboard send completed before peer request: {result:?}"),
            request = right_clipboard.recv() => request,
        }
    })
    .await
    .expect("fresh write request should arrive")
    .expect("clipboard channel should remain open");
    let request_id = match request {
        ClipboardRequest::Write { request_id, text } => {
            assert_eq!(text, "fresh session");
            request_id
        }
        ClipboardRequest::Read { .. } => panic!("expected clipboard write"),
    };
    right
        .complete_clipboard_write(request_id, Ok(()))
        .await
        .unwrap();
    fresh.await.unwrap();
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

async fn wait_capability_negotiated(
    status: &mut watch::Receiver<PresenceSnapshot>,
    capability: &str,
) -> PresenceSnapshot {
    tokio::time::timeout(WAIT, async {
        loop {
            let snapshot = status.borrow().clone();
            if snapshot.runtime().is_some_and(|runtime| {
                runtime
                    .negotiated_capability_ids()
                    .iter()
                    .any(|id| id.as_str() == capability)
            }) {
                return snapshot;
            }
            status
                .changed()
                .await
                .expect("presence channel should stay open");
        }
    })
    .await
    .expect("capability should negotiate")
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
