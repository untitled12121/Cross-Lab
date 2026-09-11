# Current Development State

This file is the durable resume guide for active Cross-Lab development. Git history, repository contents, and verification results remain factual if this document becomes stale.

## Current Phase

**Phase 1 — Core Simulator**

## Current Milestone

**M5 — Pairing + Authenticated Logical Session Simulator: in progress.**

Verified M1–M4 and M5 Tasks 1–7 are integrated into canonical `main`. The next implementation slice is M5 Task 8 after this integration-record commit is verified on `main`.

## Branch State

- `main` — canonical integrated branch; contains verified M1–M4 and M5 Tasks 1–7.
- `m5-session` — historical M5 Tasks 1–4 branch; PR #10 merged.
- `m5-session-auth` — historical M5 Task 5 branch; PR #11 merged.
- `m5-memory-transport` — historical M5 Task 6 branch; PR #12 merged.
- `m5-session-state` — historical M5 Task 7 branch; PR #13 merged.
- `planning` — planning/documentation branch; no active implementation belongs here.
- Start Task 8 from a fresh short-lived branch based on the verified current `main` head.
- No temporary `*-red` branches are required for TDD; failing contract-test checkpoints remain ordinary commits on the active implementation branch.

The connected GitHub workflow writes directly to committed branches, so there is no separate uncommitted remote working-tree state. Repository history is the durable implementation state.

## Architecture Baseline

- `docs/architecture/MASTER-ARCHITECTURE.md` — Revision 2.1, source of truth.
- `docs/architecture/CORE-SIMULATOR.md` — Phase 1 simulator and milestone specification.
- `docs/architecture/SESSION-TRANSPORT.md` — logical-session and transport contract.
- `docs/architecture/PAIRING-TRUST-REVOCATION.md` — pairing/trust semantics.
- `docs/protocol/PROTOCOL-V1.md` — protocol v1 wire/canonical rules, including the session-auth v1 canonical registry.
- ADR-0003 — single-use 256-bit pairing secret with directional HMAC-SHA-256 confirmations.
- ADR-0004 — Protocol Buffers for ordinary v1 wire encoding with independent canonical signing transcripts.
- ADR-0006 — focused `crosslab-crypto` foundation boundary used by identity, pairing, session, and later recovery cryptographic mechanics.
- ADR-0007 — event namespace and session-close wire registry.

## M5 Approved Scope

M5 implements the smallest transport-neutral simulator slice required by the Phase 1 specification:

- pairing profile v1 orchestration using the single-use out-of-band secret;
- canonical pairing transcript and directional confirmation verification;
- credential issuance plus joiner proof of private-key possession before trust commit;
- bounded in-memory transport with deterministic `InProcessTest` channel binding;
- authenticated logical-session transcript, role-separated device proofs, and derived `SessionId`;
- protocol/feature negotiation and capability exchange after session activation;
- sequenced control request/response/event flow over the in-memory control channel;
- required positive and negative M5 simulator/security tests.

M5 does **not** introduce real networking, Quinn, persistence, UI/platform adapters, authorized M6 bulk data-stream admission, plugins, or privileged services.

The approved implementation keeps deterministic state machines and bounded in-memory queues without introducing a general async runtime dependency in M5.

## M5 Progress

### Task 1 — pairing confirmation primitives

Complete, verified, and integrated.

- focused HMAC-SHA-256 generation and constant-time verification helpers in `crosslab-crypto`;
- generic cryptographic mechanics remain inside the ADR-0006 boundary;
- positive and tamper-rejection coverage.

### Task 2 — pairing transcript and invitation domain state

Complete, verified, and integrated.

- typed 128-bit `PairingId` and redacted/zeroized 256-bit `PairingSecret`;
- single-use invitation lifecycle with terminal consumed/cancelled/expired states;
- canonical `crosslab.pairing-transcript.v1` digest construction;
- role-separated inviter/joiner HMAC confirmations;
- wrong-secret/security-field/role rejection coverage;
- frozen transcript and directional HMAC golden vectors.

### Task 3 — pairing bootstrap protobuf messages

Complete, verified, and integrated.

