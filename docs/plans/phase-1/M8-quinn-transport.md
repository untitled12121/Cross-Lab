# M8 Quinn Transport Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add the first real encrypted Cross-Lab IP transport over Quinn/QUIC while proving that the existing M5–M7 logical-session, control, operation, stream, reconnect, revocation, and shutdown semantics remain transport-neutral.

**Architecture:** `crosslab-core` keeps the narrow synchronous nonblocking `TransportConnection` semantic seam. A new `crosslab-transport-quic` adapter owns Quinn, Tokio, TLS-exporter channel binding, private QUIC record framing, bounded async bridges, connection/stream tasks, and network fault mapping. Cross-Lab authentication continues to use the existing protocol/core types; Quinn/TLS protects the channel but does not become Cross-Lab identity.

**Tech Stack:** Rust 2024 / Rust 1.98.1, Quinn `0.11.11`, Tokio `1.53.1`, Quinn rustls-ring backend, rustls `0.23.44` for loopback test trust construction, rcgen `0.14.10` for ephemeral loopback test certificates.

**Spec:** `docs/plans/phase-1/M8-quinn-transport-design.md`

## Global Constraints

- Preserve `docs/architecture/SESSION-TRANSPORT.md`, M5 session/control semantics, M6 authorized-stream semantics, and M7 failure/revocation lifecycle.
- `quinn`, `tokio`, `rustls`, socket/runtime, and TLS certificate types must not enter `crosslab-core`, `crosslab-protocol`, `crosslab-policy`, or `crosslab-identity` public/domain state.
- Channel binding must implement accepted `ADR-0008-quinn-channel-binding-profile-v1.md` exactly: profile `quic-tls-exporter-v1`, 32-byte exporter output, label `EXPORTER-Cross-Lab-QUIC-Channel-Binding-v1`, context `crosslab.quic.transport.v1`.
- Do not call Quinn `Connecting::into_0rtt`; M8 authorizes no 0-RTT application/session data.
- One full QUIC/TLS connection is one fresh M8 session-bootstrap attempt. Reconnect creates a new connection, binding, nonces/proofs, `SessionId`, sequences, capabilities, and authorization state.
- Use bounded Tokio channels, explicit QUIC stream/congestion windows, explicit maximum record sizes, and no `read_to_end` with untrusted lengths.
- No discovery, NAT traversal, relay, Iroh, libp2p, datagram consumer, route migration, persistence, UI, platform adapter, privileged service, or production certificate-provisioning design enters M8.
- The uploaded Quinn tree is research `main` at package version `0.12.0`; production code pins the latest published stable Quinn `0.11.11` until a separately verified stable upgrade is approved.
- Every production behavior change follows RED → verified failure → minimal GREEN → verification → commit.

---

## File Structure

```text
transports/quic/
├── Cargo.toml
└── src/
    ├── lib.rs          # narrow exports only
    ├── binding.rs      # ADR-0008 exporter derivation
    ├── config.rs       # typed resource/window/queue limits
    ├── connection.rs   # control bridge + terminal connection state
    ├── record.rs       # private u32-be record framing
    ├── stream.rs       # uni-stream send/receive bridge drivers
    └── tests.rs        # crate-internal real-loopback M8 integration scenarios
```

M8 deliberately keeps endpoint/TLS fixture construction inside crate-internal tests. It does not freeze a production certificate-provisioning or public endpoint-manager API before M10 has an agent/platform consumer.

### Task 1: Tighten transport-neutral outbound size semantics

**Files:**
- Modify: `crates/core/src/transport/mod.rs`
- Modify: `apps/sim/src/transport.rs`
- Test: `apps/sim/tests/memory_transport.rs`
- Test: `apps/sim/tests/memory_stream_transport.rs`

**Interfaces:**
- Consumes: existing `TransportConnection`, `MemoryTransportConfig`, `ControlSendError`, `StreamOpenError`.
- Produces: ownership-preserving `ControlSendError::TooLarge(Vec<u8>)`, `StreamOpenError::TooLarge(Vec<u8>)`, and configurable in-memory control/opening-frame transport limits without changing the `TransportConnection` method signatures.

