# M9 Remote Networking ADR Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Produce reproducible evidence for Cross-Lab remote connectivity, validate Iroh against the verified Quinn baseline without weakening identity/session/policy boundaries, and close M9 with an evidence-backed remote-networking ADR.

**Architecture:** Keep the verified M8 Quinn adapter unchanged as the local/LAN production baseline. Add one non-publishable `experiments/m9-networking` workspace crate that owns candidate dependencies, measurements, local relay fixtures, and controlled-network tooling. The Iroh candidate implements the existing `TransportConnection` semantics only inside the experiment; no candidate dependency graduates into production until ADR-0009 is accepted.

**Tech Stack:** Rust 2024 / Rust 1.98.x, Quinn `0.11.11`, Tokio `1.53.1`, Iroh `1.2.0`, Iroh Relay `1.2.0` for experiment-only self-hosted relay fixtures, rustls `0.23.44`, rcgen `0.14.10`; rust-libp2p `0.57.0` only if the explicit conditional comparison gate is triggered.

**Spec:** `docs/plans/phase-1/M9-remote-networking-design.md`

## Global Constraints

- Preserve `docs/architecture/SESSION-TRANSPORT.md`, ADR-0008, and all verified M8 session/control/stream/reconnect/revocation semantics.
- Required M9 tests must use no public infrastructure. Use `Endpoint::builder(presets::Minimal)` and explicit addresses/relay maps; never silently use the Iroh `N0` preset or default n0 relay map.
- Iroh `EndpointId`, Iroh `SecretKey`, relay URLs/tokens, path IDs, libp2p `PeerId`, `Multiaddr`, and `Swarm` state remain transport/routing metadata only. They never become Cross-Lab identity, trust, policy, capability, or operation authority.
- Reuse ADR-0008 exactly for the Iroh candidate after a full handshake: profile `quic-tls-exporter-v1`, 32 bytes, label `EXPORTER-Cross-Lab-QUIC-Channel-Binding-v1`, context `crosslab.quic.transport.v1`.
- Do not use Iroh or libp2p 0-RTT/early-data paths for Cross-Lab authority.
- Every Iroh-backed Cross-Lab session is `NetworkClass::Remote` for its entire lifetime, including relay-to-direct and direct-to-relay path changes.
- Path changes inside one live Iroh protected connection are observability/performance events only. They do not change `SessionId`, channel binding, sequence state, policy classification, or operation grants.
- A closed Iroh connection followed by a new connection is a Cross-Lab reconnect: fresh exporter binding, nonces/proofs, `SessionId`, capability state, sequence state, and authorization are mandatory.
- Begin candidate resource limits from the M8 semantic defaults: control queue 8, incoming stream queue 8, outgoing stream slots 8, chunk queue 8, max control `256 KiB + 4`, max opening `4 KiB + 4`, max chunk `64 KiB`, remote uni streams 32, remote bi streams 1, stream receive window `512 KiB`, connection receive window `4 MiB`, idle timeout 30 seconds.
- No unbounded Tokio channels, `read_to_end` with attacker-controlled lengths, detached tasks, infinite reconnect loops, or unbounded relay retry loops.
- Candidate crates must remain isolated under `experiments/m9-networking`; do not add Iroh/libp2p dependencies to `crosslab-core`, protocol, identity, policy, simulator, or `transports/quic`.
- Do not extract a generic transport framework crate during M9 unless measured prototype duplication proves a concrete reusable boundary and the ADR explicitly approves it.
- Iroh production selection happens only through ADR-0009 after required deterministic tests and the controlled networking evidence are recorded.
- A libp2p prototype is conditional. Do not add `libp2p` until Task 9's trigger rule is satisfied and recorded in the evidence document.
- All behavior changes follow RED -> verified failure -> minimal GREEN -> focused verification -> full gate -> commit.

---

## File Structure

```text
experiments/m9-networking/
├── Cargo.toml
├── scripts/
│   └── netns.sh                  # manual controlled-NAT gate; Linux only
├── src/
│   ├── lib.rs                    # experiment-only module exports
│   ├── config.rs                 # common evaluation limits/timeouts
│   ├── error.rs                  # focused experiment error type
│   ├── metrics.rs                # typed samples and stable TSV rendering
│   ├── baseline.rs               # raw Quinn measurement baseline
│   ├── candidate/
│   │   ├── mod.rs
│   │   ├── endpoint.rs           # Minimal Iroh endpoint/direct pair helpers
│   │   ├── binding.rs            # ADR-0008 exporter derivation
│   │   ├── record.rs             # bounded private u32-BE records
│   │   ├── runtime.rs            # terminal state + owned task registry
│   │   ├── control.rs            # bounded reserved control bridge
│   │   ├── stream.rs             # bounded uni-stream bridge
│   │   └── connection.rs         # experiment-only TransportConnection impl
│   ├── scenarios/
│   │   ├── mod.rs
│   │   ├── auth.rs               # deterministic Cross-Lab auth fixture/orchestration
│   │   ├── lifecycle.rs          # SimNode/SimStreamRuntime scenarios
│   │   └── relay.rs              # owner-relay + path observation scenarios
│   ├── relay.rs                  # self-hosted relay fixture/server wrapper
│   ├── netprobe.rs               # cross-process controlled-network roles
│   └── bin/
│       └── m9-networking.rs       # benchmark/netprobe CLI using std::env parsing
└── tests/
    ├── baseline.rs
    ├── exporter.rs
    ├── control.rs
    ├── stream.rs
    ├── session.rs
    ├── lifecycle.rs
    ├── relay.rs
    └── metrics.rs

docs/research/
└── M9-networking-evidence.md      # machine/environment/results + decision matrix

docs/adr/
└── ADR-0009-remote-networking.md  # Proposed only after evidence exists
```

