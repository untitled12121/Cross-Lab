# M9 Remote Networking ADR Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Produce reproducible evidence for Cross-Lab remote connectivity, validate Iroh against the verified Quinn baseline without weakening identity/session/policy boundaries, and close M9 with an evidence-backed remote-networking ADR.

**Architecture:** Keep the verified M8 Quinn adapter unchanged as the local/LAN production baseline. Add one non-publishable `experiments/m9-networking` workspace crate that owns candidate dependencies, measurements, self-hosted relay fixtures, and controlled-network tooling. The Iroh adapter exists only inside the experiment until ADR-0009 selects it; no candidate type or dependency enters Cross-Lab domain crates during M9.

**Tech Stack:** Rust 2024 / repository Rust 1.98 baseline, Quinn `0.11.11`, Tokio `1.53.1`, Iroh `1.2.0`, Iroh Relay `1.2.0`, rustls `0.23.44`, rcgen `0.14.10`; rust-libp2p `0.57.0` only when Task 9's recorded trigger is satisfied.

**Spec:** `docs/plans/phase-1/M9-remote-networking-design.md`

## Global Constraints

- Preserve `docs/architecture/SESSION-TRANSPORT.md`, ADR-0008, and all verified M8 session/control/stream/reconnect/revocation semantics.
- Required tests use no public infrastructure: use `Endpoint::builder(presets::Minimal)` plus explicit addresses and explicit relay maps; never silently use the Iroh `N0` preset or default n0 relay map.
- Iroh `EndpointId`, Iroh `SecretKey`, relay URLs/tokens, path IDs, libp2p `PeerId`, `Multiaddr`, and `Swarm` state remain routing metadata only.
- Reuse ADR-0008 exactly after a full Iroh handshake: `quic-tls-exporter-v1`, 32 bytes, label `EXPORTER-Cross-Lab-QUIC-Channel-Binding-v1`, context `crosslab.quic.transport.v1`.
- Do not use Iroh/libp2p 0-RTT or early-data paths for Cross-Lab authority.
- Every Iroh-backed Cross-Lab session is `NetworkClass::Remote` for its entire lifetime, including relay/direct path changes.
- Path changes inside one live Iroh connection do not change exporter binding, `SessionId`, sequence state, policy class, or operation grants.
- A new Iroh connection requires fresh Cross-Lab authentication and authority.
- Candidate bounds start from M8 defaults: control queue 8; incoming queue 8; outgoing slots 8; chunk queue 8; max control `256 KiB + 4`; max opening `4 KiB + 4`; max chunk `64 KiB`; remote uni 32; remote bi 1; stream window `512 KiB`; connection window `4 MiB`; idle timeout 30 seconds.
- No unbounded channels, attacker-sized allocation before validation, detached tasks, infinite reconnect loops, or unconstrained relay retry loops.
- Candidate dependencies stay under `experiments/m9-networking`; do not add them to `crosslab-core`, protocol, identity, policy, simulator, or `transports/quic`.
- Do not extract a generic transport framework crate during M9.
- Iroh is selected only by ADR-0009 after deterministic and controlled-network evidence is recorded.
- Do not add libp2p unless Task 9's trigger is recorded in the evidence document.
- Each behavior slice follows RED -> verified failure -> minimal GREEN -> focused tests -> full gate -> commit.

## File Structure

```text
experiments/m9-networking/
├── Cargo.toml
├── scripts/netns.sh
├── src/
│   ├── lib.rs
│   ├── config.rs
│   ├── error.rs
│   ├── metrics.rs
│   ├── baseline.rs
│   ├── relay.rs
│   ├── netprobe.rs
│   ├── candidate/
│   │   ├── mod.rs
│   │   ├── endpoint.rs
│   │   ├── binding.rs
│   │   ├── record.rs
│   │   ├── runtime.rs
│   │   ├── control.rs
│   │   ├── stream.rs
│   │   └── connection.rs
│   ├── scenarios/
│   │   ├── mod.rs
│   │   ├── auth.rs
│   │   ├── lifecycle.rs
│   │   └── relay.rs
│   └── bin/m9-networking.rs
└── tests/
    ├── metrics.rs
    ├── baseline.rs
    ├── exporter.rs
    ├── control.rs
    ├── stream.rs
    ├── session.rs
    ├── lifecycle.rs
    └── relay.rs

docs/research/M9-networking-evidence.md
docs/adr/ADR-0009-remote-networking.md
```

