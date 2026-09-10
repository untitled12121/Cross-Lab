# Cross-Lab Phase 1 Core Simulator Specification

**Status:** Phase 0 specification  
**Milestone:** P0.9 — Core Simulator Specification  
**Architecture baseline:** `docs/architecture/MASTER-ARCHITECTURE.md`, Revision 2.0

## 1. Purpose

The Phase 1 Core Simulator proves Cross-Lab's identity, trust, authorization, protocol, logical-session, stream, reconnect, revocation, backpressure, and shutdown architecture without introducing operating-system integration or real network complexity.

The simulator is not a throwaway mock of different semantics. It composes the same domain crates and cryptographic verification code intended for later real adapters.

## 2. Initial workspace

Subject to Phase 0 acceptance of ADR-0006, the minimal Phase 1 workspace is:

```text
crosslab/
├── Cargo.toml
├── rust-toolchain.toml
├── crates/
│   ├── crypto/       # crosslab-crypto
│   ├── identity/     # crosslab-identity
│   ├── policy/       # crosslab-policy
│   ├── protocol/     # crosslab-protocol
│   └── core/         # crosslab-core
└── apps/
    └── sim/          # crosslab-sim
```

No empty future platform, transport, plugin, sync, update, desktop, Android, or iOS crates are created during the initial simulator milestone.

## 3. Dependency direction

```text
crosslab-crypto
      |
      v
crosslab-identity
      |
      v
crosslab-policy
      | \
      |  +------------------+
      v                     v
crosslab-protocol ------> crosslab-core
      ^                     |
      |                     v
      +---------------- crosslab-sim
```

More precisely:

- `crosslab-crypto` has no dependency on another Cross-Lab crate.
- `crosslab-identity` depends on `crosslab-crypto`.
- `crosslab-policy` depends on `crosslab-identity`; it may use `crosslab-crypto` only for domain-owned security objects if required and justified.
- `crosslab-protocol` depends on domain types from `crypto`, `identity`, and `policy` as needed for wire conversion; it performs no policy decisions.
- `crosslab-core` depends on `identity`, `policy`, and `protocol`, and may depend on `crypto` only for session/security orchestration primitives that are not owned by another domain.
- `crosslab-sim` is a composition root and may depend on all foundation crates.

Circular crate dependencies are forbidden.

## 4. Crate responsibilities

### 4.1 `crosslab-crypto`

Owns narrow shared cryptographic mechanics:

- profile/algorithm identifiers;
- secure random helpers/interfaces;
- Ed25519 v1 sign/verify wrapper;
- BLAKE3 digest/fingerprint helpers;
- canonical transcript v1 builder/digest;
- constant-time verification helpers;
- sensitive wrapper types required to avoid accidental logging.

It owns no identity/trust/policy/session business semantics.

### 4.2 `crosslab-identity`

Owns:

- `OwnerId`, `DeviceId`, `KeyId`;
- owner-root and delegated authority domain objects;
- device key/credential domain objects;
- credential issuance/verification;
- credential/delegation epochs;
- normal root-successor verification;
- identity-specific canonical transcript construction.

### 4.3 `crosslab-policy`

Owns:

- `TrustState`, `TrustRecord`, trust revisions;
- revocation transitions/validation required by Phase 1;
- `CapabilityId`, `CapabilityVersion`, `OperationName`;
- local capability domain records used by policy;
- policy rules/constraints/obligations;
- authorization context and pure evaluator;
- `OperationId`, `AuthorizedOperation`, operation lifecycle/invalidation.

### 4.4 `crosslab-protocol`

Owns:

- protocol version/range negotiation;
- bounded frame codecs;
- protobuf wire messages and strict domain conversions;
- session/control envelopes;
- request/response/event/cancel/error messages;
- capability advertisements;
- data-stream open header;
- pairing/session-auth wire messages;
- protocol compatibility/error codes;
- message-sequence/replay validation helpers where wire-state-specific;
- golden wire/canonical vectors and parser fuzz targets where appropriate.

It owns no sockets or authorization decisions.

### 4.5 `crosslab-core`

Owns:

- simulated/real node orchestration independent from UI/OS;
- pairing orchestration using domain/protocol components;
- logical session state machine;
- authentication/trust/capability/policy sequencing;
- control request/response/event dispatch;
- authorized operation creation/use;
- data-stream admission;
- disconnect/reconnect;
- revocation reaction;
- bounded queue/task ownership;
- cancellation/shutdown;
- narrow transport seam.

