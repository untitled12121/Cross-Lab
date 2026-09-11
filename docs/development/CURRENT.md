# Current Development State

This file is the durable resume guide for active Cross-Lab development. Git history, repository contents, and verification results remain factual if this document becomes stale.

## Current Phase

**Phase 1 — Core Simulator**

## Current Milestone

**M5 — Pairing + Authenticated Logical Session Simulator: in progress.**

M1–M4 are integrated into canonical `main`. M5 Tasks 1–3 are complete and verified on the active implementation branch.

## Branch State

- `main` — canonical integrated branch; contains verified M1–M4.
- `m5-session` — active short-lived M5 implementation branch; draft PR #10 targets `main`.
- `planning` — planning/documentation branch; no active implementation belongs here.
- No temporary `*-red` branches are required for TDD; failing contract-test checkpoints remain ordinary commits on `m5-session`.

The connected GitHub workflow writes directly to the committed feature branch, so there is no separate uncommitted remote working-tree state. Repository history is the durable implementation state.

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

## M4 Integration

M4 was merged through PR #9 into `main` at `9dfc8cdb495bfc0a08adfca8377dde423250dcd6`.

Post-merge CI run `34589394208` completed successfully on `main`, covering the locked dependency graph, Rustfmt, workspace check, Clippy with warnings denied, and the full workspace test suite.

M4 provides the bounded transport-independent protocol domain required by M5: protocol/feature negotiation, framing, protobuf/domain conversion, envelopes, capability advertisements, requests/responses/events/cancellation/errors/session close, data-stream open headers, golden wire vectors, and parser fuzz smoke coverage.

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

M5 does **not** introduce real networking, Quinn, persistence, UI/platform adapters, authorized bulk data-stream admission, plugins, or privileged services.

The approved implementation keeps deterministic state machines and bounded in-memory queues without introducing a general async runtime dependency in M5.

## M5 Progress

### Task 1 — pairing confirmation primitives

Complete and committed on `m5-session`.

- added focused HMAC-SHA-256 generation and constant-time verification helpers in `crosslab-crypto`;
- kept generic cryptographic mechanics inside the accepted ADR-0006 boundary;
- covered positive and negative verification behavior.

### Task 2 — pairing transcript and invitation domain state

Complete and verified on `m5-session`.

- typed 128-bit `PairingId` and redacted/zeroized 256-bit `PairingSecret`;
- single-use invitation lifecycle with terminal consumed/cancelled/expired states;
- canonical `crosslab.pairing-transcript.v1` digest construction;
- role-separated inviter/joiner HMAC confirmations;
- wrong-secret/security-field/role rejection coverage;
- frozen transcript and directional HMAC golden vectors.

### Task 3 — pairing bootstrap protobuf messages

Complete and verified on `m5-session`.

Implemented:

- dedicated pre-session `PairingBootstrapV1` wrapper outside `EnvelopeV1`;
- typed hello, directional confirmation, and credential-accepted domain/wire messages;
- tracked `.proto` additions and matching hand-maintained Prost v1 schema mirror;
- fixed profile-v1 validation and strict role/signature-algorithm validation;
- exact 16/32/64-byte validation for pairing IDs, owner/device/key IDs, nonces, confirmations, transcript/signed-object digests, public keys, and signatures;
- Ed25519 public-key structural validation;
- bootstrap decoding through the existing 65,536-byte `FrameLimit::BootstrapHello` path so oversized declared frames fail before protobuf decoding;
- malformed profile/body/enum/length negative coverage;
- frozen pairing-confirmation bootstrap frame vector.

The Task 3 implementation code head is `d522cb462a3ec75cff47ed955b47a73db0768a3d`.

GitHub Actions CI run `34638127568` passed:

```text
cargo metadata --locked --no-deps --format-version 1
cargo fmt --check
cargo check --workspace --all-targets --all-features
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-features
```

Protocol fuzz smoke run `34638127476` also passed on the same code head. Task 3 did not add the dedicated pairing/session-auth fuzz targets reserved for M5 Task 9; this run verifies the existing bounded parser suite remains green after the protocol extension.

## Exact Next Task

Continue **M5 Task 4 — Pairing orchestration and trust commit**, following `docs/plans/phase-1/M5-pairing-session-simulator.md` test-first.

The next implementation slice should:

1. inspect `OwnerRootRecord`, `AuthorityDelegation`, `DeviceCredential`, trust-state APIs, Task 2 pairing transcript/confirmation types, and Task 3 pairing wire-domain types before editing;
2. create failing `crates/core/tests/pairing_flow.rs` coverage for the approved S-002 and N-010..N-015 pairing/trust scenarios;
3. implement `crates/core/src/pairing/flow.rs` as explicit inviter/joiner state machines with fail-closed transitions;
4. require both directional confirmations, valid owner/credential context, and final joiner proof of possession before any `TrustRecord::Trusted` result can be emitted;
5. enforce single-use invitation consumption and ensure cancellation, expiry, replay, owner mismatch, wrong proof, or partial failure can never produce trusted state;
6. run focused tests followed by the complete format/check/Clippy/workspace-test baseline;
7. commit the completed Task 4 slice on `m5-session` and update this handoff before starting Task 5.

Do not begin session-auth or transport work until Task 4 is verified.

After M5 integration, the next milestone is **M6 — Authorized Data Streams**.

## Resume Procedure

Before continuing in a new session:

1. inspect `main`, `m5-session`, draft PR #10, recent commits, branch comparison, and latest CI;
2. read `docs/architecture/MASTER-ARCHITECTURE.md` and this file;
3. read `docs/plans/phase-1/M5-pairing-session-simulator.md` plus the pairing/trust architecture and relevant ADRs;
4. inspect existing identity/trust/pairing implementation and tests for the Task 4 slice;
5. reconcile documentation with actual code before editing;
6. continue from **M5 Task 4** above.