- dedicated pre-session `PairingBootstrapV1` outside `EnvelopeV1`;
- typed hello, directional confirmation, and credential-accepted domain/wire messages;
- strict profile, enum, public-key, fixed-length, and bootstrap frame validation;
- frozen pairing-confirmation bootstrap frame vector.

Task 3 code head `d522cb462a3ec75cff47ed955b47a73db0768a3d` passed CI `34638127568` and fuzz smoke `34638127476`.

### Task 4 — pairing orchestration and trust commit

Complete, verified, and integrated.

- explicit inviter and joiner pairing state machines in `crates/core/src/pairing/flow.rs`;
- S-002 and N-010..N-015 security/flow coverage plus owner/delegation/context negatives;
- fail-closed replay, wrong-secret, pairing-id/nonce substitution, device-key substitution, invalid proof, cancelled/consumed invitation, owner mismatch, and invalid delegation handling;
- trust creation only after both directional pairing confirmations, valid credential-chain verification, and final joiner proof of possession;
- single-use invitation consumption on terminal success/failure paths;
- focused `DeviceCredential::issue_for_public_key` issuance path so the inviter never requires the joiner private key;
- deterministic credential-accepted proof golden vector.

Task 4 final PR head `588366d69dcd89d3a9a4f709fbca833eb263be57` passed CI `34643647466` and fuzz smoke `34643647458`. PR #10 merged into `main` at `b54395109b8fe7ed49862f7f8c508659a2fd7a36`; post-merge `main` CI `34643788450` passed.

### Task 5 — session-auth domain and bootstrap wire contract

Complete, verified, and integrated.

Implemented:

- canonical `SessionAuthTranscriptV1` using the 15 fields approved in `SESSION-TRANSPORT.md`;
- deterministic canonicalization of negotiated feature IDs and separate domain-separated digests for feature set, channel-binding profile, and channel-binding value;
- exact role-separated Ed25519 proof inputs for initiator and responder without an unintended extra hashing layer;
- deterministic `SessionId` derivation from the transcript digest plus both verified signatures;
- generic message signing/verification helpers inside `crosslab-crypto` while retaining existing digest-signing APIs;
- structural `DeviceCredential::from_unverified_signed_parts` import so received signed credential data is explicitly unverified until identity/core verification succeeds;
- dedicated bounded pre-session `SessionAuthBootstrapV1` hello/proof messages outside ordinary `EnvelopeV1` traffic;
- strict profile, role, algorithm, identity, public-key, credential, nonce, transcript-digest, signature, protocol-range, feature-count, feature-ID, and frame-limit validation;
- boxed session-auth hello storage at the domain credential/raw Prost oneof seams to satisfy strict large-enum footprint lint without changing protobuf field tags or serialized wire bytes;
- frozen transcript digest, initiator proof, responder proof, and `SessionId` golden vectors;
- negative coverage for nonce/channel-binding substitution, wrong role/key/owner, malformed credentials, invalid protocol ranges/features, and malformed bootstrap fields;
- session-auth canonical field tags, helper digest labels, role proof labels, and `SessionId` derivation registered in `docs/protocol/PROTOCOL-V1.md`.

TDD and verification evidence:

- valid RED head `c37f598522b6aa114bd3f62c32a5590e96c1e7db`: CI `34644867753` passed lockfile/rustfmt and failed `cargo check` on the intentionally missing APIs;
- vector-capture CI `34652914633` passed lockfile/rustfmt/check/Clippy and failed only deliberate zero golden assertions;
- pre-refinement code head `9dbbd50d99348ba4a06eaadbc25442eec627b3f3` passed CI `34653333962` and fuzz smoke `34653333970`;
- pre-merge review renamed the structural import to `from_unverified_signed_parts` so unverified state is explicit;
- final exact PR head `8ad7c79eca6e8aa70c887d27d3a66aa713df9d9f` passed CI `34653899890` and fuzz smoke `34653899783`;
- PR #11 merged with preserved history at `8cb6f013055a4f05ff599cc0d31adac45d2756b4`;
- post-merge canonical `main` CI `34654018205` passed lockfile, Rustfmt, workspace check, Clippy with warnings denied, and the full workspace test suite;
- final integration-record `main` head `f2ace2758af626f442d41308799d19806ad46904` passed CI `34654100309`.

