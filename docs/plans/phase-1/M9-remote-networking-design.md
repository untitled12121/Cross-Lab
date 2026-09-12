# M9 Remote Networking ADR — Design

**Status:** Proposed; architecture direction approved, written spec pending review  
**Milestone:** Phase 1 / M9 — Remote Networking ADR  
**Baseline:** verified canonical `main` checkpoint `ab602d62181b3dcba056874d0013e40522a68e8f`  
**Primary contracts:** `docs/architecture/MASTER-ARCHITECTURE.md`, `docs/architecture/SESSION-TRANSPORT.md`, `docs/architecture/CORE-SIMULATOR.md`, ADR-0008, and the verified M8 Quinn transport

## 1. Goal

M9 decides how Cross-Lab reaches trusted devices across NATs and the public Internet without replacing the identity, trust, policy, logical-session, control, or authorized-stream architecture proven through M8.

The milestone is an evidence-driven networking decision, not a broad P2P-framework adoption. It must compare a remote-connectivity candidate against the verified Quinn baseline, prove that Cross-Lab security semantics survive direct/relayed connectivity, measure the operational trade-offs, and record the selected architecture through an ADR.

The preferred architecture to validate is:

```text
Cross-Lab identity / trust / policy
             |
      LogicalSession
             |
 control + authorized data semantics
             |
     TransportConnection
        /            \
 local/LAN        remote/Internet
 Quinn M8          Iroh candidate
                      |
              direct path or relay
```

Quinn remains the established local/LAN baseline. Iroh is evaluated as a separate remote-connectivity adapter providing NAT traversal, relay fallback, and path discovery. rust-libp2p remains the comparison alternative and is prototyped only if Iroh fails a concrete M9 requirement or a focused libp2p spike is needed to resolve a material uncertainty.

M9 succeeds only if the chosen remote architecture preserves Maximum Owner Control + Least-Privilege Architecture, requires no mandatory Cross-Lab-operated public service, keeps transport identity independent from Cross-Lab device identity, and maintains the same fresh session-authentication and authorization rules already proven over Quinn.

## 2. Governing Architectural Invariants

M9 must preserve these existing rules:

- Applications use a Cross-Lab `LogicalSession`, never an Iroh `Endpoint`, libp2p `Swarm`, `PeerId`, `Multiaddr`, relay account, or socket directly.
- `DeviceId`, owner trust, credentials, revocation, capability negotiation, policy, `SessionId`, operation authorization, and stream admission remain Cross-Lab domain state.
- Iroh `EndpointId`, Iroh secret keys, libp2p `PeerId`, relay URLs/tokens, path IDs, and transport addresses are routing/transport state only.
- A protected transport must provide an opaque cryptographic channel binding derived from the concrete established connection.
- A new protected connection always requires fresh Cross-Lab session authentication. Old proofs, sequence state, operation grants, and stream authority do not carry across reconnect.
- Security requirements are eligibility constraints, not route-scoring hints. A faster path may never weaken a `LocalOnly` or equivalent policy rule.
- No transport library type enters `crosslab-core`, `crosslab-protocol`, `crosslab-policy`, or `crosslab-identity` public/domain state.
- No 0-RTT/early-data path is allowed to carry Cross-Lab authority.
- Queues, stream counts, record sizes, relay buffers, task ownership, retries, and shutdown remain bounded.
- No mandatory vendor cloud, Cross-Lab public account, or Cross-Lab-operated relay is introduced.

## 3. Verified Baseline

M8 provides the comparison and compatibility baseline:

- `crosslab-transport-quic` implements the existing narrow `TransportConnection` seam over Quinn `0.11.11`;
- ADR-0008 defines `quic-tls-exporter-v1`, a 32-byte TLS-exporter binding included in the existing Cross-Lab session-auth transcript;
- existing `LogicalSession`, `SimNode`, and `SimStreamRuntime` semantics are proven over a real encrypted transport;
- reconnect creates a fresh exporter binding, nonces/proofs, `SessionId`, sequences, capability state, and operation authority;
- transport loss, cancellation, saturation, revocation, and shutdown fail closed;
- the current policy layer distinguishes `NetworkClass::{Local, Trusted, Remote}` and enforces `LocalOnly` as a hard constraint.

M9 therefore does not redesign the session or control/data protocol. It evaluates only the remote connection substrate and the minimum adapter/orchestration needed to prove compatibility.

