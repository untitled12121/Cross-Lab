# Current Development State

This file is the durable resume guide for active Cross-Lab development. Git history, repository contents, and verification results remain factual if this document becomes stale.

## Current Phase

**Phase 1 — Core Simulator**

## Current Milestone

**M5 — Pairing + Authenticated Logical Session Simulator: in progress.**

M1–M4 are integrated into canonical `main`. M5 Tasks 1–2 are complete and verified on the active implementation branch.

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

Implemented:

- typed 128-bit `PairingId`;
- redacted, zeroized 256-bit `PairingSecret`;
- single-use invitation lifecycle with `Pending`, `Consumed`, `Cancelled`, and `Expired` terminal behavior;
- canonical pairing transcript using the protocol-v1 `crosslab.pairing-transcript.v1` domain and fixed field registry;
- role-separated inviter/joiner HMAC-SHA-256 confirmations over the canonical transcript digest;
- rejection tests for wrong secret, pairing ID, device key, nonce, and confirmation role;
- frozen golden vectors for the transcript digest and both directional confirmations.

Verification at implementation head `db4cd1c3e9d1507d106bbfc75622606fc1b14051` succeeded in GitHub Actions run `34617038160`:

```text
cargo metadata --locked --no-deps --format-version 1
cargo fmt --check
cargo check --workspace --all-targets --all-features
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-features
```

All steps passed.

## Exact Next Task

Continue **M5 Task 3 — bounded pairing/session-auth bootstrap protobuf messages and strict conversion**, following `docs/plans/phase-1/M5-pairing-session-simulator.md` test-first.

The next implementation slice should:

1. read the Task 3 file list and exact message/limit requirements from the active M5 plan and protocol specification;
2. inspect existing `crosslab-protocol` schemas, conversion helpers, limits, and parser tests before editing;
3. add failing contract tests for pairing/session-auth bootstrap messages, exact-length identifiers/keys/nonces/proofs, collection bounds, malformed/unknown values, and frame limits required by the plan;
4. implement the minimum protobuf/domain types and strict bounded conversion needed to make those tests pass;
5. run focused protocol tests, then the complete format/check/Clippy/workspace-test baseline;
6. commit the completed Task 3 slice on `m5-session` and update this handoff before moving to Task 4.

Do not begin the in-memory transport/session state-machine work until Task 3 is verified.

After M5 integration, the next milestone is **M6 — Authorized Data Streams**.

## Resume Procedure

Before continuing in a new session:

1. inspect `main`, `m5-session`, draft PR #10, recent commits, branch comparison, and latest CI;
2. read `docs/architecture/MASTER-ARCHITECTURE.md`;
3. read this file;
4. read `docs/plans/phase-1/M5-pairing-session-simulator.md` and the relevant M5 architecture/protocol specifications;
5. inspect the existing protocol implementation and tests for the Task 3 slice;
6. reconcile documentation with actual code before coding;
7. continue from **M5 Task 3** above.