- [ ] **Step 1: Write failing memory-transport size tests**

Add focused cases proving the original buffer is returned and no queue/stream slot is consumed:

```rust
#[test]
fn oversized_control_frame_is_rejected_without_consuming_capacity() {
    let config = MemoryTransportConfig::default_for_tests()
        .with_frame_limits(nz(4), nz(8));
    let pair = MemoryTransportPair::with_config(config, [0x71; 32]);
    let (a, b) = pair.endpoints();

    let frame = vec![0x11; 5];
    assert_eq!(
        a.try_send_control(frame.clone()),
        Err(ControlSendError::TooLarge(frame))
    );
    assert_eq!(b.try_receive_control(), Err(ControlReceiveError::Empty));
}

#[test]
fn oversized_opening_frame_is_rejected_without_allocating_stream_state() {
    let config = MemoryTransportConfig::default_for_tests()
        .with_frame_limits(nz(8), nz(4));
    let pair = MemoryTransportPair::with_config(config, [0x72; 32]);
    let (a, b) = pair.endpoints();

    let opening = vec![0x22; 5];
    let error = a.try_open_uni_stream(opening.clone()).unwrap_err();
    assert_eq!(error, StreamOpenError::TooLarge(opening));
    assert_eq!(b.try_accept_uni_stream(), Err(StreamAcceptError::Empty));
}
```

Add the local helper in the test module only:

```rust
fn nz(value: usize) -> NonZeroUsize {
    NonZeroUsize::new(value).unwrap()
}
```

- [ ] **Step 2: Run focused tests and verify RED**

Run:

```bash
cargo test -p crosslab-sim --test memory_transport oversized_control_frame_is_rejected_without_consuming_capacity
cargo test -p crosslab-sim --test memory_stream_transport oversized_opening_frame_is_rejected_without_allocating_stream_state
```

Expected: compilation/test failure because `TooLarge`, `default_for_tests`, and `with_frame_limits` do not yet exist.

- [ ] **Step 3: Add minimal core error variants and memory limits**

Extend the existing enums without changing existing variants:

```rust
pub enum ControlSendError {
    Full(Vec<u8>),
    TooLarge(Vec<u8>),
    Closed(Vec<u8>),
}

pub enum StreamOpenError {
    Full(Vec<u8>),
    TooLarge(Vec<u8>),
    Closed(Vec<u8>),
}
```

Update redacted `Debug`, `Display`, and ownership helpers consistently. Add two fields to `MemoryTransportConfig` and preserve existing constructor call sites by defaulting them to nonzero constants:

```rust
const DEFAULT_MAX_CONTROL_FRAME_BYTES: usize = 256 * 1024 + 4;
const DEFAULT_MAX_OPENING_FRAME_BYTES: usize = 4 * 1024 + 4;

impl MemoryTransportConfig {
    pub fn default_for_tests() -> Self {
        Self::new(
            nz_const(8),
            nz_const(DEFAULT_STREAM_CAPACITY),
            nz_const(DEFAULT_CHUNK_CAPACITY),
            nz_const(DEFAULT_MAX_CHUNK_BYTES),
        )
    }

    pub const fn with_frame_limits(
        mut self,
        max_control_frame_bytes: NonZeroUsize,
        max_opening_frame_bytes: NonZeroUsize,
    ) -> Self {
        self.max_control_frame_bytes = max_control_frame_bytes;
        self.max_opening_frame_bytes = max_opening_frame_bytes;
        self
    }
}
```

Before enqueue/allocation, return `TooLarge` when the submitted `Vec<u8>` exceeds the configured limit.

- [ ] **Step 4: Run focused + existing simulator tests**

Run:

```bash
cargo test -p crosslab-sim --test memory_transport
cargo test -p crosslab-sim --test memory_stream_transport
cargo test -p crosslab-sim
```

Expected: PASS with no changed `Full`/`Closed` behavior.

- [ ] **Step 5: Commit the green transport-contract tightening**