## 4. Research Evidence

### 4.1 Iroh

The uploaded Iroh source tree is package version `1.2.0`, Rust 2024, `rust-version = 1.91`, licensed `MIT OR Apache-2.0`. Current package metadata was also verified on 2026-09-12: `iroh 1.2.0` is the latest published release.

Relevant source evidence:

- `iroh::endpoint::Connection::export_keying_material` derives TLS-session exporter bytes after connection establishment;
- `Connection::paths`, `paths_stream`, and `path_events` expose open paths and selected-path changes without requiring Cross-Lab to inspect private socket internals;
- an Iroh connection can maintain a relay path and later add/select a direct path after hole punching;
- `RelayMode::Disabled`, `RelayMode::Default`, and `RelayMode::Custom(RelayMap)` allow Cross-Lab to avoid mandatory n0 relays and to use owner-selected/self-hosted relays;
- `endpoint::presets::Minimal` configures only mandatory crypto-provider state, while the `N0` preset additionally enables n0 DNS/address lookup and default n0 relays;
- `iroh-relay` contains the relay client/server, supports running a relay server, and provides access-control modes including endpoint allow/deny lists, local shared tokens, and HTTP callout;
- the Iroh endpoint identity is included in the transport handshake and is intentionally a transport endpoint identity, not a Cross-Lab identity.

M9 required tests therefore use `presets::Minimal` plus explicit transport configuration. The `N0` preset is not used in mandatory CI or required product behavior. The experiment should start with Iroh default features disabled and enable only the reviewed crypto/NAT features it actually exercises (initially `tls-ring` and `portmapper`); relay-server test support is isolated to the experiment/test dependency surface rather than production crates.

### 4.2 rust-libp2p

The uploaded rust-libp2p tree is `libp2p 0.57.0`, Rust 2024, workspace `rust-version = 1.88.0`, licensed MIT. Current package metadata was verified on 2026-09-12: `libp2p 0.57.0` is the latest published release.

The tree provides mature components for Circuit Relay v2, DCUtR direct-connection upgrade, AutoNAT v1/v2, Swarm orchestration, connection limits, and multiple transport/multiplexer choices.

However, the current `libp2p-quic` connection wrapper stores the underlying `quinn::Connection` privately and does not expose a TLS-exporter API. Its normal public architecture also brings `PeerId`, `Multiaddr`, `Swarm`, and `NetworkBehaviour` concepts that Cross-Lab deliberately keeps outside its domain model.

This does not make libp2p unsuitable in general. It makes it a higher-integration-cost M9 alternative because Cross-Lab would need an independently reviewed way to satisfy its existing channel-binding contract without treating `PeerId`, Noise identity, or addresses as a replacement for Cross-Lab authentication.

### 4.3 Dependency/runtime compatibility

Both candidates support the repository's Rust `1.98.1` baseline. Iroh's Tokio requirement is semver-compatible with the workspace's Tokio `1.53.1`, and its rustls `0.23.x` requirement is compatible with the existing M8 rustls family. M9 must still inspect the resolved tree before accepting any candidate and must reject accidental duplicate async runtimes or unnecessary default features where feasible.

## 5. Architectural Options

### Option A — Quinn local + Iroh remote

Keep the proven Quinn adapter as the explicit local/LAN transport and add an isolated Iroh candidate for remote connectivity.

Advantages:

- focused NAT traversal and relay capability without adopting a universal P2P domain model;
- direct TLS exporter API fits the existing channel-binding architecture;
- explicit custom/self-hosted relay configuration supports owner control;
- path events provide the observability needed for benchmarks and future diagnostics;
- Iroh remains beneath `TransportConnection`, so Cross-Lab identity/policy/session semantics remain authoritative.

Costs:

- two concrete networking adapters must be maintained;
- Iroh has its own endpoint key/routing identity that requires strict isolation from `DeviceId`;
- endpoint/address discovery and relay operations still need explicit Cross-Lab product decisions later;
- internal relay/direct path migration adds lifecycle cases that M8 did not need.

**Recommendation:** prototype this first and select it if the success criteria in this design pass.

### Option B — Quinn plus Cross-Lab-owned NAT traversal/relay

Retain Quinn and build hole punching, reachability detection, relay protocol/server, path discovery, migration, relay authorization, and operational tooling in Cross-Lab.

Advantages:

- maximum protocol and infrastructure control;
- no second high-level networking framework.

Costs:

- much larger security-sensitive and operations surface;
- substantial NAT/network edge-case burden across desktop and mobile platforms;
- duplicates mature work that is not a differentiating Cross-Lab capability;
- delays M10 platform work.

**Decision for M9:** reject unless both Iroh and focused libp2p evaluation fail material owner-control/security requirements.

### Option C — rust-libp2p remote fabric

Use rust-libp2p relay/DCUtR/AutoNAT with a custom Cross-Lab protocol behavior.

Advantages:

- standardized and modular P2P components;
- broad ecosystem and explicit relay/hole-punching primitives;
- potentially useful if Cross-Lab later needs broader protocol interoperability.

Costs:

- significantly broader orchestration/model surface than currently required;
- public QUIC wrapper does not expose the existing TLS-exporter seam;
- easy to accidentally let `PeerId`, `Multiaddr`, or Swarm lifecycle leak into Cross-Lab domain state;
- likely more code/configuration for the narrow M9 goal.

**Decision for M9:** comparison alternative. Do not add a production libp2p dependency unless Iroh evidence exposes a concrete gap that libp2p can solve cleanly.

## 6. Proposed M9 Boundary

The evaluation code lives in an isolated research/experiment package rather than in production identity/session crates.

Proposed shape:

```text
experiments/
└── m9-networking/
    ├── Cargo.toml
    └── src/
        ├── main.rs or lib.rs
        ├── baseline.rs       # Quinn measurement harness
        ├── iroh_candidate.rs # Iroh candidate only
        ├── relay.rs          # test/self-hosted relay fixture
        ├── scenarios.rs      # common semantic scenarios
        └── metrics.rs        # stable measurement output
```

The package is `publish = false` and may depend on candidate networking crates without making them dependencies of `crosslab-core`, protocol, identity, policy, or the production Quinn adapter.

Do not extract a generic transport-framework crate before the prototype. If Iroh is selected and substantial implementation-neutral duplication is proven, the M9 ADR may recommend a later focused extraction. The prototype itself should prefer a little local duplication over prematurely refactoring the already verified M8 adapter.

If the final ADR rejects Iroh, the experiment package may be removed after preserving benchmark results and decision evidence in docs. If it selects Iroh, only the minimum reusable adapter code justified by M10 should graduate into a production `transports/` crate; that graduation is not automatic merely because the prototype passes.

## 7. Iroh Endpoint and Infrastructure Configuration

Mandatory M9 behavior uses `Endpoint::builder(presets::Minimal)` and explicit configuration.

Required modes:

```text
Direct-only evaluation:
  Minimal preset + relay disabled + explicit EndpointAddr/direct addresses

Owner-relay evaluation:
  Minimal preset + RelayMode::Custom(owner relay map)
  + explicit EndpointAddr relay/direct hints

Optional public-infrastructure observation:
  explicit opt-in only; never a CI or product requirement
```

The prototype must not silently use the `N0` preset, n0 DNS publication/resolution, or default n0 relay map. Any optional public-infrastructure experiment must be separately labeled and may not become required for tests or normal operation.

The self-hosted relay fixture uses Iroh's relay server implementation rather than inventing a Cross-Lab relay protocol. Required automated tests use a local/in-process or locally launched relay with explicit lifecycle ownership. Relay tasks/processes must be joined/stopped by the harness.

## 8. Identity and Key Separation

Iroh transport identity is not Cross-Lab device identity.

The adapter/harness must maintain the following separation:

```text
Cross-Lab DeviceId / device signing credential
        !=
Iroh EndpointId / Iroh SecretKey
        !=
relay access credential
```

For M9 tests, Iroh secret keys may be ephemeral and fixture-owned. M9 does not define production persistence or rotation for Iroh transport keys.

A future production integration may persist a dedicated Iroh transport key if stable remote routing requires it, but that key must remain separate from Cross-Lab signing/recovery keys and must be protected by platform storage appropriate to its routing/privacy role. That persistence decision belongs to the consuming platform/agent design.

Relay allowlists or shared tokens authorize use of relay infrastructure only. They never grant Cross-Lab trust, session activation, capability permission, or operation authority.

Endpoint IDs and relay credentials must not be logged together with sensitive Cross-Lab credentials or channel-binding bytes.

## 9. Channel Binding and Session Authentication