The experiment is a normal workspace member so deterministic code is checked by CI. Privileged namespace/NAT measurements are a documented manual gate, not timing assertions in hosted CI.

---

### Task 1: Reproducible Quinn baseline and typed measurement schema

**Files:**
- Modify: `Cargo.toml`
- Create: `experiments/m9-networking/Cargo.toml`
- Create: `experiments/m9-networking/src/{lib.rs,config.rs,error.rs,metrics.rs,baseline.rs}`
- Create: `experiments/m9-networking/tests/{metrics.rs,baseline.rs}`
- Modify: `docs/development/CURRENT.md`

**Interfaces:**

```rust
pub struct EvalConfig { /* private typed fields */ }
pub enum TransportKind { Quinn, IrohDirect, IrohRelay, IrohRelayThenDirect }
pub enum MetricKind {
    ProtectedConnectMicros,
    SessionAuthMicros,
    ControlRttMicros,
    BulkBytesPerSecond,
    RelayUpgradeMicros,
    ShutdownMicros,
}
pub struct Measurement { /* typed transport/metric/sample/value */ }
pub struct Report { /* Vec<Measurement> */ }
pub async fn run_quinn_loopback_sample(config: EvalConfig) -> Result<Report, EvalError>;
```

- [ ] **Step 1: Write RED tests**

```rust
#[test]
fn report_tsv_schema_is_stable() {
    let report = Report::new(vec![Measurement::new(
        TransportKind::Quinn,
        MetricKind::ProtectedConnectMicros,
        0,
        42,
    )]);
    assert_eq!(report.to_tsv(),
        "transport\tmetric\tsample\tvalue\nquinn\tprotected_connect_us\t0\t42\n");
}

#[tokio::test]
async fn quinn_baseline_records_required_transport_metrics() {
    let report = baseline::run_quinn_loopback_sample(EvalConfig::test()).await.unwrap();
    for metric in [
        MetricKind::ProtectedConnectMicros,
        MetricKind::ControlRttMicros,
        MetricKind::BulkBytesPerSecond,
        MetricKind::ShutdownMicros,
    ] {
        assert!(report.contains(metric));
    }
}
```

- [ ] **Step 2: Verify RED**

```bash
cargo test -p crosslab-m9-networking --test metrics
cargo test -p crosslab-m9-networking --test baseline
```

Expected: package/interfaces absent.

- [ ] **Step 3: Add the experiment member and manifest**

Root workspace gains only `"experiments/m9-networking"`.

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

- [ ] **Step 4: Implement metrics/config and raw Quinn fixture**

`EvalConfig::test()` = one sample, 64 KiB payload, 5-second timeout. Default = five samples, 4 MiB payload, 15-second timeout.

`run_quinn_loopback_sample` creates fresh loopback endpoints/certificates for each sample, measures full protected connection establishment, one bounded 32-byte bidirectional ping/echo, one fixed-size unidirectional transfer with an explicit receive limit, and endpoint shutdown. Do not change `QuicTransportConnection::new` visibility.

- [ ] **Step 5: Verify focused tests and dependency tree**

```bash
cargo test -p crosslab-m9-networking --test metrics --test baseline
cargo tree -p crosslab-m9-networking -e features
cargo metadata --locked --format-version 1 >/dev/null
```

No Iroh/libp2p should be present yet.

- [ ] **Step 6: Full gate, CURRENT checkpoint, commit**

```bash
cargo fmt --all -- --check
cargo check --workspace --all-targets --all-features
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-features
git add Cargo.toml Cargo.lock experiments/m9-networking docs/development/CURRENT.md
git commit -m "test(networking): add M9 Quinn baseline harness"
```

---

### Task 2: Direct Iroh endpoints and exact ADR-0008 exporter proof

**Files:**
- Modify: `experiments/m9-networking/Cargo.toml`
- Create: `src/candidate/{mod.rs,endpoint.rs,binding.rs}`
- Create: `tests/exporter.rs`
- Modify: `docs/development/CURRENT.md`

**Interfaces:**

```rust
pub const M9_ALPN: &[u8] = b"crosslab-m9-networking-eval";
pub struct DirectPair { /* endpoints, established connections, bindings */ }
pub async fn direct_pair() -> Result<DirectPair, EvalError>;
pub async fn unconnected_direct_endpoints() -> Result<UnconnectedDirectPair, EvalError>;
pub fn derive_channel_binding(
    connection: &iroh::endpoint::Connection,
) -> Result<ChannelBinding, EvalError>;
```