The uploaded-repository runtime was unavailable for local ZIP inspection during Task 5, so related open-source trust/transport patterns were reviewed narrowly through corresponding public upstream repositories. No external identity or transport architecture was copied into Cross-Lab.

### Task 6 — bounded in-memory transport seam

Complete, verified, and integrated.

Implemented:

- narrow transport-neutral `crosslab-core` seam for `TransportSecurityClass`, opaque `ChannelBinding`, diagnostic `ConnectionMetadata`, bounded nonblocking control send/receive, close, and closed-state observation;
- payload-preserving `ControlSendError::Full`/`Closed` so backpressure or shutdown never silently drops the caller's frame, with payload-redacting `Debug` and generic `Display` output;
- simulator-owned `MemoryTransportPair` with two endpoints, a shared bounded FIFO per control direction, explicit nonzero capacity, and deterministic `InProcessTest` channel binding;
- deterministic connection-scoped endpoint metadata without treating metadata as identity or authority;
- ordered control-frame delivery and `Empty` receive behavior while an inbound direction remains open;
- explicit queue-saturation backpressure without loss;
- deterministic `disconnect_now` fault injection that abandons queued work and closes both endpoints;
- idempotent full connection close with peer propagation;
- deterministic one-direction close injection while preserving the reverse direction;
- distinct test channel bindings across separate connections and matching binding semantics across paired endpoints;
- simulator `src/lib.rs` module exposure so integration tests exercise the same adapter API future simulator composition will use;
- no new dependency, async runtime, unbounded queue, busy polling, real networking, toy encryption, transport-library type leakage, or Task 7 session state.

The plan listed `apps/sim/Cargo.toml` and `apps/sim/src/main.rs` as possible modifications. No edit was necessary: Cargo automatically discovers `src/lib.rs`, existing dependencies already cover the adapter, and the empty simulator binary has no Task 6 composition behavior to add. This keeps the slice smaller without changing the approved transport architecture.

Tests cover bounded FIFO ordering, frame-preserving saturation, disconnect/queued-work abandonment, idempotent close propagation, directional close behavior, `InProcessTest` classification, matching/distinct channel bindings, and neutral endpoint metadata.

TDD and verification evidence:

- initial RED commit `dc1fd126e32e3e6b8710988158540d3e91c9289f` was formatting-only and is not counted as valid RED evidence;
- valid RED head `e27a2c115b635986a7193559aeb7ab5c8898f309`: CI `34654445082` passed lockfile/rustfmt and failed `cargo check` specifically because the Task 6 core transport exports and simulator library/adapter did not yet exist;
- code-verified head `9690e20467f5fdd6c80779bbe3adced4134c7f60`: CI `34654834396` passed lockfile, Rustfmt, workspace check, Clippy with warnings denied, and the full workspace test suite;
- final exact PR head `9f2fcaf157245b6dd1f1ab1083e9507961cc7e8a` passed CI `34654964311`;
- PR #12 merged with preserved history at `32bf75951c0e0e768efac80c4320059e04550483`;
- post-merge canonical `main` CI `34655051368` passed lockfile, Rustfmt, workspace check, Clippy with warnings denied, and the full workspace test suite.

No protocol parser or wire schema changed in Task 6, so the protocol-parser fuzz target surface was unchanged; Task 6's relevant verification gate was the complete workspace CI baseline.

### Task 7 — logical session activation and capability exchange

Complete, verified, and integrated.

Implemented:

- explicit logical-session states `Created`, `Authenticating`, `Active`, `Closing`, `Closed`, and `Revoked`;
- activation fails closed and transitions to `Closed` if credential-chain verification, trust validation, protocol negotiation, required-feature negotiation, channel-bound transcript proof verification, or deterministic `SessionId` derivation fails;
- session authentication reuses the existing `DeviceCredential`, `AuthorityDelegation`, `TrustRecord`, protocol-version negotiator, feature negotiator, `SessionAuthTranscriptV1`, role-separated proofs, and deterministic `SessionId` derivation rather than creating parallel trust/security models;
- local/peer identity is selected by the local session-auth role while the canonical transcript remains initiator/responder ordered;
- authenticated `SessionContext` records owner, local/peer device IDs, peer accepted credential epoch/trust revision, negotiated protocol/features, transport security class, and fresh directional control sequence counters initialized to zero;
- peer trust must be `Trusted`, match the authenticated owner/device identity, and pin the exact accepted credential epoch;
- replayed proofs under fresh nonces and proofs bound to a different channel binding fail closed;
- capability exchange is allowed only after `Active` and computes a deterministic highest compatible runtime-available `(CapabilityId, CapabilityVersion)` intersection;
- capability negotiation stores descriptive session metadata only and does not mutate `PolicyState`, create `AuthorizationGrant`, or bypass the existing independent policy gate;
- two focused `CapabilityVersionRange` accessors expose major/minor bounds needed for correct intersection without duplicating policy capability types;
- explicit graceful close and revocation terminal transitions reject invalid state changes;
- Task 8 control request/response/event dispatch remains out of Task 7.

Tests cover successful S-003-style activation, fresh sequence initialization, replayed proof/fresh nonce rejection, wrong channel binding rejection, incompatible protocol rejection, unsupported required-feature rejection, trust identity mismatch, accepted credential epoch mismatch, post-auth-only capability exchange, policy non-authority, and close/revocation lifecycle behavior.

TDD and verification evidence:

- initial Task 7 test head `3a4fafcc8cb72857549fa0e40edb3a76c4d4ea82` stopped at rustfmt and is not counted as valid RED evidence;
- valid RED head `3a7af8ad57af1ecec2ae52753f80498e0c1a105c`: CI `34655692501` passed lockfile/rustfmt and failed `cargo check` specifically because `LogicalSession`, `SessionActivation`, `SessionHandshakeSide`, `SessionState`, and `SessionError` did not yet exist;
- exact code head `606f9022c1c4e702f88d1d71cbee907dc26d5b1b` passed CI `34656446809` and Fuzz Smoke `34656446822`;
- documentation-inclusive PR head `94f418469a24dcd0d9598311a1d20bfda1bbef3a` passed CI `34658806668` and Fuzz Smoke `34658806735`;
- PR #13 merged with preserved history at `e17075245b566522bb4c822cc22b3ee2245fe7a7`;
- post-merge canonical `main` CI `34658951190` passed lockfile, Rustfmt, workspace check, Clippy with warnings denied, and the full workspace test suite.

## Exact Next Task

Continue **M5 Task 8 — Sequenced control request/response/event simulator**, following `docs/plans/phase-1/M5-pairing-session-simulator.md` test-first.

Task 8 should:

1. create failing `apps/sim/tests/session_scenarios.rs` S-006 plus duplicate/gap/nonretryable request coverage before production changes;
2. create `crates/core/src/control/mod.rs` for bounded authenticated M4 envelope dispatch and modify core exports only as needed;
3. create `apps/sim/src/node.rs` for simulator composition over the existing bounded memory transport and logical-session context;
4. use existing M4 envelope/message kinds, sequence validators, request/response correlation, event/session-close registry, cancellation semantics, and policy authorization rather than duplicating protocol or policy logic;
5. fail malformed, duplicate, gap, stale-session, invalid-kind, unauthorized, or nonretryable control traffic closed without granting capability authority;
6. keep M6 data-stream admission and all real networking out of Task 8;
7. run exact-head format/check/Clippy/tests plus fuzz smoke, checkpoint this file, merge verified Task 8 into `main`, and reverify canonical `main` before Task 9.

Task 9 then completes M5 with end-to-end scenarios, pairing/session-auth parser fuzz targets, fuzz workflow expansion, full baseline verification, final scope review, `CURRENT.md` M6 handoff, and canonical `main` integration.

After M5 integration, the next milestone is **M6 — Authorized Data Streams**.

## Resume Procedure

Before continuing in a new session:

1. inspect `main`, recent commits, branch state, and latest CI runs;
2. read `docs/architecture/MASTER-ARCHITECTURE.md` and this file;
3. read `docs/plans/phase-1/M5-pairing-session-simulator.md`, `docs/architecture/SESSION-TRANSPORT.md`, and relevant ADRs;
4. reconcile documentation with actual code before editing;
5. branch from the verified current `main` head and continue from **M5 Task 8** above.
