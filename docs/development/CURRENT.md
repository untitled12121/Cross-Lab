# Current Development State

This file is the durable resume guide for active Cross-Lab development. Git/code/tests are factual implementation state; this file records the intended handoff.

## Current Phase

**Phase 1 — Core Simulator**

## Current Milestone

**M7 — Failure and Security Lifecycle: planning complete; Tasks 1–5 GREEN; Tasks 6–7 pending.**

M1–M6 are complete, verified, and integrated into canonical `main`.

## Branch State

- `main` — exact verified M6 integration-record head `d419f0fd410f0d2da69cc9bf2e4c20600033b7fb`; CI `34671802373` passed locked metadata, rustfmt, workspace check, Clippy `-D warnings`, and all tests.
- `m7-failure-security-lifecycle` — active M7 branch created exactly from that verified `main` head.
- PR #17 — draft, `feat: implement M7 failure and security lifecycle`; keep draft until the exact final documentation-inclusive head passes CI and Fuzz Smoke.
- Current verified M7 production/test head: `f50eb7da92f783b863725994c79643aef675e939`.

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

## M7 Task 4 — S-008 Fresh Reconnect and Stale Authority Rejection

Delivered in `apps/sim/tests/lifecycle.rs` only; no production reconnect abstraction was added.

Coverage proves:

- disconnect is terminal for the old logical session and clears pending control request state;
- reconnect constructs a distinct `MemoryTransportPair` with a different binding plus fresh nonces/proofs and therefore a different `SessionId`;
- the fresh session starts with send/receive sequence zero and no negotiated capabilities;
- capabilities return only after a new post-auth exchange on the fresh session;
- an old-session control envelope injected into the new transport is rejected as `ControlDispatchError::InvalidSession` and closes the contaminated new session;
- a separate fresh session rejects an old `AuthorizedOperation` with `OperationError::BindingMismatch`, proving operation authority does not transfer across `SessionId` boundaries.

Evidence:

- initial acceptance head `9eff524c0473f13a41578f4dc82c198ed6414779`, CI `34676322835` — stopped only at rustfmt;
- formatting-only final GREEN `29762639d6c6054aa3996b3429437e7c00469966`, CI `34676386066` — locked metadata, rustfmt, workspace check, Clippy `-D warnings`, and all tests passed.

## M7 Task 5 — S-009 Active Revocation and Reconnect Denial

Delivered in `apps/sim/tests/lifecycle.rs` only; trust verification/transition ownership remains in policy and runtime reaction remains in core/simulator.

Coverage proves:

- a real owner-root-signed `TrustTransition` is verified/applied to local trust state before ordinary runtime revocation reaction;
- accepted matching peer revocation terminates active control authority locally, clears pending request state, closes transport/session, and rejects later ordinary work;
- accepted matching peer revocation terminates an active admitted data stream, cancels stream/admission authority, closes transport/session, and removes the runtime stream;
- a completely fresh transport/binding/nonces/proofs using the same still-cryptographically-valid credential is denied when local peer trust is revoked: `SessionError::PeerNotTrusted`, fail-closed `SessionState::Closed`;
- a still-trusted unchanged record returns `SessionError::PeerNotRevoked` without killing the session;
- a revoked record for an unrelated device returns `SessionError::PeerTrustMismatch` without killing the intended session.

Evidence:

- initial acceptance head `7408ed5e0ca2258dcc0c208517bf06f87289d49b`, CI `34676515865` — locked metadata passed and CI stopped only at rustfmt;
- formatting-only final GREEN `f50eb7da92f783b863725994c79643aef675e939`, CI `34676614329` — locked metadata, rustfmt, workspace check, Clippy `-D warnings`, and all tests passed.

## Exact Next Task

Begin **M7 Task 6 — failure/resource/shutdown closeout without duplicate coverage**.

Execution contract:

1. inventory existing control/stream/memory-transport tests for replay, malformed input, sequence failures, duplicate requests, bounded saturation/backpressure, cancellation, stream setup failure, and shutdown before adding anything;
2. add only the missing control backpressure transaction proof: on `ControlSendError::Full`, send sequence and pending-request bookkeeping stay unchanged, session remains active, then retry succeeds after capacity drains;
3. strengthen fatal-input cleanup only where existing tests do not already prove closed session + cleared pending authority + closed transport;
4. strengthen idempotent control/stream shutdown assertions only where still missing;
5. do not duplicate parser boundary/fuzz coverage and do not edit memory transport tests unless the inventory finds a real missing transport invariant;
6. run locked metadata, rustfmt, workspace check, Clippy `-D warnings`, all workspace tests, then require exact-head GitHub CI and existing Fuzz Smoke.

No new production abstraction or parser is expected in Task 6.

## Remaining M7 Tasks

- Task 7 — scope review, final CI/fuzz, merge exact verified head to `main`, post-merge verification, final `CURRENT.md`, then M8 handoff.

## Resume Procedure

1. inspect `main`, `m7-failure-security-lifecycle`, PR #17, recent commits, and workflows;
2. read the Master Architecture, this file, M7 design/plan, and lifecycle contracts;
3. reconcile docs with actual code before editing;
4. execute one TDD task at a time and preserve exact RED/GREEN evidence;
5. merge only the exact verified final M7 head to `main`, verify canonical `main`, and verify the final integration record;
6. never rely on chat history or stash as the only copy of incomplete work.