- [ ] **Step 1: Add Iroh only to the experiment**

```toml
iroh = { version = "=1.2.0", default-features = false, features = ["portmapper", "test-utils", "tls-ring"] }
```

- [ ] **Step 2: Write RED tests**

```rust
#[tokio::test]
async fn direct_pair_has_matching_exporter() {
    let pair = direct_pair().await.unwrap();
    assert_eq!(pair.client_binding(), pair.server_binding());
    assert_eq!(pair.client_binding().profile_id(), "quic-tls-exporter-v1");
    assert_eq!(pair.client_binding().bytes().len(), 32);
    pair.shutdown().await;
}

#[tokio::test]
async fn reconnect_changes_exporter() {
    let first = direct_pair().await.unwrap();
    let old = first.client_binding().bytes().to_vec();
    first.shutdown().await;
    let second = direct_pair().await.unwrap();
    assert_ne!(old, second.client_binding().bytes());
    second.shutdown().await;
}

#[tokio::test]
async fn minimal_endpoint_requires_explicit_address_data() {
    let pair = unconnected_direct_endpoints().await.unwrap();
    assert!(pair.client().connect(pair.server().id(), M9_ALPN).await.is_err());
    pair.shutdown().await;
}
```

- [ ] **Step 3: Verify RED**

```bash
cargo test -p crosslab-m9-networking --test exporter
```

- [ ] **Step 4: Implement Minimal direct endpoints and binding**

```rust
fn direct_builder() -> iroh::endpoint::Builder {
    iroh::Endpoint::builder(iroh::endpoint::presets::Minimal)
        .relay_mode(iroh::RelayMode::Disabled)
        .alpns(vec![M9_ALPN.to_vec()])
}

const PROFILE_ID: &str = "quic-tls-exporter-v1";
const LABEL: &[u8] = b"EXPORTER-Cross-Lab-QUIC-Channel-Binding-v1";
const CONTEXT: &[u8] = b"crosslab.quic.transport.v1";

pub fn derive_channel_binding(connection: &iroh::endpoint::Connection)
    -> Result<ChannelBinding, EvalError>
{
    let mut bytes = vec![0u8; 32];
    connection.export_keying_material(&mut bytes, LABEL, CONTEXT)
        .map_err(|_| EvalError::ChannelBinding)?;
    Ok(ChannelBinding::new(PROFILE_ID, bytes))
}
```

Connect using `server.addr()`, not `EndpointId` alone. Await the normal full handshake; never call Iroh early-data conversion APIs.

- [ ] **Step 5: Verify version/features and full gate**

```bash
cargo test -p crosslab-m9-networking --test exporter
cargo tree -p crosslab-m9-networking -e features
cargo metadata --locked --format-version 1 >/dev/null
cargo fmt --all -- --check
cargo check --workspace --all-targets --all-features
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-features
```

If exact ADR-0008 semantics fail, do not substitute another binding. Create `docs/research/M9-networking-evidence.md` immediately with the exact command, environment, failure, and `Libp2p trigger: no` unless source evidence shows libp2p plausibly solves that same problem.

- [ ] **Step 6: Checkpoint and commit**

```bash
git add experiments/m9-networking Cargo.lock docs/development/CURRENT.md docs/research
git commit -m "test(networking): prove Iroh exporter compatibility"
```

---

### Task 3: Bounded record framing and reserved control bridge

**Files:**
- Create: `src/candidate/{record.rs,runtime.rs,control.rs}`
- Create: `tests/control.rs`

**Interfaces:**

```rust
pub(crate) struct CandidateConfig { /* M8-equivalent typed limits */ }
pub(crate) struct CandidateRuntime { /* terminal + TaskRegistry */ }
pub(crate) struct ControlBridge { /* bounded tx/rx */ }
impl ControlBridge {
    pub(crate) fn try_send(&self, frame: Vec<u8>) -> Result<(), ControlSendError>;
    pub(crate) fn try_receive(&self) -> Result<Vec<u8>, ControlReceiveError>;
    pub(crate) async fn shutdown(self);
}
```

- [ ] **Step 1: Write RED tests for framing/order/backpressure/close**

```rust
#[tokio::test]
async fn oversized_control_returns_original_frame_without_queueing() {
    let pair = connected_control_pair(test_config_with_control_limit(4)).await.unwrap();
    let frame = vec![0x55; 5];
    assert_eq!(pair.client.try_send(frame.clone()), Err(ControlSendError::TooLarge(frame)));
    assert_eq!(pair.server.try_receive(), Err(ControlReceiveError::Empty));
    pair.shutdown().await;
}
```

