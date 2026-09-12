# M6 Authorized Data Streams Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Prove that bounded bulk payload bytes are exposed only after an active logical session admits a `DataStreamOpen` against a currently valid local `AuthorizedOperation`, including single-stream and explicitly bounded multi-stream use.

**Architecture:** `crosslab-policy` remains the authority owner and adds only monotonic stream-use budgeting. `crosslab-core` owns session-bound stream admission/replay state and transport-neutral data-stream interfaces. `crosslab-sim` owns deterministic bounded unidirectional stream queues and S-007 composition. No real networking, filesystem transfer, or general async runtime enters M6.

**Tech Stack:** Rust 2024, Rust 1.98.1, existing workspace crates/Prost protocol code, `Arc<Mutex<_>>`, `VecDeque`, `NonZeroU32`, and `NonZeroUsize`; no new third-party dependencies.

**Spec:** `docs/plans/phase-1/M6-authorized-data-streams-design.md`

**Architecture sources:** `docs/architecture/MASTER-ARCHITECTURE.md`, `docs/architecture/CORE-SIMULATOR.md`, `docs/architecture/SESSION-TRANSPORT.md`, `docs/protocol/PROTOCOL-V1.md`.

## Global Constraints

- Control authorizes; data carries payload. `StreamId`, transport identity, and `OperationId` alone never grant authority.
- Reuse M3 `AuthorizedOperation`, M4 `DataStreamOpen`, and M5 session/transport seams.
- `SingleAction` cannot authorize a stream; `SingleStream` permits one; `MultiStream` is explicitly bounded and never unlimited.
- Stream-use reservation is monotonic. Cancellation after admission never refunds a slot.
- Core validates active session, exact `SessionId`, negotiated capability/version, operation binding, direction, index, expiry, trust revision, policy revision, and remaining stream budget before payload exposure.
- Core owns stream-index/replay/admission state. Policy owns only operation authority and monotonic use count.
- Security-relevant replay state is never silently evicted; capacity exhaustion is a typed failure.
- Pending-open queues, live stream registries, chunk queues, and chunk bytes are all explicitly bounded.
- Backpressure preserves caller ownership of unsent opening frames/chunks; `Debug` must redact payload bytes.
- No Quinn/Iroh/libp2p, sockets, TLS, persistence, UI/platform work, real files, resumability, or M7 reconnect/revocation orchestration.
- No general async runtime and no busy polling.
- Uploaded Quinn/Iroh research is semantic reference only: graceful finish versus abortive cancellation and uni-stream open/accept lifecycle. No external types/code are copied.
- Every implementation slice follows RED -> minimal GREEN -> focused/full verification -> commit.

---

### Task 1: Bounded stream-use authority

**Files:**
- Modify: `crates/policy/src/operation/mod.rs`
- Test: `crates/policy/tests/operations.rs`

**Interfaces:**

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

`reserve_stream_use` runs the existing binding/expiry/revision validation first, rejects `SingleAction`, rejects an exhausted budget, then increments exactly once. It never decrements and does not itself consume the operation.

- [ ] **Step 1: Write failing tests**

Add tests proving: `SingleAction` -> `StreamUseNotAllowed`; `SingleStream` reserves exactly once; `MultiStream { max_streams: 2 }` reserves twice then returns `StreamBudgetExhausted`; expiry/cancel/revoke/trust-revision/policy-revision failures leave `reserved_stream_uses() == 0`.

Representative test:

```rust
#[test]
fn single_stream_budget_is_monotonic() {
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
```

- [ ] **Step 2: Verify RED**

```bash
cargo test -p crosslab-policy --test operations
```

Expected: compile failure for the missing policy/API surface.

- [ ] **Step 3: Implement minimal policy state**

Add `reserved_stream_uses: u32`, initialize to zero, and use:

```rust
const fn stream_budget(use_policy: UsePolicy) -> Option<u32> {
    match use_policy {
        UsePolicy::SingleAction => None,
        UsePolicy::SingleStream => Some(1),
        UsePolicy::MultiStream { max_streams } => Some(max_streams.get()),
    }
}
```

- [ ] **Step 4: Verify GREEN**

```bash
cargo test -p crosslab-policy --test operations
cargo fmt --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-targets --all-features
```

- [ ] **Step 5: Commit**

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