### 4.6 `crosslab-sim`

Owns only simulator composition and scenarios:

- deterministic owner/device fixtures;
- in-memory transport implementation for the core transport seam;
- deterministic/fake clock where required by local expiry tests;
- seeded deterministic non-security scenario generation where useful;
- explicit cryptographic test fixtures with fixed synthetic keys where golden vectors require exact output;
- CLI/scenario runner output;
- end-to-end simulator integration tests.

It must not contain domain rules that production adapters would need.

## 5. Determinism and cryptographic randomness

Simulator behavior should be deterministic where it improves reproducible tests, but production security APIs must not accidentally accept predictable randomness.

Rules:

- production identity/key/ID generation uses the OS CSPRNG path;
- golden tests may construct fixed synthetic IDs/keys directly through test-only fixtures;
- randomized property/scenario tests may use a seeded non-security RNG only for test-data selection, never to exercise the production key-generation API as if secure;
- deadlines use an injectable/test clock at the orchestration/policy boundary where necessary so expiry tests do not sleep in real time.

## 6. In-memory transport harness

The simulator adapter implements the P0.7 transport semantics with:

```text
MemoryTransportPair
  endpoint A
  endpoint B
  bounded ordered control queues
  bounded stream-open queues
  bounded byte chunks for simulated data streams
  per-connection ChannelBinding fixture
  disconnect/failure injection
  cancellation/close propagation
```

The transport is marked `InProcessTest`. It does not implement toy encryption or claim external-network confidentiality.

Required fault controls include:

- disconnect now;
- refuse new stream;
- saturate bounded queue;
- close one direction;
- cancel pending operation/stream setup;
- inject malformed encoded frame through parser tests rather than bypassing the protocol layer.

## 7. Simulated node

A simulated device composes:

```text
SimNode
  owner trust anchor/delegations
  local device identity/credential/key provider
  trust store
  policy state
  local capability registry
  core runtime/node state
  in-memory transport endpoint(s)
  audit sink for assertions
```

No SQLite/filesystem persistence is needed for Phase 1. Restart/persistence recovery is a later milestone.

## 8. Baseline capability fixtures

Use a deliberately small capability set sufficient to prove semantics:

```text
clipboard.read      v1.0
clipboard.write     v1.0
files.transfer      v1.0
```

The simulator does not implement a real clipboard or filesystem. Capability handlers operate on synthetic bounded payloads and observable test state.

Suggested protected operations:

```text
clipboard.read  -> get
clipboard.write -> set
files.transfer  -> send
files.transfer  -> receive
```

`files.transfer/send` is used to prove an operation-authorized data stream without building file-resume protocol in Phase 1.

## 9. Positive scenario S-001 — owner/device identity creation

Given a synthetic owner trust domain and two new simulated devices:

- create stable `OwnerId` and each `DeviceId`;
- create root + Device Signing Authority fixture;
- issue device credentials;
- verify credentials and key fingerprints;
- prove device private-key possession through the later auth handshake.

Expected: valid credentials verify and stable device IDs remain distinct from key IDs.

## 10. Positive scenario S-002 — pairing

Device A is already trusted/owner-authorizing; Device B joins.

Steps:

1. A creates single-use pairing invitation/secret.
2. B receives the secret through simulator OOB fixture.
3. A/B exchange pairing hello/nonces/keys.
4. Both reconstruct identical canonical pairing transcript.
5. Directional pairing confirmations verify.
6. A issues B's `DeviceCredential`.
7. B verifies owner root/delegation/credential.
8. B signs credential-accepted proof.
9. A verifies proof and commits `TrustRecord::Trusted`.
10. Pairing invitation becomes consumed.

Expected: A records B as trusted only after final proof; B has valid owner-domain material/credential.

## 11. Positive scenario S-003 — session authentication

After pairing:

1. establish new memory transport connection/binding;
2. exchange bounded auth hello values;
3. verify credential/owner/epoch/trust;
4. negotiate protocol 1.x;
5. build identical session-auth transcript;
6. verify initiator/responder device proofs;
7. derive same `SessionId`;
8. initialize control sequence state;
9. transition both logical sessions to Active.

Expected: both sides agree on peer identity, protocol version, session ID, and active state.

## 12. Positive scenario S-004 — capability exchange

After Active:

- exchange bounded capability advertisements;
- validate identifiers/versions/runtime flags;
- compute compatible session capability view;
- do not create policy authority from advertisement.

Expected: negotiated capability versions are available to authorization context.

## 13. Positive scenario S-005 — policy allow/deny/ask