Also test exact-limit success, declared length > limit rejected before allocation, truncated body failure, ordered frames, queue `Full`, and peer close -> `Closed`.

- [ ] **Step 2: Verify RED**

```bash
cargo test -p crosslab-m9-networking --test control
```

- [ ] **Step 3: Implement private u32-BE framing and owned runtime**

Record layout is `[4-byte big-endian length][body]`. Validate declared length before allocation, then `read_exact` only the validated body. `TaskRegistry::close_and_take()` stops accepting new child tasks before returning handles to join.

Add a test asserting every `CandidateConfig::default()` semantic limit equals `crosslab_transport_quic::QuicTransportConfig::default()`.

- [ ] **Step 4: Implement bounded writer/reader/close tasks**

Use bounded Tokio mpsc queues sized by `CandidateConfig`. The synchronous bridge returns existing core `Full`, `TooLarge`, and `Closed` errors without blocking.

- [ ] **Step 5: Full verification and commit**

```bash
cargo test -p crosslab-m9-networking --test control
cargo fmt --all -- --check
cargo check --workspace --all-targets --all-features
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-features
git add experiments/m9-networking
git commit -m "test(networking): add bounded Iroh control bridge"
```

---

### Task 4: Bounded uni-stream bridge and experiment-only TransportConnection

**Files:**
- Create: `src/candidate/{stream.rs,connection.rs}`
- Modify: `src/candidate/mod.rs`
- Create: `tests/stream.rs`

**Interfaces:**

```rust
pub(crate) struct IrohTransportConnection { /* iroh connection + bridges/runtime */ }
impl TransportConnection for IrohTransportConnection { /* existing semantic seam */ }
```

- [ ] **Step 1: Write RED stream tests**

Cover ordered opening/chunks, exact limits, stream-slot saturation, chunk queue saturation, sender reset/drop -> `Cancelled`, receiver stop/cancel -> sender `Closed`, clean FIN -> `Finished`, connection close cancelling active streams, and joined shutdown.

- [ ] **Step 2: Verify RED**

```bash
cargo test -p crosslab-m9-networking --test stream
```

- [ ] **Step 3: Implement stream drivers**

Reserve outgoing slots synchronously with a semaphore permit before returning a send handle. Validate inbound opening frames before exposing `IncomingUniStream`. Keep per-stream chunk queues bounded. Map FIN/reset/stop/connection-loss to existing Cross-Lab stream outcomes; do not add Iroh-specific domain errors.

- [ ] **Step 4: Implement the semantic connection surface**

```rust
impl TransportConnection for IrohTransportConnection {
    fn security_class(&self) -> TransportSecurityClass {
        TransportSecurityClass::AuthenticatedConfidentialChannel
    }
    // existing channel_binding/metadata/control/uni-stream/close/is_closed methods only
}
```

No Iroh type appears in a production/domain public API.

- [ ] **Step 5: Verify and commit**

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

### Task 5: Existing Cross-Lab session authentication over Iroh

**Files:**
- Create: `src/scenarios/{mod.rs,auth.rs}`
- Create: `tests/session.rs`
- Modify: `docs/development/CURRENT.md`

**Interfaces:**

```rust
pub(crate) struct AuthFixture { /* deterministic Cross-Lab owner/device credentials */ }
pub(crate) enum AuthAttempt { Normal, WrongBinding, ReplayInitiator(ReplayProof) }
pub(crate) struct AuthenticatedIrohPair {
    pub(crate) client_transport: IrohTransportConnection,
    pub(crate) server_transport: IrohTransportConnection,
    pub(crate) client_session: LogicalSession,
    pub(crate) server_session: LogicalSession,
    pub(crate) network_class: NetworkClass,
}
pub(crate) async fn authenticate_direct_pair(
    fixture: &AuthFixture,
    attempt: AuthAttempt,
) -> Result<AuthenticatedIrohPair, RejectedAuthentication>;
```

- [ ] **Step 1: Write RED tests**

```rust
#[tokio::test]
async fn trusted_peers_activate_over_iroh_exporter() { /* assert both Active */ }
#[tokio::test]
async fn wrong_connection_binding_fails_before_active() { /* assert Closed */ }
#[tokio::test]
async fn replay_after_reconnect_fails() { /* old proof rejected */ }
#[tokio::test]
async fn promotion_is_refused_before_both_sessions_are_active() { /* fail closed */ }
```