```rust
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
    StreamNotFound,
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

`RegisteredOperation` is private and owns one `AuthorizedOperation` plus its used indices. The operation remains registered after terminal use so used indices are not silently forgotten.

For inbound opens derive `OperationUseContext` exactly from authenticated session direction:

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

Before `reserve_stream_use`, validate active session/context, exact session ID, negotiated capability/version, registry capacity, duplicate active `StreamId`, index range, and duplicate `(OperationId, stream_index)`. `SingleStream` requires index `0`; bounded multi-stream accepts unique indices `< max_streams`, in any arrival order.

When finish/cancel removes the last active stream for an operation whose budget is exhausted, call `operation.consume()`. `cancel_all` cancels all active registered operations and clears active streams without making used slots reusable.

- [ ] **Step 1: Write the failing admission contract**

Create fixtures using a real active `LogicalSession` with negotiated `files.transfer` capability and real `AuthorizedOperation` state. Cover:

```text
valid SingleStream admit -> finish -> operation terminal
N-035 missing OperationId
N-036 expired/cancelled/revoked/consumed operation
N-037 wrong session/peer/capability/version/operation/direction
N-038 second SingleStream index
N-039 trust/policy revision change
duplicate active StreamId
duplicate multi-stream index
out-of-range multi-stream index
multi-stream arrival order 1 then 0
operation/admission capacity exhaustion
```

Representative assertion:

```rust
let admitted = admission
    .admit_inbound(&session, &open, 15, trust_revision, policy_revision)
    .unwrap();
assert_eq!(admitted.stream_id(), open.stream_id());
assert_eq!(admission.active_stream_count(), 1);
admission.finish_stream(open.stream_id()).unwrap();
```

- [ ] **Step 2: Verify RED**

```bash
cargo test -p crosslab-core --test stream_admission
```

Expected: compile failure because the M6 admission API does not exist.

- [ ] **Step 3: Implement minimal bounded admission state**

Use bounded `Vec` storage only; no new dependency. Mutate operation budget/used indices/active streams only after every prerequisite check succeeds.

- [ ] **Step 4: Verify GREEN**

```bash
cargo test -p crosslab-core --test stream_admission
cargo test -p crosslab-policy --test operations
cargo fmt --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-targets --all-features
```

- [ ] **Step 5: Commit**

```bash
git add crates/core/src/stream/mod.rs crates/core/src/lib.rs crates/core/tests/stream_admission.rs
git commit -m "feat(core): add authorized stream admission"
```

---

### Task 3: Transport-neutral stream seam + bounded memory streams

These changes are one atomic task because extending `TransportConnection` necessarily changes the only concrete `MemoryTransportEndpoint`; do not add fake default stream methods just to split commits.

**Files:**
- Modify: `crates/core/src/transport/mod.rs`
- Modify: `crates/core/src/lib.rs`
- Test: `crates/core/tests/transport_stream.rs`
- Modify: `apps/sim/src/transport.rs`
- Test: `apps/sim/tests/memory_stream_transport.rs`

**Transport-neutral interfaces:**

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

Extend `TransportConnection`:

```rust
fn try_open_uni_stream(
    &self,
    opening_frame: Vec<u8>,
) -> Result<Box<dyn TransportSendStream>, StreamOpenError>;

fn try_accept_uni_stream(&self) -> Result<IncomingUniStream, StreamAcceptError>;
```

Add consuming ownership helpers:

```rust
impl StreamOpenError {
    pub fn into_opening_frame(self) -> Vec<u8>;
}

