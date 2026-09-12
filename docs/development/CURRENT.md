# Current Development State

This file is the durable resume guide for active Cross-Lab development. Git/code/tests are factual implementation state; this file records the intended handoff.

## Current Phase

**Phase 1 — Core Simulator**

## Current Milestone

**M7 — Failure and Security Lifecycle: planning complete; Tasks 1–3 GREEN; Tasks 4–7 pending.**

M1–M6 are complete, verified, and integrated into canonical `main`.

## Branch State

- `main` — exact verified M6 integration-record head `d419f0fd410f0d2da69cc9bf2e4c20600033b7fb`; CI `34671802373` passed locked metadata, rustfmt, workspace check, Clippy `-D warnings`, and all tests.
- `m7-failure-security-lifecycle` — active M7 branch created exactly from that verified `main` head.
- PR #17 — draft, `feat: implement M7 failure and security lifecycle`; keep draft until the exact final documentation-inclusive head passes CI and Fuzz Smoke.
- Current verified M7 production/test head: `08515dd3bb2a93476a2004ee3f6b6e6ec8ad85e9`.

## Architecture Baseline

Primary contracts:

- uploaded Cross-Lab Master Architecture & Development Plan;
- `docs/architecture/MASTER-ARCHITECTURE.md`;
- `docs/architecture/CORE-SIMULATOR.md`;
- `docs/architecture/SESSION-TRANSPORT.md`;
- `docs/architecture/PAIRING-TRUST-REVOCATION.md`;
- M3 signed trust/operation authority, M5 authenticated session/control/memory transport, and M6 stream admission/runtime.

M7 invariants:

- transport loss is terminal for the old logical session;
- reconnect creates a new transport/binding/nonces/proofs/`SessionId` and fresh capability/authorization state;
- old request/sequence/operation authority never transfers across sessions;
- ordinary revocation is verified/applied through signed `TrustTransition` in policy before runtime reaction;
- accepted matching local revocation cancels ordinary session-scoped authority and closes without peer acknowledgement;
- revoked trust denies later ordinary authentication;
- no real networking, Quinn/Iroh/libp2p, async runtime, reconnect timers/backoff, persistence, UI/platform, privileged, recovery-reauthorization, ticket, or 0-RTT scope enters M7.

No ADR is currently required because M7 implements the approved lifecycle contract.

## M7 Planning

- Design: `docs/plans/phase-1/M7-failure-security-lifecycle-design.md`, checkpoint `281a4f724d23f2435cee0e69c10c1b13dc758ec1`.
- Implementation plan: `docs/plans/phase-1/M7-failure-security-lifecycle.md`, checkpoint `f42e5a96aac4449bdf858869b114ae20bc08f401`.
- Research inspected: uploaded Quinn connection close/loss semantics; Syncthing reconnect orchestration; RustDesk transient reconnect heuristics. Reuse is semantic only.

## M7 Task 1 — Core Session Terminal Lifecycle

Delivered:

- focused `crates/core/tests/session_lifecycle.rs`;
- `LogicalSession::transport_lost()` with active/closing/revoked terminal closure and closed-state idempotence;
- `LogicalSession::apply_peer_revocation()` requiring an active session, local `TrustState::Revoked`, exact authenticated owner/device, and exact credential epoch;
- `SessionError::PeerNotRevoked`;
- trust-transition signature/issuer/revision validation stays in `crosslab-policy::TrustTransition`.

Evidence:

- RED `623f326efc4711b1991e7de24a6cd58c46d6a84c`, CI `34675402081` — missing lifecycle APIs only;
- implementation `599f64d8f2afff0b76f52df2b34570f83e52ff19`, first CI stopped at rustfmt;
- formatting-only GREEN head `f763bd034d8ac15d44e11b4ef659d1d907c75126`, CI `34675522726` — all Rust gates passed.

## M7 Task 2 — Control Runtime Terminal Cleanup

Delivered:

- focused `apps/sim/tests/control_lifecycle.rs` using normal session authentication and real signed trust revocation;
- `ControlDispatcher::cancel_session_state()` clears pending outgoing, inbound request, and nonretryable replay bookkeeping for a dead session without resetting it for reuse;
- send-side `ControlSendError::Closed(frame)` and receive-side `ControlReceiveError::Closed` terminate the active logical session and clear session-scoped dispatcher authority;
- `ControlSendError::Full(frame)` remains nonterminal and does not commit dispatcher state;
- `SimNode::apply_peer_revocation()` validates through the core session API, clears dispatcher authority, closes transport, and completes `Revoked -> Closed`;
- `SimNode::shutdown()` is idempotent, clears session-scoped control state, terminates the logical session, and closes transport;
- wrong-device revoked records return the typed session mismatch and leave the unrelated active session/transport intact.