The Iroh candidate must prove compatibility with the existing Cross-Lab session-auth design instead of introducing an Iroh-specific identity transcript.

After the full Iroh QUIC/TLS handshake completes, derive exactly the ADR-0008 exporter profile:

```text
profile_id = "quic-tls-exporter-v1"
output_len = 32 bytes
label      = "EXPORTER-Cross-Lab-QUIC-Channel-Binding-v1"
context    = "crosslab.quic.transport.v1"
```

Required evidence:

- both peers derive identical bytes for the same fully established Iroh connection;
- a fresh Iroh connection derives different binding bytes;
- the existing Cross-Lab hello/proof flow reaches `Active` using that binding;
- a proof from another connection or previous reconnect is rejected;
- exporter bytes remain stable while the same Iroh connection changes its selected network path from relay to direct;
- no ordinary Cross-Lab control/data authority is accepted before session authentication completes.

M9 does not use Iroh's 0-RTT APIs for authority. The candidate becomes eligible for Cross-Lab authentication only after normal handshake completion and exporter availability.

If Iroh cannot reproduce ADR-0008 semantics exactly, the candidate fails this criterion. M9 must then stop and explicitly design a new reviewed channel-binding profile/ADR rather than silently weakening or substituting the binding. If Iroh is selected and does reproduce the profile exactly, the M9 ADR will explicitly record that `quic-tls-exporter-v1` is being reused by a second QUIC/TLS implementation; the stack name in ADR-0008's title does not grant Quinn-specific identity authority.

## 10. Remote Network Classification

This is a security invariant for the recommended architecture:

**Any Cross-Lab session established through the Iroh remote adapter is `NetworkClass::Remote` for its entire lifetime, regardless of whether Iroh currently sends through a relay or a direct path.**

Reason:

- Iroh may begin relayed and later select a direct path without creating a new Cross-Lab session;
- the direct path may be LAN, same-city Internet, CGNAT-assisted, or another network topology;
- changing policy classification based on a transport's current selected path could silently turn an already-established remote session into a `Local` one and bypass `LocalOnly` constraints.

Therefore path type is observability/performance metadata, not policy authority.

A future route classifier may establish a separate Quinn/local session when locality is independently proven. M9 does not reclassify an active Iroh session or migrate authorization between transports.

## 11. Path Changes, Reconnect, and Authority

Iroh may change the network path beneath one established protected connection. That is different from reconnecting the Cross-Lab transport connection.

Rules:

- relay -> direct or direct -> relay inside the same live Iroh connection keeps the same `TransportConnection`, exporter binding, Cross-Lab `SessionId`, sequence space, and active operation authority;
- path events may update metrics/diagnostics only;
- a connection close followed by a new Iroh connection is a normal Cross-Lab reconnect and requires fresh exporter binding, nonces/proofs, `SessionId`, capability state, and authorization;
- no operation grant or stream admission from the old connection survives reconnect;
- if relay/path availability changes but the protected connection stays alive, Cross-Lab must not synthesize a reconnect unnecessarily.

M9 does not add seamless migration between Quinn and Iroh transports. Moving from local Quinn to remote Iroh or vice versa is a disconnect/new-session sequence in Phase 1.

## 12. Control and Data Mapping

The Iroh candidate uses an experiment-only ALPN such as `crosslab-m9-networking-eval`; that value is not a production protocol commitment. A production remote ALPN is frozen only if the M9 ADR or a later consuming milestone requires it.

The Iroh candidate must prove the same semantic roles as M8:

- one reserved reliable bidirectional stream for Cross-Lab session bootstrap followed by ordinary control traffic;
- one transport unidirectional stream per authorized Cross-Lab data stream;
- bounded private length framing where the candidate API exposes byte streams rather than framed messages;
- the existing `LogicalSession`, control dispatcher, operation authorization, stream admission, cancellation, revocation, and shutdown semantics above the adapter.

M9 does not add datagram domain APIs, new control messages, transport-specific capability identifiers, or Iroh-specific authorization paths.

The experiment may locally reuse framing constants and semantic test helpers, but it must not expose Iroh stream/address types through `TransportConnection` or domain APIs.

## 13. Bounds and Resource Ownership

The candidate must be lightweight and bounded. At minimum it must define/measure:

- control queue capacity;
- incoming/outgoing stream slots;
- per-stream chunk queue capacity;
- maximum control/opening/chunk record sizes;
- remote QUIC stream concurrency;
- receive windows where configurable;
- relay connection/task ownership;
- address/path event queue behavior;
- retry/backoff limits for evaluation orchestration;
- endpoint/connection shutdown and task joining.

The prototype should begin from M8's semantic limits where the Iroh API permits equivalent settings rather than choosing larger defaults by convenience. Differences must be recorded in the benchmark report.

No unbounded `read_to_end`, unbounded Tokio channel, detached connection task, infinite reconnect loop, or unconstrained relay retry loop is acceptable.

## 14. Evaluation Matrix

M9 uses two evidence classes: deterministic automated semantic tests and reproducible measurements/controlled-network experiments.

### 14.1 Required automated semantic tests

These must run without public infrastructure:

1. Iroh peers derive the same ADR-0008 exporter binding.
2. Fresh reconnect changes exporter binding and rejects old proofs/session authority.
3. Existing Cross-Lab session authentication reaches `Active` over Iroh.
4. Existing control request/response/event flow works without adapter-specific authorization.
5. Existing operation-bound unidirectional stream flow works with bounded backpressure.
6. Peer revocation terminates active authority and blocks reconnect activation.
7. Relay-only connection through a self-hosted/local relay carries authenticated control/data.
8. Relay/direct path changes are observable while the session remains `NetworkClass::Remote`.
9. Connection loss cancels active streams/operations and joins candidate-owned tasks.
10. Saturation, oversize frames, stream cancellation, and shutdown fail closed.

### 14.2 Reproducible networking experiments

Record environment and results rather than making unstable WAN timings hard CI assertions.

Compare Quinn baseline and Iroh candidate for:

- protected connection establishment time;
- Cross-Lab session-auth completion time;
- small control round-trip latency;
- bulk unidirectional throughput for fixed payload sizes;
- relay-only throughput/latency in a controlled local relay topology;
- relay -> direct upgrade time when a direct path becomes possible;
- connection recovery after endpoint/network reachability change;
- idle process memory/CPU and active-transfer memory/CPU;
- task/FD/socket counts where practical;
- shutdown time and evidence that owned tasks terminate.

### 14.3 NAT traversal evidence

M9 must not claim Internet/NAT success from loopback relay tests alone.

Use a reproducible Linux controlled-network topology where practical (network namespaces/containers or an equivalent isolated harness) to place peers behind separate translated networks and record:

- direct hole-punch success/failure;
- relay fallback success;
- upgrade from relay to direct when reachable;
- behavior when one side cannot accept a direct path;
- recovery after an address/interface change.

If hosted CI cannot provide the required network privileges reliably, keep deterministic semantic tests in normal CI and run the controlled NAT matrix as a documented manual/benchmark gate before accepting the ADR. Exact environment and commands must be recorded so results are repeatable.

### 14.4 Mobile-lifecycle viability

M9 occurs before the M10 Linux/Android vertical slice, so it does not pretend to validate real Android/iOS background execution policy.

It must nevertheless evaluate candidate suitability by:

- confirming supported target/build constraints against the Cross-Lab Rust baseline;
- exercising endpoint/connection recovery across socket/address changes where the desktop test environment permits it;
- confirming the adapter does not require a process-global immortal runtime or unjoinable daemon;
- documenting known platform/background limitations found in source/docs;
- identifying the exact Android/iOS lifecycle tests that M10 must run on real devices before remote networking is called production-ready.

The M9 ADR may select the architecture with explicit mobile validation obligations for M10, but it may not claim real-device mobile lifecycle verification that M9 did not perform.

## 15. Benchmark Interpretation

Performance is evaluated only after security/owner-control eligibility passes.

No single microbenchmark chooses the architecture. The decision weighs:

1. security and channel-binding compatibility;
2. owner-controlled/self-hosted operation;
3. NAT/direct/relay success and recovery behavior;
4. architectural isolation from Cross-Lab identity/policy;
5. bounded resource/lifecycle behavior;
6. platform viability;
7. latency/throughput/resource cost;
8. maintenance/dependency complexity.

Measurements must include the Quinn baseline on the same host/environment. WAN/public-relay results are observational and cannot be compared as if network conditions were controlled unless the environment actually is controlled.

## 16. Security and Privacy Review

The M9 ADR must explicitly review:

- malicious or compromised relay behavior: relay must never be able to authenticate as a Cross-Lab peer or modify encrypted session data successfully;
- replay/splicing: Cross-Lab proofs remain bound to the exact TLS exporter and fresh nonces;
- endpoint-ID confusion: `EndpointId` is not `DeviceId` and cannot grant trust;
- relay credential scope: relay token/allowlist grants infrastructure access only;
- metadata exposure: relays may observe routing endpoint identifiers, connection timing, and traffic volume even though Cross-Lab payloads remain encrypted;
- endpoint-ID linkability: a persisted Iroh transport key creates a stable routing pseudonym and requires a future rotation/privacy policy;
- address lookup privacy: mandatory M9 behavior avoids n0 address publication/resolution and uses explicit/local fixtures;
- downgrade: path or relay selection cannot lower authentication or policy requirements;
- path reclassification: an Iroh session remains `Remote` through all internal path changes;
- resource attacks: stream/queue/window/retry limits bound memory and task growth;
- logging: never log Cross-Lab private keys, proofs, exporter bytes, relay shared tokens, or sensitive payloads.

## 17. Dependency and License Rules

M9 may add candidate dependencies only to the isolated experiment package before the networking ADR is accepted.

Initial candidate baselines:

```text
iroh    = 1.2.0   # MIT OR Apache-2.0
libp2p  = 0.57.0  # MIT; comparison only unless justified
```

Before committing dependency changes:

- verify current published versions and maintenance status;
- inspect enabled feature trees and duplicate runtime/TLS stacks;
- disable unnecessary default features where doing so does not fight the candidate's supported configuration;
- run license/security checks available to the repository;
- avoid candidate dependencies in production/domain crates until the ADR decision permits them.

The uploaded source trees are research/reference material. Cross-Lab reuses public APIs and proven ideas but does not copy their architecture blindly.

## 18. Decision Rule and ADR Outcome

The M9 networking ADR selects **Quinn local + Iroh remote** if all of the following hold:

- exact ADR-0008 exporter semantics work after full Iroh handshake;
- existing Cross-Lab session/control/authorized-stream/reconnect/revocation semantics pass without an Iroh-specific identity or authorization path;
- owner-selected/self-hosted relay operation works without mandatory n0/Cross-Lab public infrastructure;
- direct/relay path behavior is observable and does not weaken network policy classification;
- NAT/relay/recovery evidence is materially better than raw Quinn without requiring Cross-Lab to build a relay/NAT stack;
- bounded resource and shutdown behavior is acceptable;
- maintenance/platform costs are acceptable relative to the capability gained.

A focused rust-libp2p prototype becomes mandatory only if Iroh fails one of those criteria and libp2p plausibly addresses the exact failure. If neither candidate passes, the ADR records no selection and M10 proceeds without pretending remote networking is solved.

The ADR must record evidence, rejected alternatives, security/privacy impact, operational implications, dependency decision, and explicit M10 follow-ups.

## 19. Intentional Non-Goals

M9 does not implement:

- a Cross-Lab cloud account or public control plane;
- a mandatory Cross-Lab relay fleet;
- production relay deployment/operations;
- production persistence/rotation of Iroh endpoint keys;
- Cross-Lab device discovery or pairing over Iroh;
- a global route-scoring/multi-transport migration engine;
- seamless session migration between Quinn and Iroh;
- TCP/TLS fallback;
- datagram/media transport;
- UI for networking diagnostics;
- Linux/Android/iOS platform-agent integration;
- background execution policy;
- privileged networking helpers;
- a generic libp2p protocol stack;
- replacement of Cross-Lab identity with Iroh/libp2p identity.

Those remain later decisions with explicit consumers.

## 20. Deliverables

M9 produces:

1. this approved design;
2. a detailed implementation/evaluation plan;
3. an isolated reproducible networking experiment harness;
4. Quinn baseline measurements;
5. Iroh candidate semantic/NAT/relay/resource measurements;
6. a focused libp2p comparison/prototype only if triggered by the decision rule;
7. a benchmark/evidence report with environment details;
8. a networking ADR selecting or rejecting the remote architecture;
9. updated Master Architecture status if the ADR changes the candidate/selected state;
10. updated `docs/development/CURRENT.md` with exact verified commits/CI and the M10 next task.

No production dependency or architecture is considered selected merely because prototype code compiles. Selection happens only through the reviewed M9 ADR after the required evidence is recorded.
