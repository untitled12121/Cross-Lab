use std::num::NonZeroUsize;

use crosslab_core::{LogicalSession, SessionError};
use crosslab_policy::{NetworkClass, PolicyState};
use crosslab_sim::{
    runtime::{NodeError, RuntimeNode},
    transport::MemoryTransportPair,
};

#[test]
fn runtime_node_rejects_non_active_session() {
    let pair = MemoryTransportPair::new(NonZeroUsize::new(4).unwrap(), [0x31; 32]);
    let (endpoint, _) = pair.endpoints();

    let result = RuntimeNode::new(
        LogicalSession::new(),
        endpoint,
        PolicyState::new(),
        Vec::new(),
        NetworkClass::Local,
        NonZeroUsize::new(4).unwrap(),
    );

    assert!(matches!(
        result,
        Err(NodeError::Session(SessionError::InvalidState))
    ));
}