impl StreamSendError {
    pub fn into_chunk(self) -> Vec<u8>;
}
```

Errors carrying bytes use custom redacted `Debug` like `ControlSendError`.

**Memory transport configuration:**

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

Keep existing `MemoryTransportPair::new(control_capacity, binding)` by delegating to simulator defaults of 8 live/pending streams, 8 queued chunks per stream, and 64 KiB maximum chunk size.

`stream_capacity` bounds both pending opens and live tracked stream states per direction. Terminal states are pruned before a new open; if capacity remains exhausted, opening fails without creating a stream. `finish()` disallows future sends but drains queued chunks before receiver `Finished`; `cancel()` clears queued chunks and yields `Cancelled`; connection close cancels every pending/live stream.

- [ ] **Step 1: Write RED tests**

`crates/core/tests/transport_stream.rs` proves byte ownership and debug redaction. `apps/sim/tests/memory_stream_transport.rs` proves:

```text
ordered open acceptance
pending-open saturation preserves opening bytes
ordered chunk delivery
chunk-queue saturation preserves unsent chunk
oversized chunk preserves unsent chunk
graceful finish drains then Finished
cancel clears then Cancelled
connection close cancels pending/live streams
failed open leaves no dangling live stream
```

Representative bounded-chunk test:

```rust
let mut send = a.try_open_uni_stream(vec![1, 2, 3]).unwrap();
let incoming = b.try_accept_uni_stream().unwrap();
let (_, mut receive) = incoming.into_parts();