```bash
git add crates/core/src/transport/mod.rs apps/sim/src/transport.rs \
  apps/sim/tests/memory_transport.rs apps/sim/tests/memory_stream_transport.rs
git commit -m "feat(transport): bound outbound transport frames"
```

### Task 2: Add the isolated Quinn crate, dependency pins, config, and exporter binding

**Files:**
- Modify: `Cargo.toml`
- Modify: `Cargo.lock`
- Create: `transports/quic/Cargo.toml`
- Create: `transports/quic/src/lib.rs`
- Create: `transports/quic/src/config.rs`
- Create: `transports/quic/src/binding.rs`
- Create: `transports/quic/src/tests.rs`

**Interfaces:**
- Consumes: `crosslab_core::{ChannelBinding, ConnectionMetadata, TransportSecurityClass}` and ADR-0008.
- Produces: `QuicTransportConfig`, private `derive_channel_binding(&quinn::Connection) -> Result<ChannelBinding, QuicTransportError>`, and real loopback evidence that both established Quinn peers derive identical 32-byte bindings while a fresh reconnect derives a different binding.

- [ ] **Step 1: Add a RED crate test that requires an exporter-derived binding**

Create a crate-internal async test shape:

```rust
#[tokio::test]
async fn exporter_binding_matches_peer_and_changes_on_reconnect() {
    let first = loopback_connection_pair().await.unwrap();
    let first_client = derive_channel_binding(&first.client).unwrap();
    let first_server = derive_channel_binding(&first.server).unwrap();
    assert_eq!(first_client, first_server);
    assert_eq!(first_client.profile_id(), "quic-tls-exporter-v1");
    assert_eq!(first_client.bytes().len(), 32);

    let second = loopback_connection_pair().await.unwrap();
    let second_client = derive_channel_binding(&second.client).unwrap();
    assert_ne!(first_client.bytes(), second_client.bytes());
}
```

The `loopback_connection_pair` helper is test-only and generates an rcgen `localhost` certificate, creates a Quinn server endpoint bound to `127.0.0.1:0`, explicitly adds that certificate to the client root store, awaits the full client/server handshakes, and never calls `into_0rtt`.

- [ ] **Step 2: Run and verify RED**

Run:

```bash
cargo test -p crosslab-transport-quic exporter_binding_matches_peer_and_changes_on_reconnect
```

Expected: package/API missing.

- [ ] **Step 3: Add exact dependency pins and minimal config/binding implementation**

Add workspace member/dependencies:

```toml
members = [
    "apps/sim",
    "crates/core",
    "crates/crypto",
    "crates/identity",
    "crates/policy",
    "crates/protocol",
    "transports/quic",
]

[workspace.dependencies]
quinn = { version = "=0.11.11", default-features = false, features = ["runtime-tokio", "rustls-ring"] }
tokio = { version = "=1.53.1", default-features = false }
rustls = { version = "=0.23.44", default-features = false, features = ["ring", "std"] }
rcgen = { version = "=0.14.10", default-features = false, features = ["ring"] }
```

The adapter crate uses Tokio `rt`, `sync`, `time`; tests additionally use `macros` and `rt-multi-thread`. Implement the exact ADR profile:

```rust
const CHANNEL_BINDING_PROFILE: &str = "quic-tls-exporter-v1";
const EXPORTER_LABEL: &[u8] = b"EXPORTER-Cross-Lab-QUIC-Channel-Binding-v1";
const EXPORTER_CONTEXT: &[u8] = b"crosslab.quic.transport.v1";

fn derive_channel_binding(connection: &quinn::Connection) -> Result<ChannelBinding, QuicTransportError> {
    let mut bytes = [0_u8; 32];
    connection
        .export_keying_material(&mut bytes, EXPORTER_LABEL, EXPORTER_CONTEXT)
        .map_err(|_| QuicTransportError::ChannelBinding)?;
    Ok(ChannelBinding::new(CHANNEL_BINDING_PROFILE, bytes.to_vec()))
}
```

- [ ] **Step 4: Verify binding and dependency gates**

