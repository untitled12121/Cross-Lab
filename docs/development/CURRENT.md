# Current Development State

This file is the durable resume guide for active Cross-Lab development. Git/code/tests are factual implementation state; this file records the intended handoff.

## Current Phase

**Phase 1 — Core Simulator**

## Current Milestone

**M7 — Failure and Security Lifecycle: Tasks 1–6 GREEN; Task 7 final integration closeout in progress.**

M1–M6 are complete, verified, and integrated into canonical `main`.

## Branch State

- `main` — exact verified M6 integration-record head `d419f0fd410f0d2da69cc9bf2e4c20600033b7fb`; CI `34671802373` passed locked metadata, rustfmt, workspace check, Clippy `-D warnings`, and all tests.
- `m7-failure-security-lifecycle` — active M7 branch created exactly from that verified `main` head.
- PR #17 — draft, `feat: implement M7 failure and security lifecycle`; mark ready only after the exact documentation-inclusive final head passes CI.
- Current verified M7 production/test head: `f87c43fa4c5ed252fe30358406efbba3faba746f`; CI `34681410654` passed locked metadata, rustfmt, workspace check, Clippy `-D warnings`, and all tests.
- Fuzz Smoke is not scheduled for this PR because `.github/workflows/fuzz.yml` is path-filtered to `crates/protocol/**`, `crates/policy/**`, `fuzz/**`, or the workflow itself, and M7 changes none of those paths. This is recorded as not applicable to the M7 diff, not as a fuzz pass.

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
- accepted matching local revocation must match authenticated owner/device/credential epoch and advance the authenticated trust-revision snapshot;
- accepted matching local revocation cancels ordinary session-scoped authority and closes without peer acknowledgement;
- revoked trust denies later ordinary authentication;
- all simulator queues remain bounded and terminal paths deterministically cancel session-scoped authority;
- no real networking, Quinn/Iroh/libp2p, async runtime, reconnect timers/backoff, persistence, UI/platform, privileged, recovery-reauthorization, ticket, or 0-RTT scope enters M7.

No ADR is required because M7 implements the approved lifecycle contract without changing architecture.

## M7 Planning

- Design: `docs/plans/phase-1/M7-failure-security-lifecycle-design.md`, checkpoint `281a4f724d23f2435cee0e69c10c1b13dc758ec1`.
- Implementation plan: `docs/plans/phase-1/M7-failure-security-lifecycle.md`, checkpoint `f42e5a96aac4449bdf858869b114ae20bc08f401`.
- Research inspected: uploaded Quinn connection close/loss semantics; Syncthing reconnect orchestration; RustDesk transient reconnect heuristics. Reuse is semantic only.

## M7 Task 1 — Core Session Terminal Lifecycle

Delivered:

- focused `crates/core/tests/session_lifecycle.rs`;
- `LogicalSession::transport_lost()` with active/closing/revoked terminal closure and closed-state idempotence;
- `LogicalSession::apply_peer_revocation()` requiring an active session, local `TrustState::Revoked`, exact authenticated owner/device, exact credential epoch, and a trust revision strictly newer than the authenticated snapshot;
- typed `SessionError::PeerNotRevoked` and `SessionError::PeerTrustRevisionNotAdvanced` failures;
- trust-transition signature/issuer/revision validation stays in `crosslab-policy::TrustTransition`.

Evidence:

- initial RED `623f326efc4711b1991e7de24a6cd58c46d6a84c`, CI `34675402081` — missing lifecycle APIs only;
- initial GREEN `f763bd034d8ac15d44e11b4ef659d1d907c75126`, CI `34675522726` — all Rust gates passed;
- final-review revision-guard RED `ad0f5ee4cf33a22fa834e6bf45b0fae8923b6bc3`, CI `34681339461` — lockfile/rustfmt passed and workspace check failed exactly on the missing `PeerTrustRevisionNotAdvanced` contract;
- revision-guard GREEN `f87c43fa4c5ed252fe30358406efbba3faba746f`, CI `34681410654` — locked metadata, rustfmt, workspace check, Clippy `-D warnings`, and all tests passed.

