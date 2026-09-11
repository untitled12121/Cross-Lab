# Current Development State

This file is the durable resume guide for active Cross-Lab development. Git history, repository contents, and verification results remain factual if this document becomes stale.

## Current Phase

**Phase 1 — Core Simulator**

## Current Milestone

**M5 — Pairing + Authenticated Logical Session Simulator: in progress.**

Verified M1–M4 and M5 Tasks 1–5 are integrated into canonical `main`. M5 Task 6 is implemented and code-verified on `m5-memory-transport`; PR #12 must receive exact-head documentation-inclusive verification, merge into `main`, and pass post-merge `main` verification before Task 7 begins.

## Branch State

- `main` — canonical integrated branch; contains verified M1–M4 and M5 Tasks 1–5.
- `m5-session` — historical M5 Tasks 1–4 branch; PR #10 merged.
- `m5-session-auth` — historical M5 Task 5 branch; PR #11 merged.
- `m5-memory-transport` — active M5 Task 6 branch; draft PR #12 targets `main`.
- `planning` — planning/documentation branch; no active implementation belongs here.
- Do not begin Task 7 until PR #12 is exact-head verified, merged into `main`, and canonical `main` is reverified.
- Start Task 7 from a fresh short-lived branch based on that verified `main` head.
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

Implementation complete and code-verified on `m5-memory-transport`; final documentation-inclusive PR verification/integration is pending.

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
- implementation head `44d2bc83a604d33cbd00c1f8973c188db77196f0` reached rustfmt and required only one formatter rewrite in the core transport module;
- code-verified head `9690e20467f5fdd6c80779bbe3adced4134c7f60`: CI `34654834396` passed lockfile, Rustfmt, workspace check, Clippy with warnings denied, and the full workspace test suite.

No protocol parser or wire schema changed in Task 6, so the protocol-parser fuzz target surface is unchanged; Task 6's relevant verification gate is the complete workspace CI baseline above.

## Exact Next Task

After PR #12 is exact-head verified, merged, and `main` is reverified, continue **M5 Task 7 — Logical session activation and capability exchange**, following `docs/plans/phase-1/M5-pairing-session-simulator.md` test-first.

The next implementation slice should:

1. create failing `crates/core/tests/session.rs` S-003/S-004 and N-020..N-027-style coverage before production changes;
2. create `crates/core/src/session/state.rs` for the explicit `Created -> Authenticating -> Active -> Closing -> Closed` lifecycle plus the revocation terminal path;
3. create `crates/core/src/session/capabilities.rs` for post-authentication capability intersection;
4. modify `crates/core/src/session/mod.rs` only to compose/export the focused session features;
5. require credential/trust validation, protocol/feature negotiation, channel-binding validation, both directional proofs, derived `SessionId`, and directional sequence initialization before `Active`;
6. ensure capability advertisement/negotiation occurs only after authentication and never creates policy authority;
7. keep Task 8 control request/response/event dispatch out of Task 7;
8. run the full format/check/Clippy/workspace-test baseline, checkpoint this file, merge verified Task 7 into `main`, and reverify canonical `main` before Task 8.

Do not begin Task 8 until Task 7 is verified and integrated.

After M5 integration, the next milestone is **M6 — Authorized Data Streams**.

## Resume Procedure

Before continuing in a new session:

1. inspect `main`, `m5-memory-transport`, PR #12, recent commits, branch comparison, and latest CI runs;
2. read `docs/architecture/MASTER-ARCHITECTURE.md` and this file;
3. read `docs/plans/phase-1/M5-pairing-session-simulator.md`, `docs/architecture/SESSION-TRANSPORT.md`, and relevant ADRs;
4. reconcile documentation with actual code before editing;
5. finish Task 6 PR verification/integration if PR #12 is still open;
6. otherwise branch from the verified current `main` head and continue from **M5 Task 7** above.