Evidence:

- transport-loss RED `ec7682a852c813da605586dc66f47ba1540813d8`, CI `34675642797` — workspace compiled and both new tests failed because sessions remained `Active`;
- transport-loss GREEN `a959a58715ffc9270cfa7d2b290cc9161a6b0143`, CI `34675751139` — all gates passed;
- revocation/shutdown RED `57cddc7a883b199f7721fdd2ec5ee9d1df34e74b`, CI `34675821698` — lockfile/rustfmt passed and check failed only on missing `SimNode::apply_peer_revocation` / `shutdown`;
- final Task 2 GREEN `3b52f75ec2cef774225c814a3bd04d4072d5f29b`, CI `34675870576` — locked metadata, rustfmt, workspace check, Clippy `-D warnings`, and all tests passed.

## M7 Task 3 — Stream Runtime Terminal Lifecycle

Delivered:

- refactored `apps/sim/tests/stream_scenarios.rs` so authenticated sessions are moved exactly once into stream runtimes without cloning authority or shared mutable session handles;
- `SimStreamRuntime` now owns its `LogicalSession` and exposes read-only `session()` state;
- `open_uni` reacts to `StreamOpenError::Closed` as transport loss while preserving `Full(frame)` as nonterminal backpressure;
- `accept_one` reacts to `StreamAcceptError::Closed` as transport loss;
- an active admitted stream that reports `Cancelled` only terminates the logical session when the parent transport is actually closed; stream-local cancellation on an open transport remains stream-local;
- terminal transport loss cancels active inbound streams and all admission authority before closing the logical session;
- `apply_peer_revocation()` validates through the core session API, cancels stream/admission authority, closes transport, and completes `Revoked -> Closed`;
- `shutdown()` is idempotent and terminates stream authority, logical session, and transport.

Evidence:

- RED `0cb5d2e16a7454910eedba46862e083fa3fb68fc`, CI `34675990614` — lockfile/rustfmt passed and workspace check failed exactly on the old borrowed-session constructor plus missing `session()` / `apply_peer_revocation()` APIs;
- implementation `231542b548626df9a00c7a63e30702274009744c`, CI `34676120524` — stopped only at rustfmt on the revocation method signature;
- formatting-only final GREEN `08515dd3bb2a93476a2004ee3f6b6e6ec8ad85e9`, CI `34676159053` — locked metadata, rustfmt, workspace check, Clippy `-D warnings`, and all tests passed.

## Exact Next Task

Begin **M7 Task 4 — S-008 fresh reconnect and stale-authority rejection**.

Execution contract:

1. add `apps/sim/tests/lifecycle.rs` only; no new production API is expected;
2. build a deterministic fixture that creates fresh proofs from each pair's channel binding and supplied fresh nonces;
3. prove disconnect closes the old node and clears pending request state;
4. reconnect only through a new `MemoryTransportPair`, fresh binding, fresh nonces/proofs, and therefore a different `SessionId` with zeroed control sequences and no negotiated capabilities;
5. prove capabilities must be exchanged again on the new session;
6. inject an old-session control envelope into the new transport and require `ControlDispatchError::InvalidSession` plus fail-closed cleanup;
7. prove an `AuthorizedOperation` issued under the old session cannot authorize a stream on the new session (`OperationError::BindingMismatch`);
8. run focused lifecycle tests and full workspace gates before Task 5.

Do not add resumable-session authority, reconnect timers/backoff, or real networking.

## Remaining M7 Tasks

- Task 5 — S-009 active revocation + N-040/N-041 reconnect-after-revocation denial;
- Task 6 — fill only missing N-042..N-046 resource/cancellation/shutdown lifecycle assertions;
- Task 7 — scope review, final CI/fuzz, merge exact verified head to `main`, post-merge verification, final `CURRENT.md`, then M8 handoff.

## Resume Procedure

1. inspect `main`, `m7-failure-security-lifecycle`, PR #17, recent commits, and workflows;
2. read the Master Architecture, this file, M7 design/plan, and lifecycle contracts;
3. reconcile docs with actual code before editing;
4. execute one TDD task at a time and preserve exact RED/GREEN evidence;
5. merge only the exact verified final M7 head to `main`, verify canonical `main`, and verify the final integration record;
6. never rely on chat history or stash as the only copy of incomplete work.
