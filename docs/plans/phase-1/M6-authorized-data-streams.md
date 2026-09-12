# M6 Authorized Data Streams Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Prove that bounded bulk payload bytes are exposed only after an active logical session admits a `DataStreamOpen` against a currently valid, locally held `AuthorizedOperation`, including single-stream and explicitly bounded multi-stream use.

**Architecture:** `crosslab-policy` remains the authority owner and adds only monotonic stream-use budgeting. `crosslab-core` owns session-bound stream admission/replay state and transport-neutral data-stream interfaces. `crosslab-sim` extends the deterministic memory transport with bounded unidirectional stream-open/chunk queues and composes S-007 without real networking or filesystem I/O.

**Tech Stack:** Rust 2024, Rust 1.98.1, existing Cross-Lab workspace crates and Prost protocol code, `std::sync::{Arc, Mutex}`, `VecDeque`, `NonZeroU32`, and `NonZeroUsize`; no new third-party dependency or general async runtime.

**Spec:** `docs/plans/phase-1/M6-authorized-data-streams-design.md`

**Architecture sources:**
- `docs/architecture/MASTER-ARCHITECTURE.md`
- `docs/architecture/CORE-SIMULATOR.md`
- `docs/architecture/SESSION-TRANSPORT.md`
- `docs/protocol/PROTOCOL-V1.md`

## Global Constraints

- Control authorizes; data carries payload. `StreamId`, transport identity, and `OperationId` alone never grant authority.
- Reuse M3 `AuthorizedOperation`, M4 `DataStreamOpen`, and M5 session/transport seams rather than creating parallel authority or wire models.
- `SingleAction` cannot authorize a data stream; `SingleStream` permits one stream; `MultiStream` is explicitly bounded and never unlimited.
- Stream-use reservation is monotonic. A successfully admitted stream spends its slot permanently; cancellation does not refund it.
- Core validates the full decoded open before payload is exposed: active session, exact `SessionId`, negotiated capability/version, operation binding, direction, index, expiry, trust revision, policy revision, and remaining stream budget.
- Core owns stream-index/replay/admission state. Policy owns only operation authority and monotonic stream-use count.
- Security-relevant replay state is never silently evicted. Capacity exhaustion returns a typed resource-limit error.
- Every pending-open queue, active-stream registry, chunk queue, and chunk byte size is explicitly bounded.
- Backpressure preserves caller ownership of unsent opening frames/chunks and `Debug` output must not reveal payload bytes.
- No Quinn/Iroh/libp2p, sockets, TLS, relays, persistence, UI/platform work, filesystem transfer, resumability, or M7 reconnect/revocation orchestration.
- No general async runtime and no busy polling.
- Research reuse is semantic only: uploaded Quinn/Iroh sources confirm graceful finish versus abortive cancellation and uni-stream open/accept lifecycle; no external types or code are copied into Cross-Lab.
- Every task follows RED -> minimal GREEN -> focused/full verification -> commit. Do not merge a task that has not passed the exact-head gates required by the current workflow.

---

### Task 1: Bounded stream-use authority in `AuthorizedOperation`

**Files:**
- Modify: `crates/policy/src/operation/mod.rs`
- Test: `crates/policy/tests/operations.rs`

**Interfaces:**
- Consumes: existing `AuthorizedOperation::validate`, `OperationUseContext`, terminal operation state, trust/policy revision snapshots.
- Produces:

```rust
pub enum UsePolicy {
    SingleAction,
    SingleStream,
    MultiStream { max_streams: NonZeroU32 },
}

pub enum OperationError {
    RandomnessUnavailable,
    InvalidLifetime,
    BindingMismatch,
    TrustRevisionChanged,
    PolicyRevisionChanged,
    StreamUseNotAllowed,
    StreamBudgetExhausted,
    Inactive(OperationState),
}

impl AuthorizedOperation {
    pub fn reserve_stream_use(
        &mut self,
        context: &OperationUseContext,
        now: u64,
        current_trust_revision: u64,
        current_policy_revision: u64,
    ) -> Result<(), OperationError>;

    pub const fn reserved_stream_uses(&self) -> u32;
    pub const fn stream_budget_exhausted(&self) -> bool;
}
```