Run:

```bash
cargo metadata --locked --no-deps --format-version 1 > /dev/null
cargo test -p crosslab-transport-quic exporter_binding_matches_peer_and_changes_on_reconnect
cargo tree -p crosslab-transport-quic -d
```

Expected: binding test PASS; no accidental Iroh/libp2p dependency and no second async runtime.

- [ ] **Step 5: Commit**

```bash
git add Cargo.toml Cargo.lock transports/quic
git commit -m "feat(quic): establish exporter-bound Quinn transport"
```

### Task 3: Implement bounded private record framing

**Files:**
- Create: `transports/quic/src/record.rs`
- Modify: `transports/quic/src/lib.rs`
- Test: `transports/quic/src/tests.rs`

**Interfaces:**
- Produces private async helpers:

```rust
async fn write_record(send: &mut quinn::SendStream, bytes: &[u8], max: usize) -> Result<(), RecordError>;
async fn read_record(recv: &mut quinn::RecvStream, max: usize, allow_empty: bool) -> Result<Vec<u8>, RecordError>;
```

- [ ] **Step 1: Write RED tests for exact framing and hostile lengths**

Use a real loopback uni/bi stream and assert:

```rust
#[tokio::test]
async fn record_reader_rejects_declared_length_before_allocating_body() {
    let pair = loopback_connection_pair().await.unwrap();
    let (mut send, mut recv) = open_test_bi(&pair).await;
    send.write_all(&(9_u32).to_be_bytes()).await.unwrap();
    send.write_all(&[0_u8; 1]).await.unwrap();
    send.finish().unwrap();

    assert_eq!(
        read_record(&mut recv, 8, false).await,
        Err(RecordError::TooLarge { declared: 9, max: 8 })
    );
}
```

Also cover zero-length control/opening rejection, truncated 4-byte prefix, truncated body, exact-limit success, peer reset, and connection close.

- [ ] **Step 2: Verify RED**

```bash
cargo test -p crosslab-transport-quic record_
```

Expected: record helpers/errors missing.

- [ ] **Step 3: Implement only u32-be length framing**

Implementation shape:

```rust
async fn write_record(...) -> Result<(), RecordError> {
    if bytes.is_empty() || bytes.len() > max || bytes.len() > u32::MAX as usize {
        return Err(...);
    }
    send.write_all(&(bytes.len() as u32).to_be_bytes()).await?;
    send.write_all(bytes).await?;
    Ok(())
}

async fn read_record(...) -> Result<Vec<u8>, RecordError> {
    let mut prefix = [0_u8; 4];
    recv.read_exact(&mut prefix).await?;
    let declared = u32::from_be_bytes(prefix) as usize;
    if declared > max { return Err(RecordError::TooLarge { declared, max }); }
    if declared == 0 && !allow_empty { return Err(RecordError::Empty); }
    let mut bytes = vec![0_u8; declared];
    recv.read_exact(&mut bytes).await?;
    Ok(bytes)
}
```

Do not use `read_to_end` for attacker-controlled record lengths.

- [ ] **Step 4: Verify focused tests**

