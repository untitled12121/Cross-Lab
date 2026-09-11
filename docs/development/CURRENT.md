# Current Development State

This file is the durable resume guide for active Cross-Lab development. Git history, repository contents, and verification results remain factual if this document becomes stale.

## Current Phase

**Phase 1 — Core Simulator**

## Current Milestone

**M5 — Pairing + Authenticated Logical Session Simulator: in progress.**

M1–M4 and verified M5 Tasks 1–4 are integrated into canonical `main`. The exact next implementation slice is M5 Task 5.

## Branch State

- `main` — canonical integrated branch; contains verified M1–M4 and M5 Tasks 1–4.
- `m5-session` — historical M5 Tasks 1–4 feature branch; PR #10 is merged into `main`.
- `planning` — planning/documentation branch; no active implementation belongs here.
- Start Task 5 from a fresh short-lived branch based on the verified current `main` head.
- No temporary `*-red` branches are required for TDD; failing contract-test checkpoints remain ordinary commits on the active implementation branch.

The connected GitHub workflow writes directly to committed branches, so there is no separate uncommitted remote working-tree state. Repository history is the durable implementation state.

## Architecture Baseline

- `docs/architecture/MASTER-ARCHITECTURE.md` — Revision 2.1, source of truth.
- `docs/architecture/CORE-SIMULATOR.md` — Phase 1 simulator and milestone specification.
- `docs/architecture/SESSION-TRANSPORT.md` — logical-session and transport contract.
- `docs/architecture/PAIRING-TRUST-REVOCATION.md` — pairing/trust semantics.
- `docs/protocol/PROTOCOL-V1.md` — protocol v1 wire/canonical rules.
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

## M5 Tasks 1–4 Integration

PR #10 was marked ready and merged into `main` with preserved commit history at merge commit `b54395109b8fe7ed49862f7f8c508659a2fd7a36`.

Post-merge `main` CI run `34643788450` passed the locked dependency graph, Rustfmt, workspace check, Clippy with warnings denied, and the full workspace test suite.

The integrated scope contains pairing cryptography/domain/bootstrap/orchestration, trust commit gating, focused public-key credential issuance, tests, and vectors. It does not contain Task 5 session authentication, Task 6 transport, or later M5 slices.

## Exact Next Task

Continue **M5 Task 5 — Session-auth domain and bootstrap wire contract**, following `docs/plans/phase-1/M5-pairing-session-simulator.md` test-first.

The next implementation slice should:

1. read the canonical session-auth fields and channel-binding rules in `docs/architecture/SESSION-TRANSPORT.md` and `docs/protocol/PROTOCOL-V1.md` before editing;
2. inspect existing credential, signing/transcript, protocol negotiation, Task 3 bootstrap wire, and M4 framing/conversion APIs for reuse;
3. create failing `crates/core/tests/session_auth.rs` and `crates/protocol/tests/session_auth_wire.rs` coverage for transcript/proof/`SessionId` golden behavior plus wrong nonce, channel binding, role, key, profile, enum, and malformed-length cases;
4. implement `crates/core/src/session/mod.rs` and `crates/core/src/session/auth.rs` with canonical `SessionAuthTranscriptV1`, role-separated initiator/responder proofs, and deterministic `SessionId` derivation;
5. add bounded pre-session session-auth hello/proof protobuf/domain messages and strict wire conversion outside ordinary post-auth `EnvelopeV1` control traffic;
6. keep identity independent of transport identity and bind authentication to owner/device credentials, fresh nonces, negotiated protocol/features, and deterministic channel binding;
7. run focused tests and the complete format/check/Clippy/workspace-test baseline, freeze required golden vectors, commit, and checkpoint this file before Task 6.

Do not begin the in-memory transport seam or session activation state machine until Task 5 is verified.

After M5 integration, the next milestone is **M6 — Authorized Data Streams**.

## Resume Procedure

Before continuing in a new session:

1. inspect `main`, the active M5 branch, open PRs, recent commits, branch comparison, and latest CI/fuzz runs;
2. read `docs/architecture/MASTER-ARCHITECTURE.md` and this file;
3. read `docs/plans/phase-1/M5-pairing-session-simulator.md`, `docs/architecture/SESSION-TRANSPORT.md`, and relevant ADRs;
4. reconcile documentation with actual code before editing;
5. branch from the verified current `main` head if no active Task 5 branch exists;
6. continue from **M5 Task 5** above.
