# Current Development State

This file is the durable resume guide for active Cross-Lab development. Git/code/tests are factual implementation state; this file records the intended handoff.

## Current Phase

**Phase 1 — Core Simulator / Real Transport Transition**

## Current Milestone

**M7 — Failure and Security Lifecycle: complete, verified, and integrated into canonical `main`.**

M1–M7 are complete and integrated. The exact next milestone is **M8 — Quinn transport**.

## Canonical Main State

- M7 verified branch head: `2d7ecebb7c785f1dfaaf825c0567ac78e1f7de71`.
- Final branch CI: `34681524693` — locked metadata, rustfmt, workspace check, Clippy `-D warnings`, and all tests passed.
- PR #17: `feat: implement M7 failure and security lifecycle` — merged with exact-head protection using merge method `merge`.
- M7 merge commit on `main`: `2974108dcf48a9d606b094fea24a9e5da991c513`.
- Post-merge canonical `main` CI: `34681576836` — locked metadata, rustfmt, workspace check, Clippy `-D warnings`, and all tests passed.
- M7 Fuzz Smoke was not scheduled/applicable because `.github/workflows/fuzz.yml` only triggers for `crates/protocol/**`, `crates/policy/**`, `fuzz/**`, or the fuzz workflow itself, and M7 changed none of those paths. This is not recorded as a fuzz pass.

## M7 Delivered

M7 proves failure cannot preserve stale authority:

- `LogicalSession::transport_lost()` makes transport loss terminal for active/closing/revoked sessions and idempotent for closed sessions.
- accepted peer revocation requires local `TrustState::Revoked`, exact authenticated owner/device, exact credential epoch, and a trust revision strictly newer than the authenticated snapshot;
- control terminal paths deterministically clear pending outgoing/inbound/replay state before close;
- stream runtime owns its logical session and cancels admitted stream/operation authority on parent transport loss, revocation, or shutdown while keeping stream-local cancellation local;
- reconnect uses a fresh transport/channel binding, fresh nonces/proofs, new `SessionId`, zeroed directional sequences, capability renegotiation, and fresh authorization;
- old control envelopes/request state and old `AuthorizedOperation` authority cannot cross a reconnect boundary;
- active signed revocation cancels ordinary control and stream authority locally and reconnect/authentication is denied while trust remains revoked;
- bounded control backpressure is transactional and existing stream saturation/cancellation/shutdown coverage satisfies N-043..N-046;
- malformed/fatal input and peer graceful close clear session-scoped dispatcher authority before terminal close;
- no real networking, async runtime, reconnect timing/backoff, persistence, UI/platform, privileged service, recovery, session ticket, 0-RTT, or M8 implementation entered M7.

## M7 Verification Highlights

- Task 1 lifecycle RED `623f326efc4711b1991e7de24a6cd58c46d6a84c`, CI `34675402081`; GREEN `f763bd034d8ac15d44e11b4ef659d1d907c75126`, CI `34675522726`.
- Task 2 final GREEN `3b52f75ec2cef774225c814a3bd04d4072d5f29b`, CI `34675870576`.
- Task 3 RED `0cb5d2e16a7454910eedba46862e083fa3fb68fc`, CI `34675990614`; GREEN `08515dd3bb2a93476a2004ee3f6b6e6ec8ad85e9`, CI `34676159053`.
- S-008 reconnect GREEN `29762639d6c6054aa3996b3429437e7c00469966`, CI `34676386066`.
- S-009 revocation GREEN `f50eb7da92f783b863725994c79643aef675e939`, CI `34676614329`.
- Task 6 RED `58eb0a914a9ca929ff48ccc25e481d51f43b4efb`, CI `34676901018`; GREEN `4026cc397fe63a54e5cc093e341cd1f37ca721d3`, CI `34676968464`.
- Final-review trust-revision guard RED `ad0f5ee4cf33a22fa834e6bf45b0fae8923b6bc3`, CI `34681339461`; GREEN `f87c43fa4c5ed252fe30358406efbba3faba746f`, CI `34681410654`.

## Architecture Baseline for M8

Primary contracts remain:

- uploaded Cross-Lab Master Architecture & Development Plan;
- `docs/architecture/MASTER-ARCHITECTURE.md`;
- `docs/architecture/CORE-SIMULATOR.md`;
- `docs/architecture/SESSION-TRANSPORT.md`;
- `docs/architecture/PAIRING-TRUST-REVOCATION.md`;
- M3 trust/operation authority, M5 authenticated logical sessions/control transport seam, M6 authorized streams, and M7 failure/revocation lifecycle.

M8 must preserve these boundaries:

- concrete Quinn types stay inside `transports/quic` and must not leak into core identity/policy/protocol/session domain state;
- channel binding is authenticated context, not transport identity authority;
- reconnect never resumes old session or operation authority without fresh approved authentication semantics;
- bounded queues/backpressure/cancellation/shutdown remain explicit;
- transport close/loss maps to the existing M7 terminal lifecycle contract;
- no UI/platform/privileged/persistence scope is pulled into transport work.

## Exact Next Task

Begin **M8 — Quinn transport design and implementation planning** from this exact verified `main` state after the integration-record CI passes.

Execution contract:

1. verify the final documentation-only `main` head and CI;
2. create a fresh M8 branch from that exact verified head;
3. re-read `SESSION-TRANSPORT.md`, relevant Master Architecture transport/security sections, M5–M7 seams, and current workspace dependency direction;
4. inspect the uploaded Quinn repository in detail for maintained APIs covering endpoint/connection lifecycle, bidirectional/unidirectional streams, datagrams if needed, close/cancellation, TLS/exporter/channel-binding options, backpressure, and platform/runtime implications;
5. inspect relevant Iroh/rust-libp2p references only where they clarify abstractions; do not copy their architecture;
6. verify the latest stable compatible Quinn/Tokio/rustls dependency versions before choosing dependencies;
7. write and checkpoint `docs/plans/phase-1/M8-quinn-transport-design.md` plus an implementation plan before production code;
8. keep the first implementation slice transport-neutral and test-driven, preserving the existing `TransportConnection`/stream seam rather than creating a second session architecture.

Do not start M8 production code until its design and implementation plan are written and reconciled with current code.

## Resume Procedure

1. inspect canonical `main`, recent commits, workflows, and `docs/development/CURRENT.md`;
2. read the Master Architecture and relevant ADRs/contracts;
3. inspect the active milestone plan and relevant uploaded research repositories;
4. reconcile documentation with actual code before editing;
5. execute small TDD milestones, verify full relevant gates, commit/push, and update this file after meaningful progress;
6. merge only exact verified milestone heads into `main` and verify canonical `main` after merge;
7. never rely on chat history or stash as the only copy of incomplete work.