```bash
cargo test -p crosslab-transport-quic record_
```

Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add transports/quic/src/record.rs transports/quic/src/lib.rs transports/quic/src/tests.rs
git commit -m "feat(quic): add bounded stream record framing"
```

### Task 4: Bridge the reserved control stream into `TransportConnection`

**Files:**
- Create: `transports/quic/src/connection.rs`
- Modify: `transports/quic/src/lib.rs`
- Modify: `transports/quic/src/tests.rs`

**Interfaces:**
- Produces `QuicTransportConnection` implementing `TransportConnection` for security class `AuthenticatedConfidentialChannel`.
- Private constructor consumes an established Quinn `Connection`, the reserved control `SendStream`/`RecvStream`, derived `ChannelBinding`, `ConnectionMetadata`, and `QuicTransportConfig`.
- Produces adapter-specific `async fn shutdown(&self)` used by tests to join owned tasks; Quinn types remain inside the crate.

- [ ] **Step 1: Write RED control behavior tests**

Prove ordered delivery, local queue saturation, oversized ownership preservation, remote close, and explicit shutdown:

```rust
#[tokio::test]
async fn control_bridge_preserves_order_and_bounded_backpressure() {
    let (client, server) = promoted_loopback_transport_pair(1).await.unwrap();

    client.try_send_control(vec![1]).unwrap();
    assert_eq!(
        client.try_send_control(vec![2]),
        Err(ControlSendError::Full(vec![2]))
    );

    eventually(|| server.try_receive_control() == Ok(vec![1])).await;
    client.try_send_control(vec![2]).unwrap();
    eventually(|| server.try_receive_control() == Ok(vec![2])).await;
}
```

- [ ] **Step 2: Verify RED**

```bash
cargo test -p crosslab-transport-quic control_bridge_
```

Expected: `QuicTransportConnection` missing.

- [ ] **Step 3: Implement bounded control channels and terminal state**

Use bounded `tokio::sync::mpsc` channels and `try_send`/`try_recv` at the trait boundary. One writer task serializes records; one reader task reads records and awaits inbound queue capacity. A shared terminal flag transitions once. Store task `JoinHandle`s in a connection-owned registry and make `shutdown()` close the Quinn connection, stop new work, drain task handles, and await all owned tasks.

Map local semantic outcomes exactly:

```rust
match outbound.try_send(frame) {
    Ok(()) => Ok(()),
    Err(TrySendError::Full(frame)) => Err(ControlSendError::Full(frame)),
    Err(TrySendError::Closed(frame)) => Err(ControlSendError::Closed(frame)),
}
```

Check configured maximum before `try_send` and return `TooLarge(frame)` first.

- [ ] **Step 4: Verify control bridge + shutdown tests**

```bash
cargo test -p crosslab-transport-quic control_bridge_
cargo test -p crosslab-transport-quic shutdown_
```

Expected: PASS without sleeps used as synchronization primitives; polling helpers must have explicit timeout bounds.

- [ ] **Step 5: Commit**

```bash
git add transports/quic/src/connection.rs transports/quic/src/lib.rs transports/quic/src/tests.rs
git commit -m "feat(quic): bridge bounded control traffic"
```

### Task 5: Bridge authorized unidirectional data streams

**Files:**
- Create: `transports/quic/src/stream.rs`
- Modify: `transports/quic/src/connection.rs`
- Modify: `transports/quic/src/lib.rs`
- Modify: `transports/quic/src/tests.rs`

**Interfaces:**
- Implements existing `TransportSendStream` and `TransportReceiveStream` without changing core signatures.
- `try_open_uni_stream(opening_frame)` reserves a bounded outgoing-stream slot synchronously, returns a bounded send handle, and drives Quinn `open_uni()` asynchronously.
- Inbound accept task reads the bounded opening record before publishing `IncomingUniStream`.

- [ ] **Step 1: Write RED stream tests**

Cover opening-frame oversize, concurrent-stream saturation, chunk oversize, chunk queue saturation, ordered chunks, FIN → `Finished`, sender cancel/drop → reset/cancelled, receiver cancel/drop → stop, and connection close cancelling live streams.

Representative test:

```rust
#[tokio::test]
async fn uni_stream_carries_opening_frame_and_chunks_in_order() {
    let (client, server) = promoted_loopback_transport_pair(8).await.unwrap();
    let mut send = client.try_open_uni_stream(vec![0x10]).unwrap();
    send.try_send_chunk(vec![0x20]).unwrap();
    send.try_send_chunk(vec![0x21]).unwrap();
    send.finish();

    let incoming = eventually_accept(&server).await;
    assert_eq!(incoming.opening_frame(), &[0x10]);
    let (_, mut recv) = incoming.into_parts();
    assert_eq!(eventually_receive(&mut recv).await, vec![0x20]);
    assert_eq!(eventually_receive(&mut recv).await, vec![0x21]);
    eventually_finished(&mut recv).await;
}
```

- [ ] **Step 2: Verify RED**

```bash
cargo test -p crosslab-transport-quic uni_stream_
```

Expected: stream implementation missing.

- [ ] **Step 3: Implement bounded stream drivers**

Use a nonblocking semaphore/slot counter for outgoing stream capacity. Each send handle owns a bounded `mpsc::Sender<Vec<u8>>`; `finish()` drops the sender with `cancelled == false`, while `cancel()` marks cancelled then drops it. The driver awaits `connection.open_uni()`, writes the opening record, drains chunks, then calls Quinn FIN or reset based on terminal state.

The inbound accept task awaits `connection.accept_uni()`, reads/validates the opening frame, publishes an `IncomingUniStream` into a bounded queue, and spawns a bounded reader driver. Receiver cancellation signals the driver to call `RecvStream::stop()`.

Do not buffer more than configured queue capacities and QUIC windows.

- [ ] **Step 4: Verify stream tests**

```bash
cargo test -p crosslab-transport-quic uni_stream_
cargo test -p crosslab-transport-quic stream_
```

Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add transports/quic/src/stream.rs transports/quic/src/connection.rs \
  transports/quic/src/lib.rs transports/quic/src/tests.rs
git commit -m "feat(quic): bridge bounded data streams"
```