`reserve_stream_use` calls the existing binding/expiry/revision validation first, rejects `SingleAction`, rejects an exhausted budget, and increments the reservation count only after every check succeeds. It does not consume the operation; core consumes an exhausted operation only after no admitted stream remains active.

- [ ] **Step 1: Write failing stream-budget tests**

Add focused cases to `crates/policy/tests/operations.rs`:

```rust
#[test]
fn single_action_cannot_reserve_stream_use() {
    let fixture = Fixture::new();
    let mut operation = AuthorizedOperation::issue(
        fixture.grant.clone(),
        10,
        20,
        UsePolicy::SingleAction,
    )
    .unwrap();

    assert_eq!(
        operation.reserve_stream_use(
            &fixture.use_context(),
            15,
            fixture.trust_revision,
            fixture.policy_revision,
        ),
        Err(OperationError::StreamUseNotAllowed)
    );
    assert_eq!(operation.reserved_stream_uses(), 0);
}

#[test]
fn single_stream_budget_is_monotonic_and_not_refunded() {
    let fixture = Fixture::new();
    let mut operation = AuthorizedOperation::issue(
        fixture.grant.clone(),
        10,
        20,
        UsePolicy::SingleStream,
    )
    .unwrap();

    operation
        .reserve_stream_use(
            &fixture.use_context(),
            15,
            fixture.trust_revision,
            fixture.policy_revision,
        )
        .unwrap();
    assert_eq!(operation.reserved_stream_uses(), 1);
    assert!(operation.stream_budget_exhausted());
    assert_eq!(
        operation.reserve_stream_use(
            &fixture.use_context(),
            15,
            fixture.trust_revision,
            fixture.policy_revision,
        ),
        Err(OperationError::StreamBudgetExhausted)
    );
}

#[test]
fn bounded_multi_stream_stops_at_declared_budget() {
    let fixture = Fixture::new();
    let mut operation = AuthorizedOperation::issue(
        fixture.grant.clone(),
        10,
        20,
        UsePolicy::MultiStream {
            max_streams: NonZeroU32::new(2).unwrap(),
        },
    )
    .unwrap();

    for _ in 0..2 {
        operation
            .reserve_stream_use(
                &fixture.use_context(),
                15,
                fixture.trust_revision,
                fixture.policy_revision,
            )
            .unwrap();
    }
    assert!(operation.stream_budget_exhausted());
    assert_eq!(
        operation.reserve_stream_use(
            &fixture.use_context(),
            15,
            fixture.trust_revision,
            fixture.policy_revision,
        ),
        Err(OperationError::StreamBudgetExhausted)
    );
}
```

Also assert that expiry, cancellation, revocation, and trust/policy revision changes do not increment the reserved count.

- [ ] **Step 2: Run the policy test to verify RED**

Run:

```bash
cargo test -p crosslab-policy --test operations
```

Expected: compile failure for missing `MultiStream`, `reserve_stream_use`, `reserved_stream_uses`, `stream_budget_exhausted`, and new errors.

- [ ] **Step 3: Implement the minimal policy changes**

Add `reserved_stream_uses: u32` to `AuthorizedOperation`, initialize it to zero in `issue`, and implement the signatures above. Compute the budget with a private helper:

```rust
const fn stream_budget(use_policy: UsePolicy) -> Option<u32> {
    match use_policy {
        UsePolicy::SingleAction => None,
        UsePolicy::SingleStream => Some(1),
        UsePolicy::MultiStream { max_streams } => Some(max_streams.get()),
    }
}
```

Never decrement `reserved_stream_uses`.

- [ ] **Step 4: Verify focused and workspace behavior**

Run:

```bash
cargo test -p crosslab-policy --test operations
cargo fmt --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-targets --all-features
```

Expected: all PASS.

- [ ] **Step 5: Commit the green slice**

```bash
git add crates/policy/src/operation/mod.rs crates/policy/tests/operations.rs
git commit -m "feat(policy): add bounded stream-use budgets"
```

---

### Task 2: Session-bound core stream admission

**Files:**
- Create: `crates/core/src/stream/mod.rs`
- Modify: `crates/core/src/lib.rs`
- Test: `crates/core/tests/stream_admission.rs`

**Interfaces:**
- Consumes: `LogicalSession`, `SessionState`, `SessionContext`, `NegotiatedCapability`, `DataStreamOpen`, `StreamDirection`, `StreamId`, `AuthorizedOperation`, `OperationUseContext`, Task 1 `reserve_stream_use`.
- Produces:

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StreamTerminalState {
    Finished,
    Cancelled,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AdmittedStream {
    stream_id: StreamId,
    operation_id: OperationId,
    stream_index: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StreamAdmissionError {
    InactiveSession,
    InvalidSession,
    CapabilityNotNegotiated,
    OperationNotFound,
    DuplicateOperation,
    DuplicateStreamId,
    DuplicateStreamIndex,
    InvalidStreamIndex,
    ResourceLimit,
    Operation(OperationError),
}

pub struct StreamAdmission {
    capacity: usize,
    operations: Vec<RegisteredOperation>,
    active_streams: Vec<AdmittedStream>,
}

impl StreamAdmission {
    pub fn new(capacity: NonZeroUsize) -> Self;
    pub fn register_operation(
        &mut self,
        operation: AuthorizedOperation,
    ) -> Result<(), StreamAdmissionError>;
    pub fn admit_inbound(
        &mut self,
        session: &LogicalSession,
        open: &DataStreamOpen,
        now: u64,
        current_trust_revision: u64,
        current_policy_revision: u64,
    ) -> Result<AdmittedStream, StreamAdmissionError>;
    pub fn finish_stream(&mut self, stream_id: StreamId) -> Result<(), StreamAdmissionError>;
    pub fn cancel_stream(&mut self, stream_id: StreamId) -> Result<(), StreamAdmissionError>;
    pub fn cancel_all(&mut self);
    pub fn active_stream_count(&self) -> usize;
}
```

`RegisteredOperation` stays private and owns the `AuthorizedOperation` plus used stream indices. Used indices are never silently evicted while the operation is registered. Active `StreamId` values are bounded by `capacity`; a previously terminal stream cannot recreate authority because its operation index is permanently spent.

Direction-derived `OperationUseContext` is exact:

```rust
match open.direction() {
    StreamDirection::SourceToDestination => OperationUseContext::new(
        context.peer_device_id(),
        context.local_device_id(),
        context.session_id(),
        open.capability_id().clone(),
        open.capability_version(),
        open.operation_name().clone(),
    ),
    StreamDirection::DestinationToSource => OperationUseContext::new(
        context.local_device_id(),
        context.peer_device_id(),
        context.session_id(),
        open.capability_id().clone(),
        open.capability_version(),
        open.operation_name().clone(),
    ),
}
```

`SingleStream` requires index `0`; `MultiStream { max_streams }` accepts any unique index `< max_streams`; `SingleAction` is rejected by Task 1 policy validation. Before calling `reserve_stream_use`, check session/capability, registry capacities, duplicate active `StreamId`, index validity, and duplicate index so a failed admission leaves no partially reserved authority.

- [ ] **Step 1: Write the failing admission contract**

Create `crates/core/tests/stream_admission.rs` with fixtures that build an active `LogicalSession`, negotiate `files.transfer` version `1.0`, issue/register an `AuthorizedOperation`, and exercise:

```rust
#[test]
fn valid_single_stream_is_admitted_once_and_consumed_after_finish() {
    let mut fixture = Fixture::single_stream();
    let open = fixture.open(StreamDirection::SourceToDestination, 0, [9; 16]);

    let admitted = fixture
        .admission
        .admit_inbound(
            &fixture.session,
            &open,
            15,
            fixture.trust_revision,
            fixture.policy_revision,
        )
        .unwrap();

    assert_eq!(admitted.stream_id(), open.stream_id());
    assert_eq!(fixture.admission.active_stream_count(), 1);
    fixture.admission.finish_stream(open.stream_id()).unwrap();
    assert_eq!(fixture.admission.active_stream_count(), 0);

    assert_eq!(
        fixture.admission.admit_inbound(
            &fixture.session,
            &open,
            15,
            fixture.trust_revision,
            fixture.policy_revision,
        ),
        Err(StreamAdmissionError::DuplicateStreamIndex)
    );
}
```

Add focused failures for N-035..N-039: missing operation, wrong session, wrong source/destination direction, wrong capability/version/operation, expired/cancelled/revoked/consumed operation, trust/policy revision change, duplicate index, invalid index, duplicate active `StreamId`, and registry saturation. Add multi-stream out-of-order indices `1` then `0` to prove ordering is not required.

- [ ] **Step 2: Run the core test to verify RED**

Run:

```bash
cargo test -p crosslab-core --test stream_admission
```

Expected: compile failure because `StreamAdmission`, `AdmittedStream`, and `StreamAdmissionError` do not exist.

- [ ] **Step 3: Implement focused admission state**

Create `crates/core/src/stream/mod.rs`, keep `RegisteredOperation` private, use bounded `Vec` storage, and export only the public admission types from `crates/core/src/lib.rs`.

When a stream finishes/cancels, remove it from `active_streams`. If the corresponding operation has exhausted its stream budget and no active stream still references that `OperationId`, call the existing `AuthorizedOperation::consume()`.

`cancel_all` cancels active operations and clears active stream handles/state without creating reusable stream slots.

- [ ] **Step 4: Verify admission behavior**

Run:

```bash
cargo test -p crosslab-core --test stream_admission
cargo test -p crosslab-policy --test operations
cargo fmt --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-targets --all-features
```

Expected: all PASS.

- [ ] **Step 5: Commit the green slice**

```bash
git add crates/core/src/stream/mod.rs crates/core/src/lib.rs crates/core/tests/stream_admission.rs
git commit -m "feat(core): add authorized stream admission"
```

---

### Task 3: Transport-neutral unidirectional stream seam

**Files:**
- Modify: `crates/core/src/transport/mod.rs`
- Modify: `crates/core/src/lib.rs`
- Test: `crates/core/tests/transport_stream.rs`

**Interfaces:**
- Consumes: existing `TransportConnection` control/channel-binding/close surface.
- Produces transport-neutral owned stream contracts:

```rust
pub enum StreamOpenError {
    Full(Vec<u8>),
    Closed(Vec<u8>),
}

pub enum StreamAcceptError {
    Empty,
    Closed,
}

pub enum StreamSendError {
    Full(Vec<u8>),
    TooLarge(Vec<u8>),
    Closed(Vec<u8>),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StreamReceiveError {
    Empty,
    Finished,
    Cancelled,
}

pub trait TransportSendStream: Send {
    fn try_send_chunk(&mut self, chunk: Vec<u8>) -> Result<(), StreamSendError>;
    fn finish(&mut self);
    fn cancel(&mut self);
}

pub trait TransportReceiveStream: Send {
    fn try_receive_chunk(&mut self) -> Result<Vec<u8>, StreamReceiveError>;
    fn cancel(&mut self);
}

pub struct IncomingUniStream {
    opening_frame: Vec<u8>,
    stream: Box<dyn TransportReceiveStream>,
}

impl IncomingUniStream {
    pub fn opening_frame(&self) -> &[u8];
    pub fn into_parts(self) -> (Vec<u8>, Box<dyn TransportReceiveStream>);
}
```

Extend `TransportConnection` with:

```rust
fn try_open_uni_stream(
    &self,
    opening_frame: Vec<u8>,
) -> Result<Box<dyn TransportSendStream>, StreamOpenError>;

fn try_accept_uni_stream(&self) -> Result<IncomingUniStream, StreamAcceptError>;
```

The transport treats `opening_frame` as opaque bytes. Core/protocol perform all `DataStreamOpen` interpretation.

- [ ] **Step 1: Write RED tests for ownership and redaction**

Create `crates/core/tests/transport_stream.rs` and assert that `StreamOpenError` and `StreamSendError` preserve owned bytes while their `Debug` output contains a redacted length but not a recognizable payload string:

```rust
#[test]
fn stream_send_error_preserves_payload_without_debug_leak() {
    let payload = b"crosslab-sensitive-test-payload".to_vec();
    let error = StreamSendError::Full(payload.clone());

    assert_eq!(error.into_chunk(), payload);
}
```

Define consuming helpers as part of the public contract:

```rust
impl StreamOpenError {
    pub fn into_opening_frame(self) -> Vec<u8>;
}

impl StreamSendError {
    pub fn into_chunk(self) -> Vec<u8>;
}
```

Also test `Display` text does not include payload data.

- [ ] **Step 2: Run the core transport-stream test to verify RED**

Run:

```bash
cargo test -p crosslab-core --test transport_stream
```

Expected: compile failure for the missing stream transport types/methods.

- [ ] **Step 3: Implement the narrow seam**

Add the types/methods above. Implement custom `Debug` for errors carrying byte vectors, following the existing `ControlSendError` redaction pattern. Do not add async traits, external channel types, or concrete simulator types.

- [ ] **Step 4: Verify core/workspace compatibility**

Run:

```bash
cargo test -p crosslab-core --test transport_stream
cargo fmt --check
cargo check --workspace --all-targets --all-features
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-targets --all-features
```

Expected: existing transport implementers fail during GREEN implementation until Task 4 supplies the new methods; before committing Task 3, add temporary test-only default trait methods only if required to keep the task independently green. Prefer implementing Task 3 and Task 4 in one atomic commit if the trait extension makes `crosslab-sim` uncompilable and no clean default semantics exist. Do not add fake production defaults that silently disable streams.

- [ ] **Step 5: Commit only when the workspace is green**

If Task 3 can be independently green:

```bash
git add crates/core/src/transport/mod.rs crates/core/src/lib.rs crates/core/tests/transport_stream.rs
git commit -m "feat(transport): add neutral data stream seam"
```

If the trait extension necessarily breaks the only concrete implementation, defer this commit and commit Tasks 3+4 together after Task 4 GREEN. Record that combined checkpoint in `CURRENT.md` rather than weakening the trait contract.

---

### Task 4: Bounded in-memory data streams

**Files:**
- Modify: `apps/sim/src/transport.rs`
- Test: `apps/sim/tests/memory_stream_transport.rs`
- Possibly commit together with Task 3 files if required by the trait extension.

**Interfaces:**
- Consumes: Task 3 transport-neutral traits/errors and existing `MemoryTransportPair` shared state/close behavior.
- Produces:

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MemoryTransportConfig {
    control_capacity: NonZeroUsize,
    stream_capacity: NonZeroUsize,
    chunk_capacity: NonZeroUsize,
    max_chunk_bytes: NonZeroUsize,
}

impl MemoryTransportConfig {
    pub const fn new(
        control_capacity: NonZeroUsize,
        stream_capacity: NonZeroUsize,
        chunk_capacity: NonZeroUsize,
        max_chunk_bytes: NonZeroUsize,
    ) -> Self;
}

impl MemoryTransportPair {
    pub fn with_config(config: MemoryTransportConfig, binding: [u8; 32]) -> Self;
}
```

Keep `MemoryTransportPair::new(control_capacity, binding)` working by delegating to fixed simulator defaults:

```rust
const DEFAULT_STREAM_CAPACITY: usize = 8;
const DEFAULT_CHUNK_CAPACITY: usize = 8;
const DEFAULT_MAX_CHUNK_BYTES: usize = 64 * 1024;
```

`stream_capacity` bounds both pending stream opens and live stream-state registrations per direction. Before opening a new stream, prune terminal stream states; if live/pending capacity is still exhausted, return `StreamOpenError::Full(opening_frame)`.

- [ ] **Step 1: Write failing memory stream tests**

Create `apps/sim/tests/memory_stream_transport.rs` and cover:

```rust
#[test]
fn uni_stream_open_and_chunks_are_ordered_and_bounded() {
    let pair = configured_pair(2, 2, 4);
    let (a, b) = pair.endpoints();
    let mut send = a.try_open_uni_stream(vec![1, 2, 3]).unwrap();
    let incoming = b.try_accept_uni_stream().unwrap();
    assert_eq!(incoming.opening_frame(), &[1, 2, 3]);

    let (_, mut receive) = incoming.into_parts();
    send.try_send_chunk(vec![10]).unwrap();
    send.try_send_chunk(vec![11]).unwrap();
    assert_eq!(
        send.try_send_chunk(vec![12]),
        Err(StreamSendError::Full(vec![12]))
    );
    assert_eq!(receive.try_receive_chunk().unwrap(), vec![10]);
    assert_eq!(receive.try_receive_chunk().unwrap(), vec![11]);
}
```

Add tests for pending-open saturation preserving the opening frame, max-chunk rejection preserving bytes, graceful `finish` draining queued chunks before `Finished`, abortive `cancel` clearing queued chunks and producing `Cancelled`, connection close cancelling pending/active streams, and no dangling live stream after failed open.

- [ ] **Step 2: Run the simulator stream test to verify RED**

Run:

```bash
cargo test -p crosslab-sim --test memory_stream_transport
```

Expected: compile failure for the missing config and data-stream transport implementation.

- [ ] **Step 3: Implement shared bounded stream state**

Use only `Arc<Mutex<_>>`, bounded `VecDeque`, and explicit counters/registries. A stream state must distinguish send-open, graceful-finished, and cancelled states. `finish` never clears queued chunks; `cancel` clears them. Connection `close_all` cancels every tracked data stream in addition to existing control queues.

Do not spawn tasks or poll in the background.

- [ ] **Step 4: Verify memory stream semantics and whole workspace**

Run:

```bash
cargo test -p crosslab-sim --test memory_stream_transport
cargo test -p crosslab-sim --test memory_transport
cargo fmt --check
cargo check --workspace --all-targets --all-features
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-targets --all-features
```

Expected: all PASS, including existing M5 control transport tests.

- [ ] **Step 5: Commit the transport slice**

```bash
git add crates/core/src/transport/mod.rs crates/core/src/lib.rs crates/core/tests/transport_stream.rs apps/sim/src/transport.rs apps/sim/tests/memory_stream_transport.rs
git commit -m "feat(sim): add bounded memory data streams"
```

If Task 3 already committed independently, stage only the simulator files here.

---

### Task 5: Simulator stream runtime and S-007

**Files:**
- Create: `apps/sim/src/stream.rs`
- Modify: `apps/sim/src/lib.rs`
- Test: `apps/sim/tests/stream_scenarios.rs`

**Interfaces:**
- Consumes: `LogicalSession`, Task 2 `StreamAdmission`, Task 3 stream traits, Task 4 memory transport, `encode_data_stream_open`, `decode_data_stream_open`.
- Produces feature-focused simulator composition:

```rust
pub enum SimStreamError {
    Session(SessionError),
    Wire(ProtocolWireError),
    Admission(StreamAdmissionError),
    Open(StreamOpenError),
    Accept(StreamAcceptError),
    Receive(StreamReceiveError),
    StreamNotFound,
    ResourceLimit,
}

pub struct SimStreamRuntime<'a> {
    session: &'a LogicalSession,
    transport: &'a dyn TransportConnection,
    admission: StreamAdmission,
    inbound: Vec<InboundStream>,
}

impl<'a> SimStreamRuntime<'a> {
    pub fn new(
        session: &'a LogicalSession,
        transport: &'a dyn TransportConnection,
        capacity: NonZeroUsize,
    ) -> Result<Self, SimStreamError>;
    pub fn register_operation(
        &mut self,
        operation: AuthorizedOperation,
    ) -> Result<(), SimStreamError>;
    pub fn open_uni(
        &self,
        open: &DataStreamOpen,
    ) -> Result<Box<dyn TransportSendStream>, SimStreamError>;
    pub fn accept_one(
        &mut self,
        now: u64,
        current_trust_revision: u64,
        current_policy_revision: u64,
    ) -> Result<StreamId, SimStreamError>;
    pub fn try_receive_chunk(&mut self, stream_id: StreamId) -> Result<Vec<u8>, SimStreamError>;
    pub fn cancel_stream(&mut self, stream_id: StreamId) -> Result<(), SimStreamError>;
    pub fn shutdown(&mut self);
}
```

`accept_one` accepts opaque transport bytes, decodes `DataStreamOpen`, calls core admission, and only stores/exposes the receive handle after successful admission. On wire/admission failure it cancels the receive handle before returning the error. When `try_receive_chunk` observes `Finished`, it calls `StreamAdmission::finish_stream` and removes the receive handle; on `Cancelled`, it calls `cancel_stream` and removes the handle.

- [ ] **Step 1: Write S-007 as RED**

Create `apps/sim/tests/stream_scenarios.rs`. Reuse existing M5 session/capability fixtures rather than creating a second authentication model. The positive scenario must perform:

```text
policy allow -> AuthorizationGrant -> AuthorizedOperation(SingleStream)
receiver registers operation
sender encodes/opens DataStreamOpen
receiver accepts + decodes + admits
sender writes bounded synthetic chunks
receiver sees bytes only after accept/admit
sender finishes
receiver drains and observes finish
operation becomes consumed
second stream with index 0 is rejected
```

The payload should be synthetic, for example three fixed byte chunks; do not read or write a real file.

- [ ] **Step 2: Run the S-007 test to verify RED**

Run:

```bash
cargo test -p crosslab-sim --test stream_scenarios
```

Expected: compile failure for missing `crosslab_sim::stream` runtime.

- [ ] **Step 3: Implement the minimal stream runtime**

Keep stream composition out of `SimNode`. `apps/sim/src/stream.rs` owns only the feature glue above; authorization remains in policy/core and queue behavior remains in transport.

Before storing an admitted inbound receive handle, check the runtime `capacity`; on saturation cancel the just-accepted transport stream and return `ResourceLimit` without exposing payload.

- [ ] **Step 4: Verify S-007 and regressions**

Run:

```bash
cargo test -p crosslab-sim --test stream_scenarios
cargo test -p crosslab-sim --test session_scenarios
cargo test -p crosslab-sim --test m5_scenarios
cargo fmt --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-targets --all-features
```

Expected: all PASS.

- [ ] **Step 5: Commit the simulator composition**

```bash
git add apps/sim/src/stream.rs apps/sim/src/lib.rs apps/sim/tests/stream_scenarios.rs
git commit -m "feat(sim): compose authorized data streams"
```

---

### Task 6: M6 security negatives, cancellation, fuzz/baseline, and closeout

**Files:**
- Modify: `crates/core/tests/stream_admission.rs`
- Modify: `apps/sim/tests/memory_stream_transport.rs`
- Modify: `apps/sim/tests/stream_scenarios.rs`
- Modify only if required by evidence: `fuzz/fuzz_targets/data_stream_open.rs`, `.github/workflows/fuzz.yml`
- Modify: `docs/development/CURRENT.md`

**Interfaces:**
- Consumes every M6 slice and the existing `data_stream_open` decoder fuzz target.
- Produces verified M6 closeout with S-007, N-035..N-039, N-043, N-045, and M6-owned shutdown behavior covered; M7 remains next.

- [ ] **Step 1: Complete the security/lifecycle matrix**

Ensure tests explicitly prove:

```text
N-035 stream without registered OperationId -> reject before payload exposure
N-036 expired/cancelled/revoked/consumed operation -> reject
N-037 correct OperationId with wrong session/peer/capability/version/operation/direction -> reject
N-038 second stream using SingleStream -> reject
N-039 trust/policy revision change -> reject and operation becomes unusable
N-043 pending-open/chunk/active-runtime saturation -> typed backpressure/resource error
N-045 cancellation during setup/admission failure -> receiver cancelled, no admitted dangling stream
M6 shutdown -> active memory streams cancelled and runtime/admission state terminal
```

Add a bounded multi-stream scenario where indices arrive `1` then `0`, both succeed, a duplicate index fails, and a third index at/above the declared bound fails.

- [ ] **Step 2: Run all focused M6 tests**

Run:

```bash
cargo test -p crosslab-policy --test operations
cargo test -p crosslab-core --test stream_admission
cargo test -p crosslab-core --test transport_stream
cargo test -p crosslab-sim --test memory_stream_transport
cargo test -p crosslab-sim --test stream_scenarios
```

Expected: all PASS.

- [ ] **Step 3: Run the complete repository baseline**

Run:

```bash
cargo metadata --locked --format-version 1 > /dev/null
cargo fmt --check
cargo check --workspace --all-targets --all-features
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-targets --all-features
```

Expected: all PASS.

- [ ] **Step 4: Run the existing parser fuzz smoke**

The existing `data_stream_open` target remains the only M6 untrusted stream-open parser unless implementation added a new parser surface.

Run the repository's bounded fuzz workflow/commands, including at minimum:

```bash
cargo +nightly fuzz run data_stream_open -- -runs=256
```

Also run the existing control/identifier/pairing/session-auth targets through the normal `Fuzz Smoke` workflow. Expected: PASS.

Do not modify fuzz code merely to create churn. Modify `data_stream_open.rs` or the workflow only if a new parsing edge is actually introduced by implementation.

- [ ] **Step 5: Review the complete M6 diff against scope**

Confirm the branch contains only:

```text
policy stream-use budget
core stream admission + transport-neutral stream seam
bounded simulator stream transport/runtime
tests and justified fuzz changes
design/plan/CURRENT documentation
```

Reject any accidental Quinn/Iroh/libp2p, async-runtime, filesystem, M7 reconnect/revocation, UI, persistence, or platform changes.

- [ ] **Step 6: Update `CURRENT.md` with exact evidence and M7 handoff**

Record:

```text
M6 status and completed slices
valid RED commit/run IDs for each task where CI evidence exists
final exact code head
CI and Fuzz Smoke run IDs
scope review result
unfinished work, if any
exact next milestone: M7 disconnect/reconnect/revocation/failure simulator
```

- [ ] **Step 7: Verify the documentation-inclusive exact head**

Run/await normal CI and Fuzz Smoke on the exact PR head. Do not merge based on an earlier code-only head.

Expected: lockfile, rustfmt, workspace check, Clippy `-D warnings`, full tests, and bounded fuzz smoke all PASS.

- [ ] **Step 8: Merge only the verified exact head to `main` and post-merge verify**

Use preserved-history merge with expected head SHA. Then verify canonical `main` CI. If the fuzz workflow is PR-only and the merge commit tree is byte-identical to the fuzz-verified PR tree, record that exact-tree equivalence instead of inventing a push fuzz result.

- [ ] **Step 9: Write and verify the final M6 integration record**

Update canonical `main` `docs/development/CURRENT.md` with the merge commit, post-merge run, branch cleanup status, and M7 exact next task. Verify that documentation-only main head before starting M7.
