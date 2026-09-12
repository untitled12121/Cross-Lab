use std::num::NonZeroUsize;

use crosslab_core::{
    ControlReceiveError, ControlSendError, TransportConnection, TransportSecurityClass,
};
use crosslab_sim::transport::{MemorySide, MemoryTransportConfig, MemoryTransportPair};

fn pair(capacity: usize, binding: u8) -> MemoryTransportPair {
    MemoryTransportPair::new(NonZeroUsize::new(capacity).unwrap(), [binding; 32])
}

fn nonzero(value: usize) -> NonZeroUsize {
    NonZeroUsize::new(value).unwrap()
}

#[test]
fn bounded_control_delivery_is_ordered() {
    let pair = pair(3, 0x21);
    let (a, b) = pair.endpoints();

    a.try_send_control(vec![1]).unwrap();
    a.try_send_control(vec![2]).unwrap();
    a.try_send_control(vec![3]).unwrap();

    assert_eq!(b.try_receive_control().unwrap(), vec![1]);
    assert_eq!(b.try_receive_control().unwrap(), vec![2]);
    assert_eq!(b.try_receive_control().unwrap(), vec![3]);
    assert_eq!(b.try_receive_control(), Err(ControlReceiveError::Empty));
}

#[test]
fn saturated_control_queue_applies_backpressure_without_losing_the_frame() {
    let pair = pair(1, 0x22);
    let (a, b) = pair.endpoints();

    a.try_send_control(vec![1]).unwrap();
    let frame = match a.try_send_control(vec![2]) {
        Err(ControlSendError::Full(frame)) => frame,
        other => panic!("expected bounded queue saturation, got {other:?}"),
    };

    assert_eq!(b.try_receive_control().unwrap(), vec![1]);
    a.try_send_control(frame).unwrap();
    assert_eq!(b.try_receive_control().unwrap(), vec![2]);
}

#[test]
fn oversized_control_frame_is_rejected_without_consuming_capacity() {
    let config = MemoryTransportConfig::new(nonzero(1), nonzero(1), nonzero(1), nonzero(8))
        .with_frame_limits(nonzero(4), nonzero(8));
    let pair = MemoryTransportPair::with_config(config, [0x26; 32]);
    let (a, b) = pair.endpoints();

    let frame = vec![0x11; 5];
    assert_eq!(
        a.try_send_control(frame.clone()),
        Err(ControlSendError::TooLarge(frame))
    );
    assert_eq!(b.try_receive_control(), Err(ControlReceiveError::Empty));

    a.try_send_control(vec![0x22; 4]).unwrap();
    assert_eq!(b.try_receive_control().unwrap(), vec![0x22; 4]);
}

#[test]
fn injected_disconnect_abandons_queued_work_and_closes_both_endpoints() {
    let pair = pair(2, 0x23);
    let (a, b) = pair.endpoints();

    a.try_send_control(vec![1]).unwrap();
    pair.faults().disconnect_now();

    assert!(a.is_closed());
    assert!(b.is_closed());
    assert_eq!(b.try_receive_control(), Err(ControlReceiveError::Closed));
    assert_eq!(
        a.try_send_control(vec![2]),
        Err(ControlSendError::Closed(vec![2]))
    );
}

#[test]
fn connection_close_is_idempotent_and_propagates_to_the_peer() {
    let pair = pair(2, 0x24);
    let (a, b) = pair.endpoints();

    a.close();
    a.close();

    assert!(a.is_closed());
    assert!(b.is_closed());
    assert_eq!(b.try_receive_control(), Err(ControlReceiveError::Closed));
    assert_eq!(
        b.try_send_control(vec![1]),
        Err(ControlSendError::Closed(vec![1]))
    );
}

#[test]
fn one_direction_can_be_closed_without_closing_the_reverse_direction() {
    let pair = pair(2, 0x25);
    let (a, b) = pair.endpoints();

    pair.faults().close_outbound(MemorySide::A);

    assert!(!a.is_closed());
    assert!(!b.is_closed());
    assert_eq!(
        a.try_send_control(vec![1]),
        Err(ControlSendError::Closed(vec![1]))
    );
    b.try_send_control(vec![2]).unwrap();
    assert_eq!(a.try_receive_control().unwrap(), vec![2]);
    assert_eq!(b.try_receive_control(), Err(ControlReceiveError::Closed));
}

#[test]
fn channel_binding_and_metadata_are_connection_scoped_and_transport_neutral() {
    let first = pair(1, 0x31);
    let second = pair(1, 0x32);
    let (first_a, first_b) = first.endpoints();
    let (second_a, _) = second.endpoints();

    assert_eq!(
        first_a.security_class(),
        TransportSecurityClass::InProcessTest
    );
    assert_eq!(
        first_b.security_class(),
        TransportSecurityClass::InProcessTest
    );
    assert_eq!(first_a.channel_binding(), first_b.channel_binding());
    assert_eq!(first_a.channel_binding().profile_id(), "in-process-test");
    assert_eq!(first_a.channel_binding().bytes(), &[0x31; 32]);
    assert_ne!(first_a.channel_binding(), second_a.channel_binding());

    assert_eq!(
        first_a.connection_metadata().local_endpoint(),
        Some("memory:a")
    );
    assert_eq!(
        first_a.connection_metadata().remote_endpoint(),
        Some("memory:b")
    );
    assert_eq!(first_a.connection_metadata().metered(), Some(false));
    assert_eq!(
        first_b.connection_metadata().local_endpoint(),
        Some("memory:b")
    );
    assert_eq!(
        first_b.connection_metadata().remote_endpoint(),
        Some("memory:a")
    );
}
