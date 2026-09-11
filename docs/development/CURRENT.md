# Current Development State

This file is the durable resume guide for active Cross-Lab development. Git history, repository contents, and verification results remain factual if this document becomes stale.

## Current Phase

**Phase 1 — Core Simulator**

## Current Milestone

**M5 — Pairing + Authenticated Logical Session Simulator: in progress.**

M1–M4 and verified M5 Tasks 1–4 are integrated into canonical `main`. M5 Task 5 is implemented and code-verified on `m5-session-auth`; its documentation-inclusive PR head must be verified and merged before Task 6 begins.

## Branch State

- `main` — canonical integrated branch; contains verified M1–M4 and M5 Tasks 1–4.
- `m5-session` — historical M5 Tasks 1–4 feature branch; PR #10 is merged into `main`.
- `m5-session-auth` — active M5 Task 5 feature branch; draft PR #11 targets `main`.
- `planning` — planning/documentation branch; no active implementation belongs here.
- Do not begin Task 6 until PR #11 is exact-head verified, merged into `main`, and canonical `main` is reverified.
- Start Task 6 from a fresh short-lived branch based on that verified `main` head.
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

Implemented:

- explicit inviter and joiner pairing state machines in `crates/core/src/pairing/flow.rs`;
- S-002 and N-010..N-015 security/flow coverage plus owner/delegation/context negatives;
- fail-closed replay, wrong-secret, pairing-id/nonce substitution, device-key substitution, invalid proof, cancelled/consumed invitation, owner mismatch, and invalid delegation handling;
- trust creation only after both directional pairing confirmations, valid credential-chain verification, and final joiner proof of possession;
- single-use invitation consumption on terminal success/failure paths;
- focused `DeviceCredential::issue_for_public_key` issuance path so the inviter never requires the joiner private key;
- deterministic credential-accepted proof golden vector.

Task 4 final code head `36583b732b20df6a7eef1905cec685ad50675a6b` passed CI `34641303311` and fuzz smoke `34641303272`.

The final PR-head checkpoint `588366d69dcd89d3a9a4f709fbca833eb263be57` passed CI `34643647466` and fuzz smoke `34643647458`.

### Task 5 — session-auth domain and bootstrap wire contract

Implementation complete and code-verified on `m5-session-auth`; final documentation-inclusive PR verification/integration is pending.

Implemented:

- canonical `SessionAuthTranscriptV1` using the 15 fields approved in `SESSION-TRANSPORT.md`;
- deterministic canonicalization of negotiated feature IDs and separate domain-separated digests for feature set, channel-binding profile, and channel-binding value;
- exact role-separated Ed25519 proof inputs for initiator and responder without an unintended extra hashing layer;
- deterministic `SessionId` derivation from the transcript digest plus both verified signatures;
- generic message signing/verification helpers inside `crosslab-crypto` while retaining existing digest-signing APIs;
- structural `DeviceCredential::from_signed_parts` import so session-auth wire decoding can reconstruct a signed credential without moving credential-chain verification into the protocol crate;
- dedicated bounded pre-session `SessionAuthBootstrapV1` hello/proof messages outside ordinary `EnvelopeV1` traffic;
- strict profile, role, algorithm, identity, public-key, credential, nonce, transcript-digest, signature, protocol-range, feature-count, feature-ID, and frame-limit validation;
- boxed session-auth hello storage at the domain credential/raw Prost oneof seams to satisfy the strict large-enum footprint lint without changing protobuf field tags or serialized wire bytes;
- frozen transcript digest, initiator proof, responder proof, and `SessionId` golden vectors;
- negative coverage for nonce/channel-binding substitution, wrong role/key/owner, malformed credentials, invalid protocol ranges/features, and malformed bootstrap fields.

TDD evidence:

- RED contract head `c37f598522b6aa114bd3f62c32a5590e96c1e7db` passed lockfile/rustfmt and failed `cargo check` on the intentionally missing signed-credential import/session-auth APIs in CI `34644867753`;
- vector-capture CI `34652914633` passed lockfile/rustfmt/check/Clippy and failed only the deliberate zero golden assertions;
- final code head `9dbbd50d99348ba4a06eaadbc25442eec627b3f3` passed CI `34653333962` and fuzz smoke `34653333970`.

`docs/protocol/PROTOCOL-V1.md` now registers the session-auth transcript field tags plus exact feature-set/channel-binding digest labels, role proof labels, and `SessionId` derivation required by the approved session architecture.

The uploaded-repository runtime was unavailable for local ZIP inspection during this slice, so related open-source trust/transport patterns were reviewed narrowly through the corresponding public upstream repositories. No external identity or transport architecture was copied into Cross-Lab.

## M5 Tasks 1–4 Integration

PR #10 was marked ready and merged into `main` with preserved commit history at merge commit `b54395109b8fe7ed49862f7f8c508659a2fd7a36`.

Post-merge `main` CI run `34643788450` passed the locked dependency graph, Rustfmt, workspace check, Clippy with warnings denied, and the full workspace test suite.

The integrated scope contains pairing cryptography/domain/bootstrap/orchestration, trust commit gating, focused public-key credential issuance, tests, and vectors. Task 5 remains on PR #11 until its final documentation-inclusive head is verified and merged.

## Exact Next Task

After PR #11 is exact-head verified, merged, and `main` is reverified, continue **M5 Task 6 — Bounded in-memory transport seam**, following `docs/plans/phase-1/M5-pairing-session-simulator.md` test-first.

The next implementation slice should:

1. read the transport-neutral connection/channel-binding contract in `docs/architecture/SESSION-TRANSPORT.md` before editing;
2. inspect the existing core session-auth, M4 framing/control, and simulator APIs plus relevant uploaded/public upstream transport references for reuse patterns without adopting their identity models;
3. create failing `apps/sim/tests/memory_transport.rs` coverage for bounded ordering, queue saturation, disconnect, close propagation, and distinct deterministic channel bindings;
4. create `crates/core/src/transport/mod.rs` as the narrow transport-neutral seam for connection metadata, channel binding, control send/receive, and close behavior;
5. create `apps/sim/src/transport.rs` with a deterministic bounded `MemoryTransportPair` using the `InProcessTest` security class;
6. keep transport-library types out of the core domain and use no unbounded queues, busy polling, real networking, or general async runtime dependency;
7. verify deterministic failure behavior, run the complete format/check/Clippy/workspace-test baseline, commit, and checkpoint this file before Task 7.

Do not begin the Task 7 logical-session activation/capability-exchange state machine until Task 6 is verified and integrated.

After M5 integration, the next milestone is **M6 — Authorized Data Streams**.

## Resume Procedure

Before continuing in a new session:

1. inspect `main`, `m5-session-auth`, PR #11, recent commits, branch comparison, and latest CI/fuzz runs;
2. read `docs/architecture/MASTER-ARCHITECTURE.md` and this file;
3. read `docs/plans/phase-1/M5-pairing-session-simulator.md`, `docs/architecture/SESSION-TRANSPORT.md`, and relevant ADRs;
4. reconcile documentation with actual code before editing;
5. finish Task 5 PR verification/integration if PR #11 is still open;
6. otherwise branch from the verified current `main` head and continue from **M5 Task 6** above.