- [ ] **Step 2: Verify RED**

```bash
cargo test -p crosslab-m9-networking --test session
```

- [ ] **Step 3: Implement the existing hello/proof bootstrap unchanged**

Use `OwnerRootRecord`, `AuthorityDelegation`, `DeviceCredential`, `TrustRecord`, `SessionAuthTranscriptV1`, `SessionActivation`, `SessionAuthHello`, `SessionAuthProofMessage`, `encode_session_auth_bootstrap`, and `decode_session_auth_bootstrap`. Use deterministic test keys/IDs only. Never treat Iroh endpoint identity as a Cross-Lab credential.

Sequence:

```text
full Iroh handshake -> ADR-0008 exporter -> reserved bi stream
-> existing hello/proof exchange -> both LogicalSession::Active
-> same bi stream promoted into IrohTransportConnection
```

Set `AuthenticatedIrohPair::network_class` to `NetworkClass::Remote` at construction and expose no setter.

- [ ] **Step 4: Full gate, CURRENT checkpoint, commit**

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

### Task 6: Direct-Iroh control/data/reconnect/revocation semantics

**Files:**
- Create: `src/scenarios/lifecycle.rs`
- Create: `tests/lifecycle.rs`

**Interfaces:** consumes `AuthenticatedIrohPair` and existing `SimNode`, `SimStreamRuntime`, policy/trust/operation APIs. Produces no new domain API.

- [ ] **Step 1: Write RED lifecycle tests**

Cover capability advertisement + request/response/event; authorized operation-bound uni stream; unknown operation rejection; fresh reconnect changing binding and `SessionId`; old proof/session/operation authority rejection; signed revocation terminating authority; reconnect-after-revocation denial; saturation/cancellation/shutdown.

Add the LocalOnly regression using real existing policy types:

```rust
let rule = PolicyRule::new(rule_id, source, capability.clone(), operation.clone(), RuleEffect::Allow)
    .with_constraint(Constraint::LocalOnly);
let mut policy = PolicyState::new();
policy.insert(rule).unwrap();
let context = AuthorizationContext::new(
    source, destination, session_id, capability, version, operation,
    TrustState::Trusted, trust_revision, local_capability, NetworkClass::Remote,
);
assert_eq!(policy.evaluate(&context).reason(), DecisionReason::ConstraintFailed);
```

- [ ] **Step 2: Verify RED**

```bash
cargo test -p crosslab-m9-networking --test lifecycle
```

- [ ] **Step 3: Implement only test orchestration**

`SimNode` and `SimStreamRuntime` remain behavior owners. Do not add Iroh-specific policy exceptions or operation state.

- [ ] **Step 4: Verify and commit**

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

### Task 7: Owner-controlled relay and path-change invariants

**Files:**
- Modify: `experiments/m9-networking/Cargo.toml`
- Create: `src/relay.rs`
- Create: `src/scenarios/relay.rs`
- Create: `tests/relay.rs`
- Modify: `docs/development/CURRENT.md`

**Interfaces:**

```rust
pub(crate) struct OwnerRelay { /* iroh-relay Server + RelayUrl */ }
impl OwnerRelay {
    pub(crate) async fn start() -> Result<Self, EvalError>;
    pub(crate) fn url(&self) -> RelayUrl;
    pub(crate) async fn shutdown(self) -> Result<(), EvalError>;
}

pub(crate) struct RelayObservedPair { /* authenticated pair + immutable Remote class */ }
impl RelayObservedPair {
    pub(crate) fn network_class(&self) -> NetworkClass;
    pub(crate) fn channel_binding(&self) -> &ChannelBinding;
    pub(crate) fn session_id(&self) -> SessionId;
    pub(crate) async fn wait_for_direct_path(&mut self, timeout: Duration) -> Result<(), EvalError>;
}
```

- [ ] **Step 1: Add the self-hosted relay server dependency only to the experiment**

```toml
iroh-relay = { version = "=1.2.0", default-features = false, features = ["server", "test-utils", "tls-ring"] }
```

- [ ] **Step 2: Write RED tests**

