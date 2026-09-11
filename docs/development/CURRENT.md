# Current Development State

This file is the durable resume guide for active Cross-Lab development. Git history, repository contents, and verification results remain factual if this document becomes stale.

## Current Phase

**Phase 1 — Core Simulator**

## Current Milestone

**M5 — Pairing + Authenticated Logical Session Simulator: approved and starting.**

M1–M4 are integrated into canonical `main`.

## Branch State

- `main` — canonical integrated branch; contains verified M1–M4.
- `planning` — planning/documentation branch; no active implementation belongs here.
- M5 uses one short-lived implementation branch only. No temporary `*-red` branches are required for TDD.

## Architecture Baseline

- `docs/architecture/MASTER-ARCHITECTURE.md` — Revision 2.1, source of truth.
- `docs/architecture/CORE-SIMULATOR.md` — Phase 1 simulator and milestone specification.
- `docs/architecture/SESSION-TRANSPORT.md` — logical-session and transport contract.
- `docs/architecture/PAIRING-TRUST-REVOCATION.md` — pairing/trust semantics.
- `docs/protocol/PROTOCOL-V1.md` — protocol v1 wire/canonical rules.
- ADR-0003 — single-use 256-bit pairing secret with directional HMAC-SHA-256 confirmations.
- ADR-0004 — Protocol Buffers for ordinary v1 wire encoding with independent canonical signing transcripts.
- ADR-0007 — event namespace and session-close wire registry.

## M4 Integration

M4 was merged through PR #9 into `main` at `9dfc8cdb495bfc0a08adfca8377dde423250dcd6`.

Post-merge CI run `34589394208` completed successfully on `main`, covering the locked dependency graph, Rustfmt, workspace check, Clippy with warnings denied, and the full workspace test suite.

M4 provides the bounded transport-independent protocol domain required by M5: protocol/feature negotiation, framing, protobuf/domain conversion, envelopes, capability advertisements, requests/responses/events/cancellation/errors/session close, data-stream open headers, golden wire vectors, and parser fuzz smoke coverage.

## M5 Approved Scope

M5 will implement the smallest transport-neutral simulator slice required by the Phase 1 specification:

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

## Exact Next Task

1. create the single M5 implementation branch from current `main`;
2. add the focused M5 implementation plan against the accepted simulator/session/pairing specifications;
3. implement pairing confirmation primitives and pairing-domain state test-first;
4. add bounded pairing/session-auth protobuf bootstrap messages and strict conversion;
5. implement core pairing/session state machines plus the in-memory simulator transport;
6. exercise capability exchange and sequenced request/response/event scenarios;
7. run the complete format/check/Clippy/test baseline and relevant parser fuzz smoke;
8. update this handoff, review the M5 diff, and integrate only when verified.

After M5 integration, the next milestone is **M6 — Authorized Data Streams**.

## Resume Procedure

Before continuing in a new session:

1. inspect `main`, the active M5 branch/PR if present, recent commits, branches, and CI;
2. read `docs/architecture/MASTER-ARCHITECTURE.md`;
3. read this file;
4. read `CORE-SIMULATOR.md`, `SESSION-TRANSPORT.md`, `PAIRING-TRUST-REVOCATION.md`, and relevant protocol/identity/policy code;
5. reconcile documentation with actual code before coding;
6. continue from the exact next task above.