The experiment crate is intentionally in the normal workspace so CI compiles and tests deterministic M9 code. Manual namespace/NAT measurements are not hard CI assertions because they require Linux network privileges and environment-sensitive timing.

---

### Task 1: Add the experiment crate and a reproducible Quinn measurement baseline

**Files:**
- Modify: `Cargo.toml`
- Create: `experiments/m9-networking/Cargo.toml`
- Create: `experiments/m9-networking/src/lib.rs`
- Create: `experiments/m9-networking/src/config.rs`
- Create: `experiments/m9-networking/src/error.rs`
- Create: `experiments/m9-networking/src/metrics.rs`
- Create: `experiments/m9-networking/src/baseline.rs`
- Create: `experiments/m9-networking/tests/metrics.rs`
- Create: `experiments/m9-networking/tests/baseline.rs`
- Modify: `docs/development/CURRENT.md`

**Interfaces:**
- Produces `EvalConfig`, `TransportKind`, `MetricKind`, `Measurement`, `Report`, and `baseline::run_quinn_loopback_sample`.
- No Iroh dependency exists at the end of this task; this is the verified M8 comparison harness.

- [ ] **Step 1: Write RED tests for stable measurement output and Quinn baseline coverage**

Create `tests/metrics.rs`:

```rust
use crosslab_m9_networking::metrics::{Measurement, MetricKind, Report, TransportKind};

#[test]
fn report_tsv_schema_is_stable() {
    let report = Report::new(vec![Measurement::new(
        TransportKind::Quinn,
        MetricKind::ProtectedConnectMicros,
        0,
        42,
    )]);

    assert_eq!(
        report.to_tsv(),
        "transport\tmetric\tsample\tvalue\nquinn\tprotected_connect_us\t0\t42\n"
    );
}
```

Create `tests/baseline.rs`:

```rust
use crosslab_m9_networking::{config::EvalConfig, metrics::MetricKind};

#[tokio::test]
async fn quinn_baseline_records_connect_control_bulk_and_shutdown() {
    let report = crosslab_m9_networking::baseline::run_quinn_loopback_sample(EvalConfig::test())
        .await
        .expect("Quinn baseline should complete");

    for metric in [
        MetricKind::ProtectedConnectMicros,
        MetricKind::ControlRttMicros,
        MetricKind::BulkBytesPerSecond,
        MetricKind::ShutdownMicros,
    ] {
        assert!(report.contains(metric), "missing {metric:?}");
    }
}
```

- [ ] **Step 2: Run focused tests and verify RED**

Run:

```bash
cargo test -p crosslab-m9-networking --test metrics
cargo test -p crosslab-m9-networking --test baseline
```

Expected: failure because the workspace member/package and measurement interfaces do not exist.

- [ ] **Step 3: Add the workspace member and experiment manifest**

Append only the experiment member to the root workspace:

```toml
"experiments/m9-networking",
```

Create `experiments/m9-networking/Cargo.toml`:

```toml
[package]
name = "crosslab-m9-networking"
version.workspace = true
edition.workspace = true
rust-version.workspace = true
license.workspace = true
repository.workspace = true
publish = false

[lints]
workspace = true

[dependencies]
crosslab-core = { path = "../../crates/core" }
crosslab-crypto = { path = "../../crates/crypto" }
crosslab-identity = { path = "../../crates/identity" }
crosslab-policy = { path = "../../crates/policy" }
crosslab-protocol = { path = "../../crates/protocol" }
crosslab-sim = { path = "../../apps/sim" }
crosslab-transport-quic = { path = "../../transports/quic" }
quinn.workspace = true
rcgen.workspace = true
rustls.workspace = true
tokio = { workspace = true, features = ["io-util", "macros", "net", "process", "rt-multi-thread", "sync", "time"] }
```

Do not add Iroh yet.

- [ ] **Step 4: Implement typed evaluation configuration and metrics**