```rust
#[tokio::test]
async fn relay_only_pair_carries_authenticated_control_and_data() { /* owner relay only */ }

#[tokio::test]
async fn relay_to_direct_keeps_remote_class_binding_and_session() {
    let mut pair = relay_then_direct_pair().await.unwrap();
    let binding = pair.channel_binding().bytes().to_vec();
    let session_id = pair.session_id();
    assert_eq!(pair.network_class(), NetworkClass::Remote);
    pair.wait_for_direct_path(Duration::from_secs(10)).await.unwrap();
    assert_eq!(pair.network_class(), NetworkClass::Remote);
    assert_eq!(binding, pair.channel_binding().bytes());
    assert_eq!(session_id, pair.session_id());
    pair.shutdown().await;
}
```

- [ ] **Step 3: Verify RED**

```bash
cargo test -p crosslab-m9-networking --test relay
```

- [ ] **Step 4: Implement owned local relay lifecycle**

Start `iroh_relay::server::Server` with `ServerConfig { relay: Some(RelayConfig::new(bind_addr)), ..Default::default() }`, bind port 0, use `http_url()` from test-utils, and await `Server::shutdown` during fixture shutdown.

Relay-only endpoints use:

```rust
Endpoint::builder(presets::Minimal)
    .clear_ip_transports()
    .relay_mode(RelayMode::Custom(relay_map.clone()))
    .alpns(vec![M9_ALPN.to_vec()])
```

Relay-then-direct keeps IP transports enabled and initially dials `EndpointAddr::new(server.id()).with_relay_url(relay_url)`. Observe `paths_stream()`/`path_events()` until an IP path exists. A lagged path-event consumer reloads current path state; it does not alter policy/session authority.

- [ ] **Step 5: Verify full semantic gate**

```bash
cargo test -p crosslab-m9-networking --test relay --test session --test lifecycle
cargo fmt --all -- --check
cargo check --workspace --all-targets --all-features
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-features
```

If owner-controlled relay operation requires public n0 infrastructure, record the failure and do not switch required tests to public services.

- [ ] **Step 6: CURRENT checkpoint and commit**

```bash
git add experiments/m9-networking Cargo.lock docs/development/CURRENT.md
git commit -m "test(networking): validate owner relay and Iroh path changes"
```

---

### Task 8: Reproducible benchmarks and controlled Linux NAT/relay gate

**Files:**
- Modify: `src/{metrics.rs,baseline.rs}`
- Create: `src/netprobe.rs`
- Create: `src/bin/m9-networking.rs`
- Create: `scripts/netns.sh`
- Create: `docs/research/M9-networking-evidence.md`
- Modify: `tests/{metrics.rs,baseline.rs,relay.rs}`

**Interfaces:**

```rust
pub enum Command {
    LocalQuinn(EvalConfig),
    LocalIrohDirect(EvalConfig),
    LocalIrohRelay(EvalConfig),
    LocalAll(EvalConfig),
    NetprobeRelay(NetprobeRelayArgs),
    NetprobeServer(NetprobePeerArgs),
    NetprobeClient(NetprobePeerArgs),
}
impl Command { pub fn parse<I, S>(args: I) -> Result<Self, EvalError>; }
```

- [ ] **Step 1: Write RED CLI/report tests**

```rust
#[test]
fn local_command_parses_typed_sample_and_payload_limits() {
    let command = Command::parse(["local-all", "--samples", "3", "--payload-bytes", "1048576"]).unwrap();
    assert_eq!(command.eval_config().unwrap().samples(), 3);
    assert_eq!(command.eval_config().unwrap().bulk_payload_bytes(), 1_048_576);
}
```

Report headers must record OS/arch, repository Rust version, sample count, payload bytes, and dependency labels `quinn=0.11.11`, `iroh=1.2.0`.

- [ ] **Step 2: Verify RED**

```bash
cargo test -p crosslab-m9-networking --test metrics --test baseline --test relay
```

- [ ] **Step 3: Implement local benchmark modes**

`local-quinn`, `local-iroh-direct`, `local-iroh-relay`, and `local-all` record protected connect, Cross-Lab auth, control RTT, fixed-size uni throughput, and shutdown. On Linux, read RSS from `/proc/self/status` and FD count from `/proc/self/fd`; on unsupported OSes report no value rather than adding unsafe system calls.

- [ ] **Step 4: Implement safe cross-process rendezvous**

The rendezvous file contains only:

```text
endpoint_id=public routing identifier
relay_url=http://relay-host:port or relay_url=-
ip=socket-address or ip=-
```

Reconstruct with `EndpointAddr::new(id)` plus `with_relay_url` / `with_ip_addr`. Never write private keys, relay tokens, Cross-Lab credentials/proofs, exporter bytes, or payloads.

