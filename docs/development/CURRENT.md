# Current Development State

This file is the durable resume guide for active Cross-Lab development. Git/code/tests are factual implementation state; this file records the intended handoff.

## Current Phase

**Phase 1 — Core Simulator / Real Transport Transition**

## Current Milestone

**M8 — Quinn Transport: design checkpoint written; awaiting design review before implementation planning and production code.**

M1–M7 are complete and integrated into canonical `main`. M8 work is isolated on `m8-quinn-transport` from the exact verified post-M7 documentation head.

## Canonical Main State

- M7 verified branch head: `2d7ecebb7c785f1dfaaf825c0567ac78e1f7de71`.
- Final branch CI: `34681524693` — locked metadata, rustfmt, workspace check, Clippy `-D warnings`, and all tests passed.
- PR #17: `feat: implement M7 failure and security lifecycle` — merged with exact-head protection using merge method `merge`.
- M7 merge commit on `main`: `2974108dcf48a9d606b094fea24a9e5da991c513`.
- Post-merge canonical `main` CI: `34681576836` — locked metadata, rustfmt, workspace check, Clippy `-D warnings`, and all tests passed.
- Final documentation-only canonical `main` head: `b2e16f27a5c6d8307c58f4c6a4c760bb867ff223` (`docs: record M7 integration`).
- Final documentation-only `main` CI: `34681661930` — passed.
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

## M8 Design Checkpoint

Branch:

- `m8-quinn-transport`, created from exact verified `main` head `b2e16f27a5c6d8307c58f4c6a4c760bb867ff223`.

Written design:

- `docs/plans/phase-1/M8-quinn-transport-design.md`.
- Design commit: `9ef3e88886290ad2b43a3b0ae2fba0e74cefa338`.

Proposed design decisions:

- add concrete `crosslab-transport-quic` under `transports/quic`; Quinn/Tokio/rustls types remain private to the adapter;
- preserve the existing runtime-neutral `TransportConnection`/stream seam and bridge Quinn async I/O with bounded Tokio channels plus connection-owned tasks;
- use one reserved QUIC bidirectional stream for bounded session-auth bootstrap and, only after authentication, ordinary ordered control frames;
- map Cross-Lab unidirectional data streams to Quinn unidirectional streams with bounded transport-private record framing;
- derive `ChannelBinding` from Quinn TLS exporter material after the full handshake, proposed profile `quic-tls-exporter-v1`; no 0-RTT path;
- keep TLS certificate trust separate from Cross-Lab device identity; loopback tests may use explicitly trusted ephemeral self-signed certificates;
- do not add datagrams, Iroh, libp2p, NAT/relay, discovery, route scoring, transport migration, persistence, UI/platform, or privileged scope;
- strengthen the transport-neutral seam with ownership-preserving `TooLarge` errors for outbound control/opening frames before real network buffers are queued;
- keep all queues, stream concurrency, record lengths, receive windows, and idle behavior explicitly bounded.

Verified current dependency/API research for the design:

- Quinn `0.11.11` with minimal `runtime-tokio` + `rustls-ring` features;
- Tokio `1.53.1` as the one async runtime for the adapter/application boundary;
- Quinn's compatible rustls `0.23.x` stack; avoid a direct rustls dependency unless concrete configuration APIs require it;
- `rcgen 0.14.10` proposed dev-only for loopback certificate fixtures;
- Quinn supports TLS exporter keying material, uni/bi stream open/accept, explicit connection close/closed state, send reset/finish, receive stop, and explicit transport stream/window limits required by M8.

Research limitation:

- the uploaded Quinn archive could not be reliably enumerated in the current execution environment after repeated archive-tool failures. The design was cross-checked against maintained upstream Quinn 0.11.11 source/API documentation. Before production implementation, inspect/reconcile the uploaded archive in a normal local checkout if available; do not copy architecture blindly.

No production code or dependency changes have entered M8 yet.

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

Review and approve `docs/plans/phase-1/M8-quinn-transport-design.md`.

After approval, before production code:

1. write proposed `docs/adr/ADR-0008-quinn-channel-binding-profile-v1.md` recording the exact TLS-exporter label/context/output profile and full-handshake/no-0-RTT rule;
2. write `docs/plans/phase-1/M8-quinn-transport.md` as the detailed TDD implementation plan;
3. self-review the plan for complete design coverage, no placeholders, and type/interface consistency;
4. checkpoint/push the planning documents on `m8-quinn-transport`;
5. only then begin the first RED production slice from the approved plan.

Do not start M8 production code until the design is approved and the implementation plan is written and reconciled with current code.

## Resume Procedure

1. inspect canonical `main`, current M8 branch, recent commits/workflows, and `docs/development/CURRENT.md`;
2. read the Master Architecture, M8 design, relevant ADRs/contracts, and the implementation plan once approved;
3. inspect relevant uploaded research repositories and reconcile them with the pinned upstream APIs;
4. reconcile documentation with actual code before editing;
5. execute small TDD milestones, verify full relevant gates, commit/push, and update this file after meaningful progress;
6. merge only the exact verified M8 milestone head into `main` and verify canonical `main` after merge;
7. never rely on chat history or stash as the only copy of incomplete work.
