# Current Development State

This file is the durable resume guide for active Cross-Lab development. Git/code/tests are factual implementation state; this file records the intended handoff.

## Current Phase

**Phase 1 — Core Simulator**

## Current Milestone

**M7 — Failure and Security Lifecycle: planning complete; Task 1 core session lifecycle GREEN; Tasks 2–7 pending.**

M1–M6 are complete, verified, and integrated into canonical `main`.

## Branch State

- `main` — exact verified M6 integration-record head `d419f0fd410f0d2da69cc9bf2e4c20600033b7fb`; CI `34671802373` passed lockfile, rustfmt, workspace check, Clippy `-D warnings`, and all tests.
- `m7-failure-security-lifecycle` — active M7 branch created exactly from `d419f0fd410f0d2da69cc9bf2e4c20600033b7fb`.
- PR #17 — draft, `feat: implement M7 failure and security lifecycle`; keep draft until the exact final documentation-inclusive head passes CI and Fuzz Smoke.
- Current verified Task 1 head: `f763bd034d8ac15d44e11b4ef659d1d907c75126`.

## Architecture Baseline

Primary contracts:

- uploaded Cross-Lab Master Architecture & Development Plan;
- `docs/architecture/MASTER-ARCHITECTURE.md`;
- `docs/architecture/CORE-SIMULATOR.md`;
- `docs/architecture/SESSION-TRANSPORT.md`;
- `docs/architecture/PAIRING-TRUST-REVOCATION.md`;
- existing M3 trust/operation authority, M5 authenticated session/control/memory transport, and M6 stream admission/runtime.

M7 preserves these rules:

- transport loss is terminal for the old logical session;
- reconnect is a new transport + binding + fresh nonces/proofs + new `SessionId` + fresh capability/authorization state;
- old request/sequence/operation authority never transfers across sessions;
- ordinary revocation is first verified/applied through signed `TrustTransition` in policy;
- only then may session/runtime code react to the matching local `TrustRecord::Revoked`;
- accepted revocation cancels ordinary session-scoped authority and closes locally without peer acknowledgement;
- reconnect after revocation is denied despite a still-cryptographically-valid ordinary credential;
- no Quinn/Iroh/libp2p, socket/TLS, async runtime, reconnect timers/backoff, persistence, UI/platform, privileged, recovery reauthorization, session-ticket, or 0-RTT work enters M7.

No ADR is currently required because M7 implements the already-approved lifecycle contract.

## M7 Planning

Design:

`docs/plans/phase-1/M7-failure-security-lifecycle-design.md`

- `281a4f724d23f2435cee0e69c10c1b13dc758ec1` — approved M7 design checkpoint.

Implementation plan:

`docs/plans/phase-1/M7-failure-security-lifecycle.md`

- `f42e5a96aac4449bdf858869b114ae20bc08f401` — seven-task TDD plan.

Relevant research inspected before implementation:

- uploaded Quinn — connection close/loss makes connection-scoped work terminal; transport outcome stays below logical-session authority;
- uploaded Syncthing — reconnect scheduling/backoff belongs above individual connections;
- uploaded RustDesk — link heuristics/reconnect timing are product/network orchestration concerns rather than security-session authority.

Reuse is semantic only; M7 adds no external transport dependency.

## M7 Task 1 — Core Logical-Session Terminal Lifecycle

Focused test file:

`crates/core/tests/session_lifecycle.rs`

Behavior defined test-first:

- active transport loss closes the logical session immediately;
- repeated transport-loss notification is idempotent after closure;
- pre-auth `Created` state rejects the active-runtime transport-loss API;
- a real signed, applied, matching peer revocation drives `Active -> Revoked -> Closed`;
- a still-trusted record, wrong peer identity, or wrong credential epoch cannot terminate the session.

TDD evidence:

- RED: `623f326efc4711b1991e7de24a6cd58c46d6a84c`, CI `34675402081`; lockfile/rustfmt passed and workspace check failed exactly on the missing `transport_lost`, `apply_peer_revocation`, and `PeerNotRevoked` contract.
- GREEN implementation: `599f64d8f2afff0b76f52df2b34570f83e52ff19`; its first CI stopped at rustfmt only.
- formatting-only head: `f763bd034d8ac15d44e11b4ef659d1d907c75126`.
- exact-head CI `34675522726` passed lockfile, rustfmt, workspace check, Clippy `-D warnings`, and all tests.
- no Fuzz Smoke run was triggered for this core-only checkpoint.

Implemented core APIs:

- `LogicalSession::transport_lost(&mut self)` — `Active|Closing|Revoked -> Closed`, `Closed` idempotent, pre-auth states reject;
- `LogicalSession::apply_peer_revocation(&mut self, &TrustRecord)` — requires active session, `TrustState::Revoked`, exact authenticated owner/device, and exact authenticated credential epoch before entering `Revoked`;
- `SessionError::PeerNotRevoked`.

Trust-transition signature/issuer/revision validation remains owned by `crosslab-policy::TrustTransition`; core does not duplicate it.

## Exact Next Task

Begin **M7 Task 2 — control runtime cleanup on disconnect, active peer revocation, fatal failure, and shutdown**.

Execute test-first:

1. add focused control-lifecycle RED tests rather than enlarging unrelated scenario fixtures;
2. prove send-side and receive-side transport closure make an active node session `Closed` and clear pending request authority;
3. prove valid matching revoked trust closes the node/transport, while wrong-device revocation is rejected;
4. add narrow `ControlDispatcher::cancel_session_state()` and simulator terminal helpers only after valid RED;
5. preserve `ControlSendError::Full(frame)` as nonterminal backpressure and never commit dispatcher sequence/request state until transport send succeeds;
6. run focused and workspace gates, commit/push exact GREEN evidence, then update this file before Task 3.

Do not begin Task 3 until Task 2 is verified.

## Remaining M7 Tasks

- Task 3 — make `SimStreamRuntime` own its logical session and terminate stream authority on disconnect/revocation/shutdown;
- Task 4 — S-008 fresh reconnect + stale request/sequence/operation rejection;
- Task 5 — S-009 active revocation + N-040/N-041 reconnect-after-revocation denial;
- Task 6 — fill only missing N-042..N-046 resource/cancellation/shutdown lifecycle assertions;
- Task 7 — scope review, final CI/fuzz, PR merge to `main`, post-merge verification, final `CURRENT.md`, then M8 handoff.

## Resume Procedure

1. inspect `main`, `m7-failure-security-lifecycle`, PR #17, recent commits, and workflows;
2. read the Master Architecture, this file, M7 design/plan, and lifecycle contracts;
3. reconcile docs with actual code before editing;
4. execute one TDD task at a time and preserve exact RED/GREEN evidence;
5. merge only an exact verified final M7 head to `main`, verify canonical `main`, then write/verify the final integration record;
6. never rely on chat history or stash as the only copy of incomplete work.