## M7 Task 2 — Control Runtime Terminal Cleanup

Delivered:

- focused `apps/sim/tests/control_lifecycle.rs` using normal session authentication and real signed trust revocation;
- `ControlDispatcher::cancel_session_state()` clears pending outgoing, inbound request, and nonretryable replay bookkeeping for a dead session without resetting it for reuse;
- send-side `ControlSendError::Closed(frame)` and receive-side `ControlReceiveError::Closed` terminate the active logical session and clear session-scoped dispatcher authority;
- `ControlSendError::Full(frame)` remains nonterminal and does not commit dispatcher state;
- `SimNode::apply_peer_revocation()` validates through the core session API, clears dispatcher authority, closes transport, and completes `Revoked -> Closed`;
- `SimNode::shutdown()` is idempotent, clears session-scoped control state, terminates the logical session, and closes transport;
- wrong-device/unrevoked records return typed errors without terminating the unrelated active session.

Evidence:

- transport-loss RED `ec7682a852c813da605586dc66f47ba1540813d8`, CI `34675642797`;
- transport-loss GREEN `a959a58715ffc9270cfa7d2b290cc9161a6b0143`, CI `34675751139`;
- revocation/shutdown RED `57cddc7a883b199f7721fdd2ec5ee9d1df34e74b`, CI `34675821698`;
- final Task 2 GREEN `3b52f75ec2cef774225c814a3bd04d4072d5f29b`, CI `34675870576` — all Rust gates passed.

## M7 Task 3 — Stream Runtime Terminal Lifecycle

Delivered:

- `SimStreamRuntime` owns its `LogicalSession` instead of borrowing it, avoiding cloned authority/shared mutable session handles;
- stream open/accept closed-transport errors terminate session authority while `Full` remains nonterminal backpressure;
- stream-local cancellation remains local while parent-transport cancellation terminates session authority;
- transport loss/revocation/shutdown cancel active inbound streams and `StreamAdmission` authority before closing the logical session/transport;
- `apply_peer_revocation()` and `shutdown()` are deterministic and synchronous with no background runtime.

Evidence:

- RED `0cb5d2e16a7454910eedba46862e083fa3fb68fc`, CI `34675990614`;
- implementation `231542b548626df9a00c7a63e30702274009744c`, CI `34676120524` — rustfmt-only stop;
- final GREEN `08515dd3bb2a93476a2004ee3f6b6e6ec8ad85e9`, CI `34676159053` — all Rust gates passed.

## M7 Task 4 — S-008 Fresh Reconnect and Stale Authority Rejection

Delivered in `apps/sim/tests/lifecycle.rs` without adding a reconnect abstraction.

Coverage proves:

- disconnect closes the old logical session and clears pending control state;
- reconnect uses a distinct memory transport/binding plus fresh nonces/proofs and therefore a different `SessionId`;
- fresh send/receive sequences start at zero and negotiated capabilities start empty;
- capabilities must be exchanged again after fresh authentication;
- old-session control envelopes fail closed on the new session;
- old `AuthorizedOperation` authority is rejected on the new session with `OperationError::BindingMismatch`.

Evidence:

- initial acceptance `9eff524c0473f13a41578f4dc82c198ed6414779`, CI `34676322835` — rustfmt-only stop;
- final GREEN `29762639d6c6054aa3996b3429437e7c00469966`, CI `34676386066` — all Rust gates passed.

## M7 Task 5 — S-009 Active Revocation and Reconnect Denial

Coverage proves:

- a real owner-root-signed `TrustTransition` is verified/applied to local trust state before runtime reaction;
- accepted matching peer revocation terminates active control authority, pending requests, admitted data streams, stream/admission authority, transport, and logical session locally;
- a fresh transport/binding/nonces/proofs using the same still-cryptographically-valid credential is denied when local trust is revoked: `SessionError::PeerNotTrusted` and terminal closed authentication state;
- still-trusted and unrelated revoked records return typed failures without killing the intended active session.