Configure exact rules on B:

- A may `clipboard.write/set`;
- A is denied `clipboard.read/get`;
- A may `files.transfer/receive` only after synthetic owner approval.

Expected:

- write returns Allow;
- read returns Deny;
- file receive initially returns Ask;
- adding scoped verified approval then reevaluating returns Allow and creates a bounded operation.

## 14. Positive scenario S-006 — control request/response and event

Under authorized capability state:

- send control request with sequence 0 and random RequestId;
- receiver validates session/capability/policy and dispatches handler;
- response correlates to RequestId;
- send a permitted event with next sequence;
- both endpoints advance directional sequence correctly.

Expected: observable handler/test state and correlation are correct.

## 15. Positive scenario S-007 — authorized data stream

1. A requests `files.transfer/send` control operation.
2. B policy returns Allow/required approval fulfilled.
3. B creates active `OperationId` bound to A/B/session/capability/version/operation and `SingleStream` use policy.
4. A opens memory data stream with `DataStreamOpenV1` referencing the operation.
5. B atomically validates/reserves stream use.
6. bounded synthetic bytes flow with backpressure.
7. operation becomes Consumed/complete.

Expected: payload reaches handler only after operation admission and cannot open a second stream when single-use.

## 16. Positive scenario S-008 — disconnect and reconnect

- disconnect active session;
- session closes and session-scoped operations become unusable;
- establish a new memory connection/channel binding;
- authenticate again with fresh nonces;
- derive a different `SessionId`;
- exchange capabilities again;
- old request/operation IDs do not authorize new-session actions.

Expected: clean state restoration without session-authority reuse.

## 17. Positive scenario S-009 — revocation

While A and B have an active session:

- accept a valid owner-authorized revocation of A on B;
- B marks trust Revoked;
- active A-bound operations become Revoked;
- B session transitions Revoked -> Closed;
- new control/data work is denied;
- reconnect/authentication attempt from A fails despite a cryptographically valid old credential.

Expected: revocation takes effect locally without remote acknowledgment.

## 18. Required negative/security tests

### Identity/credential

```text
N-001 tampered device credential -> invalid signature
N-002 unknown owner -> reject
N-003 wrong issuer role -> reject
N-004 stale credential epoch -> reject
N-005 recovery key signs ordinary device credential -> reject
N-006 transport identifier matches trusted peer but credential does not -> reject
```

### Pairing

```text
N-010 wrong pairing secret confirmation -> reject/consume invitation
N-011 replay old pairing confirmation -> reject
N-012 pairing-id or nonce substitution -> reject
N-013 device-key substitution after transcript -> reject
N-014 cancelled/consumed invitation reused -> reject
N-015 wrong credential-accepted device key -> reject
```

### Session/protocol

```text
N-020 replay old session proof on fresh nonce/binding -> reject
N-021 wrong channel binding -> reject
N-022 protocol major no overlap -> reject
N-023 unknown required feature -> reject
N-024 malformed control frame -> reject
N-025 oversized declared frame -> reject before oversized allocation
N-026 duplicate/lower control message sequence -> reject
N-027 sequence gap on ordered control channel -> close/error
N-028 duplicate nonretryable RequestId -> reject
```

### Capability/policy/operation

```text
N-030 unsupported capability -> deny
N-031 trusted but no rule -> deny
N-032 explicit policy deny -> deny
N-033 missing approval -> Ask/no operation
N-034 spoofed peer approval evidence -> cannot satisfy local obligation
N-035 stream without OperationId -> reject
N-036 expired/cancelled/revoked OperationId -> reject
N-037 correct OperationId wrong session/peer/capability/direction -> reject
N-038 second stream using SingleStream operation -> reject
N-039 policy revision invalidates operation -> reject
```

### Lifecycle/resource

```text
N-040 revocation during active session -> operations cancelled/session closed
N-041 reconnect after revocation -> auth denied
N-042 bounded control queue saturation -> backpressure/resource error without unbounded growth
N-043 bounded stream queue saturation -> backpressure/resource error
N-044 cancellation during request -> owned task stops and state terminal
N-045 cancellation during stream setup -> no admitted dangling stream
N-046 clean runtime shutdown with active sessions -> all owned tasks terminate
```

## 19. Golden tests

Phase 1 must include fixed vectors for:

- KeyId derivation;
- authority delegation transcript/digest/signature;
- device credential transcript/digest/signature;
- pairing transcript digest + HMAC confirmations;
- credential-accepted proof;
- trust revocation transition signature;
- session-auth transcript/proofs/SessionId;
- protobuf encoding/decoding examples for public v1 messages where byte stability is required for interoperability fixtures.