send.try_send_chunk(vec![10]).unwrap();
send.try_send_chunk(vec![11]).unwrap();
assert_eq!(
    send.try_send_chunk(vec![12]),
    Err(StreamSendError::Full(vec![12]))
);
assert_eq!(receive.try_receive_chunk().unwrap(), vec![10]);
assert_eq!(receive.try_receive_chunk().unwrap(), vec![11]);
```

- [ ] **Step 2: Verify RED**

```bash
cargo test -p crosslab-core --test transport_stream
cargo test -p crosslab-sim --test memory_stream_transport
```

Expected: compile failures for missing M6 stream transport APIs.

- [ ] **Step 3: Implement the neutral traits/errors and memory stream state**

Use only `Arc<Mutex<_>>` and bounded `VecDeque`. Transport opening bytes remain opaque; no protocol decoding belongs in `apps/sim/src/transport.rs`.

- [ ] **Step 4: Verify GREEN and M5 regressions**

```bash
cargo test -p crosslab-core --test transport_stream
cargo test -p crosslab-sim --test memory_stream_transport
cargo test -p crosslab-sim --test memory_transport
cargo fmt --check
cargo check --workspace --all-targets --all-features
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-targets --all-features
```

- [ ] **Step 5: Commit**

```bash
git add crates/core/src/transport/mod.rs crates/core/src/lib.rs crates/core/tests/transport_stream.rs apps/sim/src/transport.rs apps/sim/tests/memory_stream_transport.rs
git commit -m "feat(transport): add bounded memory data streams"
```

---

### Task 4: Simulator stream runtime + S-007

**Files:**
- Create: `apps/sim/src/stream.rs`
- Modify: `apps/sim/src/lib.rs`
- Test: `apps/sim/tests/stream_scenarios.rs`

**Interfaces:**

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
    capacity: usize,
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

`new` requires an active session. `open_uni` only encodes the existing bounded `DataStreamOpen` and asks transport to open; remote admission remains authoritative.

`accept_one` first checks `inbound.len() < capacity`. If saturated, it returns `ResourceLimit` **without dequeuing** a pending transport stream and without spending an operation slot. Otherwise it accepts opaque bytes, decodes with `decode_data_stream_open`, calls core admission, and stores the receive handle only after admission succeeds. Wire/admission failure cancels the accepted receive handle before returning.

When `try_receive_chunk` sees `Finished`, call `StreamAdmission::finish_stream` and remove the handle. On `Cancelled`, call `cancel_stream` and remove it. `shutdown` calls admission cancellation and closes the transport, making memory streams terminal.

- [ ] **Step 1: Write S-007 as RED**

Use existing M5 session/capability fixtures. The positive test must compose:

```text
policy Allow -> AuthorizationGrant
AuthorizedOperation(SingleStream)
receiver registers operation
sender opens encoded DataStreamOpen
receiver accepts/decodes/admits
three bounded synthetic chunks flow in order
sender finishes
receiver drains and sees terminal finish
same SingleStream operation cannot admit a second stream
```

No real file I/O.

Also add a test where runtime inbound capacity is full and confirm a second pending open remains unaccepted and its operation budget remains unspent until capacity is available.

- [ ] **Step 2: Verify RED**

```bash
cargo test -p crosslab-sim --test stream_scenarios
```

Expected: compile failure for missing `crosslab_sim::stream`.

- [ ] **Step 3: Implement minimal simulator composition**

Keep all policy/admission logic in their owning crates and all queue logic in transport; `apps/sim/src/stream.rs` is glue only.

- [ ] **Step 4: Verify GREEN and M5 regressions**

```bash
cargo test -p crosslab-sim --test stream_scenarios
cargo test -p crosslab-sim --test session_scenarios
cargo test -p crosslab-sim --test m5_scenarios
cargo fmt --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-targets --all-features
```

- [ ] **Step 5: Commit**

```bash
git add apps/sim/src/stream.rs apps/sim/src/lib.rs apps/sim/tests/stream_scenarios.rs
git commit -m "feat(sim): compose authorized data streams"
```

---

### Task 5: M6 negatives, cancellation, fuzz, and closeout

**Files:**
- Modify: `crates/core/tests/stream_admission.rs`
- Modify: `apps/sim/tests/memory_stream_transport.rs`
- Modify: `apps/sim/tests/stream_scenarios.rs`
- Modify only if evidence requires: `fuzz/fuzz_targets/data_stream_open.rs`, `.github/workflows/fuzz.yml`
- Modify: `docs/development/CURRENT.md`

**Produces:** completed S-007 plus applicable N-035..N-039, N-043, N-045 and M6-owned shutdown behavior, exact verification evidence, and M7 handoff.

- [ ] **Step 1: Complete the security/lifecycle matrix**

Explicitly prove:

```text
N-035 no registered OperationId -> reject before payload exposure
N-036 expired/cancelled/revoked/consumed -> reject
N-037 wrong session/peer/capability/version/operation/direction -> reject
N-038 second SingleStream -> reject
N-039 trust/policy revision change -> reject
N-043 pending-open/chunk/runtime capacity saturation -> backpressure/resource error
N-045 cancellation during setup/admission failure -> receive side cancelled, no dangling admission
shutdown -> active memory streams cancelled and admission/runtime state terminal
MultiStream(2) -> indices 1 then 0 succeed; duplicate index and index 2 fail
```

- [ ] **Step 2: Run all focused M6 tests**

```bash
cargo test -p crosslab-policy --test operations
cargo test -p crosslab-core --test stream_admission
cargo test -p crosslab-core --test transport_stream
cargo test -p crosslab-sim --test memory_stream_transport
cargo test -p crosslab-sim --test stream_scenarios
```

- [ ] **Step 3: Run the complete baseline**

```bash
cargo metadata --locked --format-version 1 > /dev/null
cargo fmt --check
cargo check --workspace --all-targets --all-features
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-targets --all-features
```

- [ ] **Step 4: Run parser fuzz smoke**

The existing `data_stream_open` target remains the M6 untrusted parser surface unless implementation adds a genuinely new parser.

```bash
cargo +nightly fuzz run data_stream_open -- -runs=256
```

Run the repository `Fuzz Smoke` workflow so existing `control_frame`, `identifiers`, `pairing_bootstrap`, and `session_auth` targets remain green too. Do not modify fuzz files without a concrete new parsing edge.

- [ ] **Step 5: Review branch scope**

The implementation diff may contain only policy stream budgets, core stream admission/transport seam, bounded simulator stream transport/runtime, M6 tests, justified fuzz changes, and M6 docs. Reject real networking, async runtime, real files, M7 lifecycle, UI, persistence, or platform scope creep.

- [ ] **Step 6: Update `CURRENT.md` with exact evidence**

Record valid RED heads/run IDs, final code head, CI/Fuzz Smoke IDs, completed scope, unfinished work if any, and exact next milestone **M7 — disconnect/reconnect/revocation/failure simulator**.

- [ ] **Step 7: Verify the documentation-inclusive exact PR head**

Require lockfile, rustfmt, workspace check, Clippy `-D warnings`, all tests, and bounded fuzz smoke on the exact head that will merge.

- [ ] **Step 8: Merge only the verified exact head to `main` and verify canonical `main`**

Use preserved-history merge with expected head SHA. Verify post-merge `main` CI. If fuzz is PR-only and the merge tree is byte-identical to the fuzz-verified PR tree, record that exact-tree equivalence rather than inventing a push fuzz result.

- [ ] **Step 9: Write and verify the final M6 integration record**

Update canonical `main` `docs/development/CURRENT.md` with merge commit, post-merge run, branch cleanup status, and M7 exact next task. Verify that documentation-only main head before starting M7.