- [ ] **Step 5: Implement the Linux namespace topology script**

The script uses fixed names `cl-m9-ra`, `cl-m9-rb`, `cl-m9-a`, `cl-m9-b`, bridge `cl-m9-br`, transit `172.30.90.0/24`, peer A `10.90.1.0/24`, peer B `10.90.2.0/24`. It must refuse to run if a planned namespace/bridge already exists.

Core setup commands are explicit:

```bash
ip netns add cl-m9-ra
ip netns add cl-m9-rb
ip netns add cl-m9-a
ip netns add cl-m9-b
ip link add cl-m9-br type bridge
ip addr add 172.30.90.1/24 dev cl-m9-br
ip link set cl-m9-br up
```

Create router external veths into `cl-m9-br`, router internal veths to each peer namespace, assign `172.30.90.2/24` and `172.30.90.3/24` externally plus `10.90.1.1/24` and `10.90.2.1/24` internally, set peer default routes, and enable forwarding only inside router namespaces.

Each router gets an nftables NAT table equivalent to:

```nft
table ip cl_m9_nat {
  chain postrouting {
    type nat hook postrouting priority srcnat;
    oifname "ext0" masquerade
  }
}
```

Run the relay on `172.30.90.1`; first block router-to-router direct UDP so relay fallback is mandatory, then allow direct UDP and observe the same live Iroh connection add/select an IP path. `trap cleanup EXIT INT TERM` kills child processes and deletes nft tables, namespaces, veths, bridge, and rendezvous files.

- [ ] **Step 6: Deterministic full gate**

```bash
cargo test -p crosslab-m9-networking
cargo fmt --all -- --check
cargo check --workspace --all-targets --all-features
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-features
```

- [ ] **Step 7: Same-host measurements**

```bash
cargo run -p crosslab-m9-networking --bin m9-networking -- local-all --samples 10 --payload-bytes 4194304 > /tmp/m9-local.tsv
/usr/bin/time -v cargo run -p crosslab-m9-networking --bin m9-networking -- local-all --samples 10 --payload-bytes 4194304 > /tmp/m9-local-time.tsv 2> /tmp/m9-local-time.txt
```

Record CPU model, RAM, kernel, Rust version, VM/container state, and load conditions.

- [ ] **Step 8: Controlled NAT/relay gate**

```bash
cargo build -p crosslab-m9-networking --bin m9-networking
sudo experiments/m9-networking/scripts/netns.sh target/debug/m9-networking /tmp/m9-netns.tsv
```

Evidence must distinguish: relay fallback success; authenticated control/data over relay; direct-path appearance after UDP is enabled; unchanged Remote classification/binding/session through path change; fresh binding/session after forced reconnect; any hole-punch failure with preserved logs.

- [ ] **Step 9: Write the evidence report**

`docs/research/M9-networking-evidence.md` sections: Environment; Dependency/feature tree; Security/owner-control eligibility; Quinn baseline; Iroh direct; Iroh relay; Controlled NAT; Recovery/path changes; Resource observations; Mobile/platform obligations; Failures/anomalies; Decision matrix; `Libp2p trigger: yes` or `Libp2p trigger: no` with a concrete reason.

- [ ] **Step 10: Commit evidence checkpoint**

```bash
git add experiments/m9-networking docs/research/M9-networking-evidence.md Cargo.lock
git commit -m "test(networking): measure M9 remote connectivity candidates"
```

---

### Task 9: Conditional rust-libp2p probe

**Trigger:** run this task only when the evidence report records an Iroh decision-criterion failure and explains why Relay v2/DCUtR/AutoNAT plausibly addresses that exact failure. Otherwise record `Libp2p trigger: no` and skip the dependency entirely.

**Files if triggered:**
- Modify: `experiments/m9-networking/Cargo.toml`
- Create: `src/libp2p_candidate.rs`
- Create: `tests/libp2p_candidate.rs`
- Modify: `docs/research/M9-networking-evidence.md`

- [ ] **Step 1: Confirm the written trigger before changing dependencies**

The report must name the failed criterion, the observed Iroh evidence, and the specific libp2p capability expected to address it.

- [ ] **Step 2: Add only NAT/relay features**

```toml
libp2p = { version = "=0.57.0", default-features = false, features = ["autonat", "dcutr", "identify", "macros", "quic", "relay", "tokio"] }
```

- [ ] **Step 3: Write and verify one focused RED probe**