Use typed enums rather than stringly-typed metric names:

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TransportKind {
    Quinn,
    IrohDirect,
    IrohRelay,
    IrohRelayThenDirect,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MetricKind {
    ProtectedConnectMicros,
    SessionAuthMicros,
    ControlRttMicros,
    BulkBytesPerSecond,
    RelayUpgradeMicros,
    ShutdownMicros,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Measurement {
    transport: TransportKind,
    metric: MetricKind,
    sample: usize,
    value: u128,
}
```

`EvalConfig::test()` must use one sample, 64 KiB bulk payload, and a 5-second timeout. `EvalConfig::default()` must use five samples, 4 MiB bulk payload, and a 15-second timeout. Keep the TSV formatter deterministic with the exact header from the RED test.

- [ ] **Step 5: Implement the raw Quinn baseline harness without changing `transports/quic` visibility**

Use the same pinned Quinn/rustls/rcgen family as M8. The experiment owns a local fixture:

```rust
pub async fn run_quinn_loopback_sample(config: EvalConfig) -> Result<Report, EvalError>;
```

For each sample:

1. create fresh loopback endpoints and ephemeral test certificate;
2. measure full protected connection establishment;
3. open one bidirectional stream and measure a bounded 32-byte ping/echo RTT;
4. open one unidirectional stream and transfer exactly `config.bulk_payload_bytes()` bytes with a fixed receive limit;
5. close both endpoints and measure shutdown completion.

Do not call `QuicTransportConnection::new`; its crate-private visibility is an intentional production boundary.

- [ ] **Step 6: Run focused tests and dependency audit**

Run:

```bash
cargo test -p crosslab-m9-networking --test metrics --test baseline
cargo tree -p crosslab-m9-networking -e features
cargo metadata --locked --format-version 1 >/dev/null
```

Expected: tests PASS; dependency tree contains no Iroh/libp2p yet.

- [ ] **Step 7: Run the normal repository gate and checkpoint CURRENT.md**

Run:

```bash
cargo fmt --all -- --check
cargo check --workspace --all-targets --all-features
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-features
```

Update `CURRENT.md` with the exact GREEN commit/CI once available and record Task 2 as next.

- [ ] **Step 8: Commit**

```bash
git add Cargo.toml Cargo.lock experiments/m9-networking docs/development/CURRENT.md
git commit -m "test(networking): add M9 Quinn baseline harness"
```

---

### Task 2: Add direct Iroh endpoints and prove exact ADR-0008 exporter compatibility

**Files:**
- Modify: `experiments/m9-networking/Cargo.toml`
- Create: `experiments/m9-networking/src/candidate/mod.rs`
- Create: `experiments/m9-networking/src/candidate/endpoint.rs`
- Create: `experiments/m9-networking/src/candidate/binding.rs`
- Create: `experiments/m9-networking/tests/exporter.rs`
- Modify: `docs/development/CURRENT.md`

**Interfaces:**
- Produces `candidate::endpoint::DirectPair` and `direct_pair()`.
- Produces `candidate::binding::derive_channel_binding(&iroh::endpoint::Connection) -> Result<ChannelBinding, EvalError>`.
- Consumes no public address lookup or default relay service.

- [ ] **Step 1: Add Iroh to the experiment only**

Pin the reviewed source/version and explicitly disable defaults:

```toml
iroh = { version = "=1.2.0", default-features = false, features = ["portmapper", "test-utils", "tls-ring"] }
```

Do not add Iroh to `[workspace.dependencies]` or any production crate.

- [ ] **Step 2: Write RED exporter/direct-address tests**

Create `tests/exporter.rs`:

```rust
#[tokio::test]
async fn direct_iroh_pair_uses_explicit_addressing_and_matching_exporter() {
    let pair = direct_pair().await.expect("direct Iroh pair");
    assert_eq!(pair.client_binding(), pair.server_binding());
    assert_eq!(pair.client_binding().profile_id(), "quic-tls-exporter-v1");
    assert_eq!(pair.client_binding().bytes().len(), 32);
    pair.shutdown().await;
}

#[tokio::test]
async fn fresh_iroh_connection_has_fresh_exporter() {
    let first = direct_pair().await.unwrap();
    let first_binding = first.client_binding().bytes().to_vec();
    first.shutdown().await;

    let second = direct_pair().await.unwrap();
    assert_ne!(first_binding, second.client_binding().bytes());
    second.shutdown().await;
}

#[tokio::test]
async fn minimal_endpoint_does_not_resolve_endpoint_id_without_explicit_address() {
    let pair = unconnected_direct_endpoints().await.unwrap();
    assert!(pair.client.connect(pair.server.id(), M9_ALPN).await.is_err());
    pair.shutdown().await;
}
```

- [ ] **Step 3: Verify RED**

Run:

```bash
cargo test -p crosslab-m9-networking --test exporter
```

Expected: compile failure because candidate endpoint/binding helpers are absent.

- [ ] **Step 4: Implement Minimal direct endpoints**

Use the exact Iroh configuration shape:

```rust
const M9_ALPN: &[u8] = b"crosslab-m9-networking-eval";

fn direct_builder() -> iroh::endpoint::Builder {
    iroh::Endpoint::builder(iroh::endpoint::presets::Minimal)
        .relay_mode(iroh::RelayMode::Disabled)
        .alpns(vec![M9_ALPN.to_vec()])
}
```

`direct_pair()` must connect with the server's explicit `EndpointAddr` (`server.addr()`), never by `EndpointId` alone. The server side accepts the normal full-handshake `Incoming`; do not call `into_0rtt`/`into_0_5rtt` or equivalent early-data APIs.

- [ ] **Step 5: Implement ADR-0008 exporter derivation exactly**

```rust
const PROFILE_ID: &str = "quic-tls-exporter-v1";
const LABEL: &[u8] = b"EXPORTER-Cross-Lab-QUIC-Channel-Binding-v1";
const CONTEXT: &[u8] = b"crosslab.quic.transport.v1";
const OUTPUT_LEN: usize = 32;

pub fn derive_channel_binding(
    connection: &iroh::endpoint::Connection,
) -> Result<ChannelBinding, EvalError> {
    let mut bytes = vec![0u8; OUTPUT_LEN];
    connection
        .export_keying_material(&mut bytes, LABEL, CONTEXT)
        .map_err(|_| EvalError::ChannelBinding)?;
    Ok(ChannelBinding::new(PROFILE_ID, bytes))
}
```

- [ ] **Step 6: Run tests and dependency review**

Run:

```bash
cargo test -p crosslab-m9-networking --test exporter
cargo tree -p crosslab-m9-networking -e features | tee /tmp/m9-features.txt
cargo metadata --locked --format-version 1 >/dev/null
```

Verify the experiment resolves Iroh `1.2.0`, no second async runtime, and no libp2p dependency.

**Decision gate:** if exact exporter semantics cannot be achieved after a full Iroh handshake, do not invent a substitute binding. Record the failure in `docs/research/M9-networking-evidence.md` and stop Iroh promotion work until the architecture decision is revisited.

- [ ] **Step 7: Full gate, CURRENT.md checkpoint, commit**

```bash
cargo fmt --all -- --check
cargo check --workspace --all-targets --all-features
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-features

git add experiments/m9-networking Cargo.lock docs/development/CURRENT.md
git commit -m "test(networking): prove Iroh exporter compatibility"
```

---

### Task 3: Build bounded record framing and reserved control bridge over Iroh

**Files:**
- Create: `experiments/m9-networking/src/candidate/record.rs`
- Create: `experiments/m9-networking/src/candidate/runtime.rs`
- Create: `experiments/m9-networking/src/candidate/control.rs`
- Create: `experiments/m9-networking/tests/control.rs`

**Interfaces:**
- Produces `CandidateConfig` with M8-equivalent bounds.
- Produces `CandidateRuntime` with shared terminal state and a joinable task registry.
- Produces `ControlBridge::{spawn, try_send, try_receive, shutdown}`.
- Does not implement `TransportConnection` yet.

- [ ] **Step 1: Write RED bounded framing/control tests**

Cover exact-limit round trip, oversized declared length rejected before allocation, truncated record failure, ordered controls, queue saturation, oversize ownership return, and peer close -> terminal.

```rust
#[tokio::test]
async fn oversized_control_returns_original_frame_without_queueing() {
    let pair = connected_control_pair(test_config().with_control_limit(nz(4))).await.unwrap();
    let frame = vec![0x55; 5];
    assert_eq!(
        pair.client.try_send(frame.clone()),
        Err(ControlSendError::TooLarge(frame))
    );
    assert_eq!(pair.server.try_receive(), Err(ControlReceiveError::Empty));
    pair.shutdown().await;
}
```

- [ ] **Step 2: Verify RED**

```bash
cargo test -p crosslab-m9-networking --test control
```

Expected: compile failure because candidate record/runtime/control modules do not exist.

- [ ] **Step 3: Implement private record framing**

Use the same private shape as M8 without extracting a shared crate:

```text
[u32 big-endian body length][body bytes]
```

Read exactly four prefix bytes, validate declared length against the supplied maximum before allocating the body, then `read_exact` the body. Never use unbounded `read_to_end`.

- [ ] **Step 4: Implement candidate config and runtime ownership**

`CandidateConfig::default()` must equal the M8 semantic limits. Add a unit test comparing every corresponding getter to `crosslab_transport_quic::QuicTransportConfig::default()`.

`CandidateRuntime` owns:

```rust
pub(crate) struct CandidateRuntime {
    terminal: Arc<AtomicBool>,
    tasks: Arc<TaskRegistry>,
}
```

`TaskRegistry::close_and_take()` must atomically stop accepting child tasks before shutdown joins the existing handles.

- [ ] **Step 5: Implement the control bridge**

`ControlBridge::spawn` takes an established Iroh connection plus the already-opened reserved bidirectional control stream. It owns bounded Tokio mpsc queues sized from `CandidateConfig`, one writer task, one reader task, and one connection-close monitor. The synchronous surface returns existing core errors (`Full`, `TooLarge`, `Closed`) without blocking.

- [ ] **Step 6: Run focused + full gate and commit**

```bash
cargo test -p crosslab-m9-networking --test control
cargo fmt --all -- --check
cargo check --workspace --all-targets --all-features
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-features

git add experiments/m9-networking Cargo.lock
git commit -m "test(networking): add bounded Iroh control bridge"
```

---

### Task 4: Add bounded unidirectional streams and the experiment-only TransportConnection

**Files:**
- Create: `experiments/m9-networking/src/candidate/stream.rs`
- Create: `experiments/m9-networking/src/candidate/connection.rs`
- Modify: `experiments/m9-networking/src/candidate/mod.rs`
- Create: `experiments/m9-networking/tests/stream.rs`

**Interfaces:**
- Produces `IrohTransportConnection`, implementing `crosslab_core::TransportConnection` only inside the experiment package.
- Consumes `CandidateRuntime`, `ControlBridge`, `CandidateConfig`, and ADR-0008 `ChannelBinding`.

- [ ] **Step 1: Write RED stream lifecycle tests**

Mirror the M8 semantic matrix, not Quinn APIs:

```rust
#[tokio::test]
async fn iroh_uni_stream_preserves_opening_chunks_fin_and_backpressure() {
    let pair = promoted_transport_pair(test_config()).await.unwrap();
    let mut send = pair.client.try_open_uni_stream(b"open".to_vec()).unwrap();
    let incoming = eventually_accept(&pair.server).await;
    let (_, mut recv) = incoming.into_parts();

    send.try_send_chunk(b"one".to_vec()).unwrap();
    assert_eq!(eventually_receive(recv.as_mut()).await, b"one");
    send.finish();
    eventually_finished(recv.as_mut()).await;
    pair.shutdown().await;
}
```

Also cover stream-slot saturation, oversized opening/chunk ownership return, chunk queue saturation, sender RESET/drop -> receiver `Cancelled`, receiver STOP/cancel -> sender closed, connection close cancelling active streams, and joined shutdown.

- [ ] **Step 2: Verify RED**

```bash
cargo test -p crosslab-m9-networking --test stream
```

Expected: missing `IrohTransportConnection`/stream driver API.

- [ ] **Step 3: Implement bounded outgoing/incoming uni-stream drivers**

Use synchronous semaphore reservation before `try_open_uni_stream` returns. Each outgoing stream owns a bounded chunk queue and the permit until finish/cancel/drop. Each incoming stream validates the bounded opening record before becoming visible and feeds chunks into a bounded queue.

Map Iroh QUIC semantics exactly:

```text
clean FIN       -> StreamReceiveError::Finished after queued chunks drain
sender reset    -> StreamReceiveError::Cancelled
receiver stop   -> future sender sends fail Closed
connection loss -> all stream authority terminal/Cancelled
```

- [ ] **Step 4: Implement `TransportConnection`**

Required semantic surface:

```rust
impl TransportConnection for IrohTransportConnection {
    fn security_class(&self) -> TransportSecurityClass {
        TransportSecurityClass::AuthenticatedConfidentialChannel
    }
    // channel_binding, metadata, try_send/receive_control,
    // try_open/accept_uni_stream, close, is_closed
}
```

Do not expose Iroh types through `crosslab-core` or another production crate.

- [ ] **Step 5: Focused + full verification and commit**

```bash
cargo test -p crosslab-m9-networking --test control --test stream
cargo fmt --all -- --check
cargo check --workspace --all-targets --all-features
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-features

git add experiments/m9-networking
git commit -m "test(networking): adapt bounded Iroh transport semantics"
```

---

### Task 5: Prove the existing Cross-Lab session-auth protocol over Iroh

**Files:**
- Create: `experiments/m9-networking/src/scenarios/mod.rs`
- Create: `experiments/m9-networking/src/scenarios/auth.rs`
- Create: `experiments/m9-networking/tests/session.rs`
- Modify: `docs/development/CURRENT.md`

**Interfaces:**
- Produces deterministic `AuthFixture`, `AuthAttempt`, and `AuthenticatedIrohPair` test/evaluation helpers.
- Promotes the same reserved bidirectional stream from bootstrap framing into `IrohTransportConnection` only after both `LogicalSession`s are `Active`.

- [ ] **Step 1: Write RED authentication scenarios**

Required tests:

```rust
#[tokio::test]
async fn trusted_peers_activate_over_iroh_exporter() { /* assert both Active */ }

#[tokio::test]
async fn proof_bound_to_wrong_iroh_connection_is_rejected() { /* assert Closed */ }

#[tokio::test]
async fn proof_replay_after_iroh_reconnect_is_rejected() { /* old proof fails */ }

#[tokio::test]
async fn ordinary_control_cannot_promote_before_both_sessions_are_active() { /* fail closed */ }
```

- [ ] **Step 2: Verify RED**

```bash
cargo test -p crosslab-m9-networking --test session
```

Expected: missing auth fixture/orchestration.

- [ ] **Step 3: Implement deterministic credentials and existing bootstrap messages**

Use the public APIs already exercised by M8: `OwnerRootRecord`, `AuthorityDelegation`, `DeviceCredential`, `TrustRecord`, `SessionAuthTranscriptV1`, `SessionActivation`, `SessionAuthHello`, `SessionAuthProofMessage`, `encode_session_auth_bootstrap`, and `decode_session_auth_bootstrap`.

Derive deterministic fixture keys/IDs from fixed test byte arrays only. Do not construct a new signature transcript or use Iroh endpoint identity as a Cross-Lab credential.

- [ ] **Step 4: Promote only after both sessions are active**

The orchestration sequence is:

```text
full Iroh handshake
-> ADR-0008 exporter
-> reserved bi stream
-> existing hello/proof exchange
-> LogicalSession::Active on both peers
-> consume same bi stream into IrohTransportConnection
```

- [ ] **Step 5: Full verification, durable checkpoint, commit**

```bash
cargo test -p crosslab-m9-networking --test session
cargo fmt --all -- --check
cargo check --workspace --all-targets --all-features
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-features

git add experiments/m9-networking docs/development/CURRENT.md
git commit -m "test(networking): authenticate Cross-Lab sessions over Iroh"
```

---

### Task 6: Re-run Cross-Lab control/data/reconnect/revocation semantics over direct Iroh

**Files:**
- Create: `experiments/m9-networking/src/scenarios/lifecycle.rs`
- Create: `experiments/m9-networking/tests/lifecycle.rs`

**Interfaces:**
- Consumes `AuthenticatedIrohPair`.
- Reuses production `SimNode`, `SimStreamRuntime`, `PolicyState`, operation authorization, stream admission, and revocation APIs unchanged.

- [ ] **Step 1: Write RED lifecycle scenarios**

Port the semantic assertions, not the Quinn fixture, from M8:

1. capability advertisement + request/response/event;
2. authorized operation-bound uni stream;
3. unknown operation rejected;
4. fresh reconnect changes exporter + `SessionId` and clears old request/operation authority;
5. old proof/session/operation authority rejected after reconnect;
6. signed peer revocation terminates active authority;
7. reconnect after revocation cannot reach `Active`;
8. transport saturation/cancellation/shutdown fail closed.

Use `NetworkClass::Remote` in every Iroh authorization context.

- [ ] **Step 2: Verify RED**

```bash
cargo test -p crosslab-m9-networking --test lifecycle
```

Expected: lifecycle helpers/scenarios missing.

- [ ] **Step 3: Implement only orchestration/helpers needed by the tests**

Do not add new policy exceptions or transport-specific operation logic. Existing `SimNode::new` and `SimStreamRuntime::new` remain the behavior owners.

Add an explicit LocalOnly regression:

```rust
assert_eq!(
    remote_context_with_local_only_rule().decision().reason(),
    DecisionReason::ConstraintFailed
);
```

- [ ] **Step 4: Focused + full gate and commit**

```bash
cargo test -p crosslab-m9-networking --test lifecycle
cargo fmt --all -- --check
cargo check --workspace --all-targets --all-features
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-features

git add experiments/m9-networking
git commit -m "test(networking): prove Cross-Lab lifecycle over Iroh"
```

---

### Task 7: Add owner-controlled relay fixtures, relay-only transport, and path-observation invariants

**Files:**
- Modify: `experiments/m9-networking/Cargo.toml`
- Create: `experiments/m9-networking/src/relay.rs`
- Create: `experiments/m9-networking/src/scenarios/relay.rs`
- Create: `experiments/m9-networking/tests/relay.rs`
- Modify: `docs/development/CURRENT.md`

**Interfaces:**
- Produces `OwnerRelay` with explicit start/url/shutdown lifecycle.
- Produces relay-only and relay-then-direct authenticated Iroh fixtures.
- Exposes path observations only to metrics/tests; no policy mutator exists.

- [ ] **Step 1: Add experiment-only relay server dependency**

```toml
iroh-relay = { version = "=1.2.0", default-features = false, features = ["server", "test-utils", "tls-ring"] }
```

This dependency remains inside the experiment crate.

- [ ] **Step 2: Write RED self-hosted-relay tests**

Required cases:

```rust
#[tokio::test]
async fn relay_only_pair_uses_owner_relay_without_public_lookup() { /* authenticated control/data */ }

#[tokio::test]
async fn relay_connection_can_observe_direct_path_upgrade_without_reclassification() {
    let mut pair = relay_then_direct_pair().await.unwrap();
    assert_eq!(pair.network_class(), NetworkClass::Remote);
    pair.wait_for_direct_path(TEST_TIMEOUT).await.unwrap();
    assert_eq!(pair.network_class(), NetworkClass::Remote);
    assert_eq!(pair.binding_before_path_change(), pair.current_binding());
    assert_eq!(pair.session_id_before_path_change(), pair.current_session_id());
    pair.shutdown().await;
}
```

Also assert that relay credentials/endpoint IDs are not accepted as Cross-Lab credentials by any fixture API.

- [ ] **Step 3: Verify RED**

```bash
cargo test -p crosslab-m9-networking --test relay
```

- [ ] **Step 4: Implement an owned local relay wrapper**

Use `iroh_relay::server::Server::spawn` with `ServerConfig` containing `RelayConfig::new(bind_addr)` and no mandatory external TLS/ACME/public service. `OwnerRelay::shutdown(self)` must await server shutdown. Tests may bind port `0` and use `server.http_url()` from the `test-utils` feature.

- [ ] **Step 5: Implement relay-only and relay->direct endpoint modes**

Relay-only endpoints:

```rust
Endpoint::builder(presets::Minimal)
    .clear_ip_transports()
    .relay_mode(RelayMode::Custom(relay_map.clone()))
    .alpns(vec![M9_ALPN.to_vec()])
```

Relay-then-direct endpoints keep IP transports enabled but connect initially with:

```rust
EndpointAddr::new(server.id()).with_relay_url(relay_url)
```

Observe `paths_stream()`/`path_events()` until an IP path appears. Treat `Lagged` as a diagnostics event and recover current state from `connection.paths()` rather than dropping policy/session authority.

- [ ] **Step 6: Run required semantic tests and full gate**

```bash
cargo test -p crosslab-m9-networking --test relay --test session --test lifecycle
cargo fmt --all -- --check
cargo check --workspace --all-targets --all-features
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-features
```

**Decision gate:** if owner-selected/self-hosted relay operation requires public n0 services or cannot preserve session/binding/policy semantics, record the exact failure. Do not silently switch to public infrastructure.

- [ ] **Step 7: Update CURRENT.md and commit**

```bash
git add experiments/m9-networking Cargo.lock docs/development/CURRENT.md
git commit -m "test(networking): validate owner relay and Iroh path changes"
```

---

### Task 8: Add reproducible benchmark output and the controlled Linux NAT/relay gate

**Files:**
- Modify: `experiments/m9-networking/src/metrics.rs`
- Modify: `experiments/m9-networking/src/baseline.rs`
- Create: `experiments/m9-networking/src/netprobe.rs`
- Create: `experiments/m9-networking/src/bin/m9-networking.rs`
- Create: `experiments/m9-networking/scripts/netns.sh`
- Create: `docs/research/M9-networking-evidence.md`
- Create: `experiments/m9-networking/tests/baseline.rs` additions
- Create: `experiments/m9-networking/tests/relay.rs` additions

**Interfaces:**
- CLI produces stable TSV to stdout/file for local benchmark modes.
- `netprobe` supports explicit relay/server/client roles for the privileged namespace script.
- Evidence document records environment and raw/summary results; no WAN timing is turned into a hard correctness assertion.

- [ ] **Step 1: Write RED CLI/report tests**

Test argument parsing without adding Clap:

```rust
#[test]
fn parse_local_benchmark_args() {
    let command = Command::parse([
        "local", "--samples", "3", "--payload-bytes", "1048576"
    ]).unwrap();
    assert_eq!(command.samples(), 3);
    assert_eq!(command.payload_bytes(), 1_048_576);
}
```

Test the report always includes environment metadata fields: Rust version, OS/arch, sample count, payload bytes, and dependency labels `quinn=0.11.11` / `iroh=1.2.0`.

- [ ] **Step 2: Verify RED**

```bash
cargo test -p crosslab-m9-networking --test metrics --test baseline --test relay
```

- [ ] **Step 3: Implement local benchmark modes**

CLI modes:

```text
m9-networking local-quinn
m9-networking local-iroh-direct
m9-networking local-iroh-relay
m9-networking local-all
m9-networking netprobe-relay
m9-networking netprobe-server
m9-networking netprobe-client
```

Every local mode records protected-connect time, Cross-Lab session-auth time, control RTT, fixed-size uni throughput, shutdown time, and process RSS/FD observations where available. On Linux, collect RSS/FD counts from `/proc/self/status` and `/proc/self/fd`; return `None` on unsupported platforms rather than adding unsafe OS APIs.

- [ ] **Step 4: Implement cross-process netprobe roles**

Use plain text rendezvous files with explicit fields, one per line:

```text
endpoint_id=<hex/display EndpointId>
relay_url=<RelayUrl or ->
ip=<SocketAddr>
```

Reconstruct `EndpointAddr` with `EndpointAddr::new(id).with_relay_url(...).with_ip_addr(...)`. Never serialize Cross-Lab credentials, Iroh secret keys, relay tokens, proofs, or exporter bytes into rendezvous files.

- [ ] **Step 5: Implement `scripts/netns.sh` with fail-safe cleanup**

The script must:

1. require Linux root plus `ip`, `nft`, and the built `m9-networking` binary;
2. create a transit bridge and two router namespaces;
3. create one peer namespace behind each router;
4. enable forwarding only in router namespaces;
5. use nftables masquerade on each router external interface to produce two independent translated networks;
6. launch the owner relay on the transit network;
7. run a relay-only probe with direct UDP blocked between router external interfaces;
8. enable direct UDP and run/observe relay->direct upgrade;
9. force an endpoint restart/address change and record reconnect behavior;
10. trap EXIT/INT/TERM and remove every namespace, bridge, nft table, process, and temporary rendezvous file.

Use fixed RFC1918 test ranges owned only by the script, for example transit `172.30.90.0/24`, peer A `10.90.1.0/24`, peer B `10.90.2.0/24`. Refuse to run if any planned namespace already exists.

- [ ] **Step 6: Run deterministic CI-safe verification**

```bash
cargo test -p crosslab-m9-networking
cargo fmt --all -- --check
cargo check --workspace --all-targets --all-features
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-features
```

- [ ] **Step 7: Run reproducible local measurements**

On the same host, from a clean build:

```bash
cargo run -p crosslab-m9-networking --bin m9-networking -- local-all --samples 10 --payload-bytes 4194304 \
  > /tmp/m9-local.tsv
/usr/bin/time -v cargo run -p crosslab-m9-networking --bin m9-networking -- local-all --samples 10 --payload-bytes 4194304 \
  > /tmp/m9-local-time.tsv 2> /tmp/m9-local-time.txt
```

Record CPU model, RAM, kernel, Rust version, power mode/VM/container status, and whether other traffic/load was present.

- [ ] **Step 8: Run the controlled NAT/relay manual gate**

```bash
cargo build -p crosslab-m9-networking --bin m9-networking
sudo experiments/m9-networking/scripts/netns.sh \
  target/debug/m9-networking \
  /tmp/m9-netns.tsv
```

Required recorded outcomes:

- relay fallback succeeds with direct UDP blocked;
- Cross-Lab auth/control/data work over relay;
- when direct UDP is enabled, Iroh observes a direct IP path and the same live connection/session remains `NetworkClass::Remote`;
- forced connection restart yields fresh exporter/session authority;
- if direct hole punching fails, preserve logs/topology and record the failure rather than treating local relay success as NAT success.

- [ ] **Step 9: Populate the evidence report**

`docs/research/M9-networking-evidence.md` must contain:

```text
Environment
Dependency/feature tree
Security/owner-control eligibility
Quinn baseline measurements
Iroh direct measurements
Iroh relay measurements
Controlled NAT results
Recovery/path-change results
Resource observations
Mobile/platform source-review obligations
Failures/anomalies
Decision matrix
Libp2p trigger: yes/no + exact reason
```

Never paste secret keys, relay tokens, proofs, exporter bytes, or payload contents.

- [ ] **Step 10: Commit the verified experiment/evidence checkpoint**

```bash
git add experiments/m9-networking docs/research/M9-networking-evidence.md Cargo.lock
git commit -m "test(networking): measure M9 remote connectivity candidates"
```

---

### Task 9: Conditional focused rust-libp2p comparison only if the evidence trigger is satisfied

**Trigger rule:** execute this task only if `docs/research/M9-networking-evidence.md` records a failed Iroh decision criterion and explains why libp2p's Relay v2/DCUtR/AutoNAT architecture plausibly addresses that exact failure. A generic desire to compare libraries is not sufficient.

**Files if triggered:**
- Modify: `experiments/m9-networking/Cargo.toml`
- Create: `experiments/m9-networking/src/libp2p_candidate.rs`
- Create: `experiments/m9-networking/tests/libp2p_candidate.rs`
- Modify: `docs/research/M9-networking-evidence.md`

**Interfaces:**
- Produces only a focused measurement/probe for the recorded failure criterion.
- Does not implement a second Cross-Lab identity/session model and does not graduate libp2p into production.

- [ ] **Step 1: Re-verify the trigger before changing Cargo.toml**

The evidence report must contain one explicit line:

```text
Libp2p trigger: yes — <failed Iroh criterion> — <why relay/DCUtR/AutoNAT plausibly addresses it>
```

If this line cannot be written truthfully, skip Task 9 and do not add libp2p.

- [ ] **Step 2: Add the smallest relevant libp2p feature set**

For a NAT/relay trigger, use:

```toml
libp2p = { version = "=0.57.0", default-features = false, features = ["autonat", "dcutr", "identify", "macros", "quic", "relay", "tokio"] }
```

Do not enable `full`, Kademlia, gossip, mDNS, TCP, WebSocket, or unrelated behaviors.

- [ ] **Step 3: Write a RED probe for the exact failed criterion**

For example, if the trigger is relay->direct upgrade reliability, the test/probe must assert only connection establishment through Circuit Relay v2 and DCUtR direct upgrade observability in the controlled topology. Do not duplicate the full Iroh adapter.

- [ ] **Step 4: Verify source-level channel-binding limitation before any Cross-Lab promotion**

The uploaded libp2p `0.57.0` QUIC wrapper stores `quinn::Connection` privately. Unless the published API exposes a reviewed cryptographic binding equivalent during execution, the libp2p probe is **not eligible** to become a Cross-Lab protected transport. Record this explicitly; do not substitute `PeerId`, Noise identity, Multiaddr, or connection IDs for ADR-0008.

- [ ] **Step 5: Run focused probe, dependency audit, and full gate**

```bash
cargo test -p crosslab-m9-networking --test libp2p_candidate
cargo tree -p crosslab-m9-networking -e features
cargo fmt --all -- --check
cargo check --workspace --all-targets --all-features
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-features
```

- [ ] **Step 6: Update evidence and commit**

```bash
git add experiments/m9-networking docs/research/M9-networking-evidence.md Cargo.lock
git commit -m "test(networking): compare libp2p for M9 decision gap"
```

If Task 9 is skipped, record `Libp2p trigger: no` with the passing Iroh criteria and source-level integration cost rationale.

---

### Task 10: Write ADR-0009, reconcile architecture, verify exact head, merge, and hand off M10

**Files:**
- Create: `docs/adr/ADR-0009-remote-networking.md`
- Modify: `docs/adr/README.md`
- Modify: `docs/architecture/MASTER-ARCHITECTURE.md`
- Modify: `docs/plans/phase-1/M9-remote-networking-design.md`
- Modify: `docs/development/CURRENT.md`
- Modify or remove experiment candidate dependencies/code only as dictated by the ADR; do not graduate them automatically.

**Interfaces:**
- Produces the durable remote-networking decision and exact M10 next task.
- If Iroh is selected, the ADR selects the architecture, not a production endpoint manager or platform integration API.

- [ ] **Step 1: Decide strictly from the evidence matrix**

Select **Quinn local/LAN + Iroh remote** only if every mandatory design criterion passed:

```text
ADR-0008 exporter exact: pass
existing Cross-Lab auth/control/data/reconnect/revocation: pass
owner-selected/self-hosted relay without mandatory public infra: pass
path observation without policy reclassification: pass
controlled NAT/relay/recovery evidence: acceptable
bounded resources/shutdown: pass
platform/maintenance cost: acceptable
```

If any mandatory criterion failed and no candidate passes, ADR-0009 must record no remote selection rather than pretending M9 succeeded.

- [ ] **Step 2: Write ADR-0009 with required sections**

Required structure:

```markdown
# ADR-0009: Remote networking architecture

**Status:** Proposed
**Date:** 2026-09-12

## Context
## Decision
## Alternatives considered
## Security impact
## Compatibility impact
## Operational impact
## Consequences
```

If Iroh passes, decision text must state:

- Quinn remains local/LAN baseline;
- Iroh is selected for remote/NAT/relay connectivity;
- Iroh endpoint identity remains transport-only;
- `quic-tls-exporter-v1` is reused exactly after full Iroh handshake;
- Iroh sessions are always `NetworkClass::Remote`;
- owner-selected/self-hosted relays are supported and no n0/Cross-Lab public service is mandatory;
- M10 must still prove Android/mobile lifecycle and platform integration before remote networking is production-ready.

- [ ] **Step 3: Review/accept the ADR before changing normative architecture status**

Do not mark the ADR `Accepted` or change the Master Architecture candidate/selected status until the evidence and ADR text are reviewed and approved.

- [ ] **Step 4: After approval, reconcile normative docs**

Update the Master Architecture networking technology/status table and unresolved-decision table to match ADR-0009. Update the M9 design status to Implemented/Decision Complete. Index ADR-0009 in `docs/adr/README.md`.

- [ ] **Step 5: Decide experiment retention**

If Iroh is selected, keep the experiment/evidence harness as reproducible research unless M10 immediately needs a production adapter. Do **not** move `IrohTransportConnection` into `transports/` during M9 solely because tests pass.

If Iroh is rejected, remove unused candidate dependency/code only after the evidence report and ADR preserve the reasons/results. Keep enough benchmark tooling to reproduce the decision if useful.

- [ ] **Step 6: Write the durable M10 handoff in CURRENT.md**

Record:

- exact M9 feature/evidence head;
- CI IDs and manual controlled-network environment/result status;
- ADR-0009 status/decision;
- whether libp2p Task 9 was triggered;
- production dependencies changed or intentionally unchanged;
- exact next task: M10 Linux + Android first platform vertical slice from the Master Architecture;
- explicit mobile validation obligations inherited from M9.

- [ ] **Step 7: Run the exact final repository gate**

```bash
cargo metadata --locked --format-version 1 >/dev/null
cargo fmt --all -- --check
cargo check --workspace --all-targets --all-features
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-features
```

Re-run the deterministic M9 suite explicitly:

```bash
cargo test -p crosslab-m9-networking
```

The controlled namespace/NAT gate must already be recorded in the evidence report; do not replace it with loopback tests.

- [ ] **Step 8: Commit the exact reviewed M9 closeout head**

```bash
git add docs experiments/m9-networking Cargo.toml Cargo.lock
git commit -m "docs: complete M9 remote networking decision"
```

- [ ] **Step 9: Update draft PR #19 with exact evidence and mark ready only when complete**

PR body must list the selected/rejected architecture, evidence report, controlled NAT status, dependency isolation, exact head SHA, and full CI run. Merge only that exact verified head.

- [ ] **Step 10: Merge and verify canonical main**

After the exact PR head is green and reviewed:

1. merge with an expected-head SHA guard;
2. confirm `main` points to the merge commit;
3. wait for push CI on `main`;
4. require the same full gate to pass;
5. update `CURRENT.md` on `main` only if needed to record the canonical merge/CI checkpoint;
6. verify that final documentation-only head too.

---

## Plan Self-Review Checklist

Before execution begins, verify these mappings:

- Spec sections 7–9 (Minimal endpoint, identity separation, exporter) -> Tasks 2 and 5.
- Spec sections 10–11 (Remote classification/path changes/reconnect) -> Tasks 6 and 7.
- Spec sections 12–13 (control/data mapping and resource bounds) -> Tasks 3 and 4.
- Spec sections 14–15 (semantic matrix, measurements, controlled NAT) -> Tasks 6–8.
- Spec sections 16–17 (security/privacy/dependency review) -> Tasks 2, 7–10.
- Spec sections 18–20 (decision rule, non-goals, deliverables) -> Tasks 9–10.
- No production Iroh/libp2p dependency is introduced by this plan before ADR acceptance.
- No task changes Cross-Lab identity, policy, session transcript, or `TransportConnection` signatures.
- No placeholder implementation step or unbounded network operation remains in the plan.