Evidence:

- initial acceptance `7408ed5e0ca2258dcc0c208517bf06f87289d49b`, CI `34676515865` — rustfmt-only stop;
- final GREEN `f50eb7da92f783b863725994c79643aef675e939`, CI `34676614329` — all Rust gates passed.

## M7 Task 6 — Resource, Cancellation, Malformed Input, and Shutdown Closeout

Inventory confirmed existing coverage already satisfies:

- N-043 bounded stream queue/chunk saturation and backpressure;
- N-044 request cancellation state termination;
- N-045 stream setup/admission cancellation without dangling admitted state;
- N-046 idempotent control and stream runtime shutdown.

Added only the missing behavior:

- N-042 simulator-level bounded control backpressure proves `ControlSendError::Full` does not advance send sequence or commit pending-request bookkeeping and retry succeeds after capacity drains;
- malformed/fatal control input and peer graceful close now clear pending dispatcher authority before terminal session/transport close.

Evidence:

- Task 6 RED `58eb0a914a9ca929ff48ccc25e481d51f43b4efb`, CI `34676901018` — locked metadata, rustfmt, workspace check, and Clippy passed; tests failed exactly because malformed-input and peer-close paths retained one pending request while the session was terminal;
- Task 6 GREEN `4026cc397fe63a54e5cc093e341cd1f37ca721d3`, CI `34676968464` — locked metadata, rustfmt, workspace check, Clippy `-D warnings`, and all tests passed.

## M7 Final Scope Review

PR #17 changes exactly these 11 files:

```text
apps/sim/src/node.rs
apps/sim/src/stream.rs
apps/sim/tests/control_lifecycle.rs
apps/sim/tests/lifecycle.rs
apps/sim/tests/stream_scenarios.rs
crates/core/src/control/mod.rs
crates/core/src/session/state.rs
crates/core/tests/session_lifecycle.rs
docs/development/CURRENT.md
docs/plans/phase-1/M7-failure-security-lifecycle-design.md
docs/plans/phase-1/M7-failure-security-lifecycle.md
```

Review result:

- no transport-library/networking dependency or async runtime introduced;
- no policy/identity/protocol ownership boundary moved into simulator UI/runtime code;
- no stale authority is intentionally transferable across reconnect;
- no recovery, credential rotation, 0-RTT/tickets, reconnect timing/backoff, persistence, UI/platform, privileged, or M8 scope entered;
- no submitted PR reviews or unresolved review threads are present at this checkpoint.

## Exact Next Task

Complete **M7 Task 7 — final integration**:

1. require exact documentation-inclusive branch head CI to pass locked metadata, rustfmt, workspace check, Clippy `-D warnings`, and all tests;
2. update PR #17 body with final scope/evidence and record Fuzz Smoke as not applicable because its path filters exclude the M7 diff;
3. mark PR #17 ready only after that exact head is green;
4. merge exactly that verified PR head into `main` using the repository merge method, with expected-head protection;
5. require post-merge canonical `main` CI to pass;
6. update `CURRENT.md` on `main` with the M7 merge commit, post-merge verification, and **M8 — Quinn transport** as the exact next milestone;
7. verify the final documentation-only `main` head before starting M8.

Do not start M8 before M7 is merged and canonical `main` plus the final integration record are verified.

## Resume Procedure

1. inspect `main`, `m7-failure-security-lifecycle`, PR #17, recent commits, and workflows;
2. read the Master Architecture, this file, M7 design/plan, and lifecycle contracts;
3. reconcile docs with actual code before editing;
4. execute one TDD task at a time and preserve exact RED/GREEN evidence;
5. merge only the exact verified final M7 head to `main`, verify canonical `main`, and verify the final integration record;
6. never rely on chat history or stash as the only copy of incomplete work.