If the trigger concerns relay/direct upgrade, the probe tests Circuit Relay v2 establishment plus DCUtR direct-upgrade observability in the same controlled topology. It does not implement a second Cross-Lab adapter.

```bash
cargo test -p crosslab-m9-networking --test libp2p_candidate
```

- [ ] **Step 4: Enforce the channel-binding eligibility rule**

The uploaded libp2p `0.57.0` QUIC wrapper stores its `quinn::Connection` privately. Unless execution finds a reviewed public cryptographic channel-binding API, the libp2p probe is not eligible for Cross-Lab session promotion. Never substitute `PeerId`, Noise identity, Multiaddr, or connection IDs for ADR-0008.

- [ ] **Step 5: Audit, verify, report, commit**

```bash
cargo tree -p crosslab-m9-networking -e features
cargo fmt --all -- --check
cargo check --workspace --all-targets --all-features
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-features
git add experiments/m9-networking docs/research/M9-networking-evidence.md Cargo.lock
git commit -m "test(networking): compare libp2p for M9 decision gap"
```

---

### Task 10: ADR-0009, architecture reconciliation, merge, and M10 handoff

**Files:**
- Create: `docs/adr/ADR-0009-remote-networking.md`
- Modify: `docs/adr/README.md`
- Modify: `docs/architecture/MASTER-ARCHITECTURE.md`
- Modify: `docs/plans/phase-1/M9-remote-networking-design.md`
- Modify: `docs/development/CURRENT.md`
- Retain/remove experiment code only as the reviewed ADR directs; do not auto-promote it into `transports/`.

- [ ] **Step 1: Decide from the evidence matrix only**

Iroh is eligible only when exact exporter semantics, existing Cross-Lab auth/control/data/reconnect/revocation, owner-controlled relay, Remote classification invariant, controlled NAT/recovery evidence, bounded lifecycle, and acceptable platform/maintenance cost all pass. If no candidate passes, ADR-0009 records no selection.

- [ ] **Step 2: Write the Proposed ADR**

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

If Iroh passes, state that Quinn remains local/LAN; Iroh is the remote/NAT/relay architecture; EndpointId remains transport-only; `quic-tls-exporter-v1` is reused exactly after full handshake; Iroh sessions remain Remote; self-hosted relays are supported; no public service is mandatory; M10 still owns real Android/mobile lifecycle validation.

- [ ] **Step 3: Stop for ADR review before normative status changes**

Do not mark ADR-0009 Accepted or edit Master Architecture candidate/selected status until the ADR/evidence is reviewed and approved.

- [ ] **Step 4: After approval, reconcile docs**

Index ADR-0009, update Master Architecture networking status/unresolved-decision table, mark the M9 design implemented/decision-complete, and keep intentional non-goals unchanged.

- [ ] **Step 5: Write durable M10 handoff**

CURRENT.md records exact M9 evidence head, CI, manual namespace gate status/environment, ADR decision, libp2p trigger status, dependency-retention decision, and the exact M10 Linux + Android first-platform-vertical-slice task plus mobile validation obligations.

- [ ] **Step 6: Exact final gate**

```bash
cargo metadata --locked --format-version 1 >/dev/null
cargo fmt --all -- --check
cargo check --workspace --all-targets --all-features
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-features
cargo test -p crosslab-m9-networking
```

The controlled NAT gate must already be recorded; loopback does not replace it.

- [ ] **Step 7: Commit, update PR #19, merge exact verified head, verify main**

```bash
git add docs experiments/m9-networking Cargo.toml Cargo.lock
git commit -m "docs: complete M9 remote networking decision"
```

Update PR #19 with the evidence/decision and exact head SHA. Mark ready only after full verification. Merge with expected-head guard, confirm `main`, require push CI to pass the same gate, then update/verify CURRENT.md on `main` if a canonical integration checkpoint is needed.

---

## Self-Review Mapping

- Spec §§7–9 -> Tasks 2 and 5.
- Spec §§10–11 -> Tasks 6 and 7.
- Spec §§12–13 -> Tasks 3 and 4.
- Spec §§14–15 -> Tasks 6–8.
- Spec §§16–17 -> Tasks 2, 7–10.
- Spec §§18–20 -> Tasks 9–10.
- No production Iroh/libp2p dependency is introduced before ADR acceptance.
- No task changes Cross-Lab identity, policy, session transcript, or `TransportConnection` signatures.
- The plan contains no unresolved implementation placeholder; conditional Task 9 is governed by a concrete evidence trigger.