### Task 6: Prove existing Cross-Lab session authentication over the Quinn binding

**Files:**
- Modify: `transports/quic/Cargo.toml` dev dependencies only if needed
- Modify: `transports/quic/src/tests.rs`

**Interfaces:**
- Consumes existing `SessionAuthHello`, `SessionAuthProofMessage`, `encode_session_auth_bootstrap`, `decode_session_auth_bootstrap`, `SessionAuthTranscriptV1`, `SessionActivation`, `LogicalSession`, trust/identity fixtures.
- Produces no new public domain protocol. Test-only orchestration exchanges existing bounded bootstrap records on the reserved QUIC control stream, then promotes the same stream to `QuicTransportConnection` after both logical sessions become `Active`.

- [ ] **Step 1: Write RED end-to-end auth tests**

Add scenarios for trusted peers, wrong connection binding, replayed proof on reconnect, and control-before-auth rejection in the bootstrap harness.

Core assertion:

```rust
assert_eq!(client_session.state(), SessionState::Active);
assert_eq!(server_session.state(), SessionState::Active);
assert_eq!(
    client_session.context().unwrap().transport_security_class(),
    TransportSecurityClass::AuthenticatedConfidentialChannel
);
assert_ne!(
    first_client_session.context().unwrap().session_id(),
    reconnect_client_session.context().unwrap().session_id()
);
```

- [ ] **Step 2: Verify RED**

```bash
cargo test -p crosslab-transport-quic session_auth_
```

Expected: test bootstrap orchestration not yet implemented.

- [ ] **Step 3: Implement test-only bootstrap orchestration using existing production domain APIs**

The helper must:

```text
establish full QUIC/TLS
derive exporter binding
open/accept one bi control stream
exchange existing encoded SessionAuthHello frames
validate local trust/credentials and negotiate protocol/features
build the same SessionAuthTranscriptV1 on both ends
exchange role-separated existing proof frames
activate both LogicalSession values with AuthenticatedConfidentialChannel
promote the same bi stream into the ordinary bounded transport bridge
```

Do not add a second auth transcript, Quinn certificate → `DeviceId` mapping, or special bypass path.

- [ ] **Step 4: Verify authentication tests**

```bash
cargo test -p crosslab-transport-quic session_auth_
```

Expected: PASS for valid peers; wrong binding/replay fail before `Active`.

- [ ] **Step 5: Commit**

```bash
git add transports/quic/Cargo.toml transports/quic/src/tests.rs
git commit -m "test(quic): authenticate Cross-Lab sessions over QUIC"
```

### Task 7: Prove control, authorized streams, reconnect, revocation, and network failure over Quinn

**Files:**
- Modify: `transports/quic/src/tests.rs`
- Modify: `.github/workflows/ci.yml` only if a deterministic loopback test timeout/environment setting is required; do not weaken existing gates.

