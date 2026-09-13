use crosslab_policy::{CapabilityId, CapabilityVersion, OperationName};
use crosslab_protocol::{ControlRequest, RequestId, RetryClass};
use crosslab_sim::node::NodeEvent;

#[test]
fn node_event_debug_does_not_expose_request_payload() {
    let event = NodeEvent::RequestDispatched(ControlRequest::new(
        RequestId::from_bytes([0xa1; 16]),
        CapabilityId::parse("clipboard.write").unwrap(),
        CapabilityVersion::new(1, 0),
        OperationName::parse("set").unwrap(),
        RetryClass::NonRetryable,
        vec![221, 222, 223, 224],
    ));

    let debug = format!("{event:?}");
    assert!(!debug.contains("221, 222, 223, 224"), "{debug}");
}