Golden tests validate the specification, not internal private helper layout.

## 20. Fuzz targets

When the relevant parsers exist, add fuzz targets for:

- control frame length + protobuf decode/domain conversion;
- capability/operation identifier validation;
- pairing/session-auth bootstrap message parsing;
- data-stream open header parsing;
- canonical transcript builder inputs where untrusted parsed fields feed it.

Fuzzing is not required to block initial workspace scaffolding but is required before P1 protocol/parser milestone completion.

## 21. Concurrency/backpressure

Phase 1 async design requirements:

- all channels have explicit capacities;
- send paths await capacity or return a typed resource/cancellation error;
- no background busy polling;
- session/runtime tasks are owned and cancellable;
- stream copy uses bounded chunks/buffers;
- dropping a caller handle alone must not leak an unowned task with authority;
- shutdown has a deterministic test path.

Numeric capacities are implementation constants tuned for tests/performance and are not wire protocol values unless documented.

## 22. Error layering

Each crate exposes focused typed errors. The simulator application may use an application-level error/reporting wrapper, but library/domain crates do not erase typed security errors into strings.

Protocol errors are mapped deliberately from domain/core failures; internal details/secrets are not sent to peers.

## 23. Initial implementation milestones

### M1 — Rust workspace foundation

Deliver:

- workspace/toolchain manifests;
- `crypto`, `identity`, `policy`, `protocol`, `core`, `sim` package shells with correct dependency direction;
- baseline CI for format/clippy/test;
- no feature implementation beyond compile/test skeletons.

### M2 — Crypto + identity

Deliver shared v1 cryptographic primitives/canonical transcript engine, typed IDs, key generation, delegations, device credentials, verification, rotations/epochs, and golden/negative tests.

### M3 — Trust + policy

Deliver trust/revocation records, capability ID/version types, policy evaluator, approval evidence fixtures, authorized operation lifecycle, and security tests.

### M4 — Protocol

Deliver protobuf schemas/codecs, version negotiation, bounded framing, envelope/request/event/cancel/error/data-stream headers, canonical vectors, compatibility tests, and initial fuzz targets.

### M5 — Pairing + authenticated logical session simulator

Deliver memory transport, pairing orchestration, session auth/channel-binding fixture, capability exchange, control request/response/events.

### M6 — Authorized data streams

Deliver operation-bound stream admission, bounded data queues/chunks, backpressure, cancellation, and single/multistream use semantics needed by tests.

### M7 — Failure/security lifecycle

Deliver reconnect, replay rejection, active revocation, reconnect-after-revocation denial, queue saturation, cancellation race coverage, malformed input, and clean shutdown.

### M8 — Quinn transport (after simulator exit criteria)

Add `transports/quic` and prove real encrypted transport/channel binding/network fault behavior. Quinn is not required to declare the in-memory Core Simulator complete.

## 24. Verification commands once workspace exists

Every Rust milestone finishes with:

```text
cargo fmt --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace
```

Security/parser milestones additionally run their relevant golden/cross-version/fuzz checks. Fuzzing may run in a separate bounded CI/manual job if not suitable for the default unit-test command.

## 25. Core Simulator exit criteria

The in-memory Phase 1 Core Simulator is complete when:

- S-001 through S-009 pass;
- N-001 through N-046 applicable tests pass;
- required canonical/signature/protocol golden vectors pass;
- protocol compatibility tests pass;
- relevant parser fuzz targets exist and complete the configured bounded smoke run;
- formatting/clippy/test are green;
- there is no OS/platform/GUI/database/real-network dependency in the simulator core path;
- dependency direction matches the accepted architecture/ADRs;
- `CURRENT.md` identifies M8 Quinn as the next network milestone.

## 26. Phase 0 review inputs

Before creating the Rust workspace, Phase 0 review must confirm:

- P0.2 threat/security boundaries;
- P0.3 identity/key hierarchy;
- P0.4 pairing/trust/revocation;
- P0.5 policy/authorization;
- P0.6 protocol/canonical signing/compatibility;
- P0.7 session/transport contract;
- P0.8 recovery/update security;
- this P0.9 simulator specification;
- proposed ADRs for repository license, identity crypto profile, pairing bootstrap, protocol encoding, update trust, and crypto crate boundary.

Only accepted decisions are implemented as normative Phase 1 architecture.