**Interfaces:**
- Consumes `SimNode`, `SimStreamRuntime`, existing policy/operation fixtures, and the authenticated Quinn pair from Task 6.
- Produces M8 behavioral evidence for the same domain semantics that already pass over `MemoryTransportPair`.

- [ ] **Step 1: Add RED M8 scenario tests**

Cover at minimum:

```text
encrypted loopback trusted session
capability advertisement + authorized control request/response/event
OperationId-bound data stream and payload
stream open without valid operation rejected by existing admission
transport disconnect closes session authority
fresh reconnect gets new binding + SessionId and cannot replay old control/operation state
active signed peer revocation cancels control/stream authority and closes Quinn transport
reconnect after revocation is denied
bounded queue/stream saturation remains ownership-preserving
cancellation during stream setup terminates cleanly
shutdown with active tasks joins cleanly
```

Use deterministic `tokio::time::timeout` bounds around every eventually/awaited network assertion.

- [ ] **Step 2: Verify RED for the first network-lifecycle scenario**

```bash
cargo test -p crosslab-transport-quic m8_reconnect_reauthenticates_with_fresh_authority -- --nocapture
```

Expected: fail until any missing adapter lifecycle mapping is implemented.

- [ ] **Step 3: Make only the minimal adapter fixes required by each failing scenario**

Do not move reconnect/revocation into Quinn. The adapter only marks terminal transport state and closes/cancels stream drivers. Existing M7 runtime code must continue to perform `LogicalSession::transport_lost()`, dispatcher cleanup, `StreamAdmission::cancel_all()`, and peer-revocation reaction.

- [ ] **Step 4: Run complete M8 + simulator regression suite**

```bash
cargo test -p crosslab-transport-quic
cargo test -p crosslab-sim
cargo test -p crosslab-core
```

Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add transports/quic/src/tests.rs transports/quic/src/*.rs .github/workflows/ci.yml
git commit -m "test(quic): prove M8 session and failure lifecycle"
```

### Task 8: Full verification, architecture reconciliation, and durable M9 handoff

**Files:**
- Modify: `docs/plans/phase-1/M8-quinn-transport-design.md` status/research notes if not already reconciled
- Modify: `docs/development/CURRENT.md`
- Modify: `docs/adr/README.md` if it maintains an ADR index

**Interfaces:**
- Produces a verified M8 branch head and exact next task for M9 remote-networking evaluation. No M9 dependency or prototype is introduced in this task.

- [ ] **Step 1: Run mandatory repository gates**

```bash
cargo metadata --locked --no-deps --format-version 1 > /dev/null
cargo fmt --check
cargo check --workspace --all-targets --all-features
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-features
```

Run any fuzz smoke required by `.github/workflows/fuzz.yml` if M8 changes a watched protocol/policy/fuzz path. Do not claim fuzz passed if the workflow is not applicable.

- [ ] **Step 2: Review dependency and architecture boundaries**

Run:

```bash
cargo tree -p crosslab-core
cargo tree -p crosslab-protocol
cargo tree -p crosslab-transport-quic
```

Confirm Quinn/Tokio/rustls do not appear under identity/policy/protocol/core and no Iroh/libp2p dependency entered M8.

- [ ] **Step 3: Reconcile documentation**

Record exact verified commit/CI IDs, implemented bounds, Quinn/Tokio/rustls/rcgen versions, ADR-0008 status, tests passed, limitations, and exact M9 next task in `docs/development/CURRENT.md`. Update the M8 design status to implemented/verified only after the gates actually pass.

- [ ] **Step 4: Commit final M8 checkpoint**

```bash
git add docs/plans/phase-1/M8-quinn-transport-design.md \
  docs/development/CURRENT.md docs/adr/README.md
git commit -m "docs: record M8 final verification checkpoint"
```

- [ ] **Step 5: Integrate only the exact verified head**

Open/refresh the M8 PR, require CI success on the exact head, merge that exact head into `main`, verify post-merge canonical `main` CI, then make the documentation-only integration checkpoint on `main` and verify it before deleting the feature branch.
