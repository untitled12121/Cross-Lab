# Current Development State

This file is the durable resume guide for active Cross-Lab development. Git/code/tests are factual state; this file records the intended handoff.

## Current Phase

**Phase 1 — Core Simulator / Real Transport Transition**

## Current Milestone

**M9 — Remote Networking ADR. Foundation authority/replay remediation and remote-networking Tasks 1–6 are merged and post-merge verified. Task 7 is next.**

M1–M8 are complete. ADR-0009 remains undecided/reserved; Iroh stays isolated under `experiments/m9-networking` until the M9 evidence gate selects a remote-networking architecture.

## Canonical Baseline

- Current verified code baseline: `ec213a20176787c6da18217603e163b8259eae7e` on `main` via PR **#26 — M9 Task 6: prove direct Iroh lifecycle semantics**.
- Task 6 final PR head: `5bfa734fa2783fdc0bd50af036bc65e6e6f7fe57`.
- Task 6 exact-head Rust CI `35017602746` passed lockfile verification, dependency audit, rustfmt, full workspace check, Clippy with `-D warnings`, and complete workspace tests.
- Task 6 merge: `ec213a20176787c6da18217603e163b8259eae7e`.
- Task 6 post-merge Rust CI `35018281624` passed the same full gate.
- Task 6 RED run `35014990427` failed for the intended missing `scenarios::lifecycle` surface.
- Task 6 GREEN focused gate `35015706320` passed before the implementation checkpoint.
- Initial full CI `35015978881` exposed one unused crate-private accessor under Clippy; minimal fix `d17aec0c80e434cc6e324bd68a20a4df83fbb615` removed only that dead accessor.
- Focused repair run `35016582879` and full repaired-head CI `35016696594` passed.
- Task 5 merge: `80118866d38525a33ee2f7cae85c2d82c9037f0d` via PR #25; post-merge Rust CI `35011352926` passed.
- Task 4 merge: `3a403661ed02bcaf59f6cdfe45ad8425d7e2275a` via PR #24; post-merge Rust CI `35003931830` passed.
- Foundation remediation merge: `450615a7c361384cfbe68bc8991b7ca03e4482fa` via PR #23; exact-head Rust CI `34974781151`, Fuzz Smoke `34974781112`, and post-merge Rust CI `34979066740` passed.
- `paste 1.0.15` / `RUSTSEC-2024-0436` remains the sole allowed maintenance warning, isolated to the Iroh experiment and requiring re-evaluation before production networking promotion.

## Accepted Foundation State

Accepted artifacts:

- `docs/adr/ADR-0010-authoritative-owner-authority-currentness.md`
- `docs/adr/ADR-0011-bounded-request-replay-semantics.md`
- `docs/superpowers/specs/2026-09-14-foundation-currentness-replay-design.md`
- `docs/superpowers/plans/2026-09-14-foundation-authority-replay-remediation.md`

The completed remediation establishes:

- identity-owned `OwnerAuthorityState` as the local source of truth for active owner root and delegated-role currentness;
- current Device Signing authority for credential issuance, verification, rotation, pairing, trust transitions, and session authentication;
- current Administrative authority for owner approval evidence;
- root or Device Signing replacement invalidates ordinary sessions and cancels session-scoped control/stream authority before close;
- Administrative/Recovery-only rotation does not invalidate ordinary sessions;
- `(SessionId, message_seq)` remains exact-envelope replay/order protection;
- `RequestId` remains bounded correlation/duplicate/retry history, and aged-out IDs are new authenticated attempts subject to current authorization;
- high-level stream admission derives trust/policy currentness from local `TrustRecord` and `PolicyState` rather than caller-selected revisions;
- obsolete caller-selected high-level authority/currentness APIs are removed from ordinary public paths;
- identity, policy, and protocol golden/wire vectors remain unchanged.

No Master Architecture revision, protobuf schema change, canonical transcript change, signature-format change, or Quinn channel-binding change was introduced by the remediation.

## M9 Networking State

- Quinn remains the verified local/LAN baseline.
- Iroh `1.2.0` remains isolated under `experiments/m9-networking` until ADR-0009 selects the remote-networking architecture.
- M9 networking Tasks 1–6 are complete on `main`.
- Task 4 established bounded Iroh control/uni-stream semantics behind the existing `TransportConnection` seam; no Iroh type entered Cross-Lab domain/public APIs.
- Task 5 reuses the existing Cross-Lab hello/proof/session activation protocol over Iroh with ADR-0008 `quic-tls-exporter-v1` binding after the full handshake. Iroh endpoint identity remains routing metadata only.
- Authenticated Iroh pairs are fixed to `NetworkClass::Remote` with no setter; no 0-RTT authority exists.
- Task 6 adds experiment-only lifecycle orchestration over existing `SimNode`, `SimStreamRuntime`, policy, trust, operation, and transport seams; it introduces no new Cross-Lab domain API, wire/schema/signature change, dependency change, or production transport promotion.
- Task 6 proves capability/control request-response-event flow, authorized operation-bound streams, unknown-operation rejection, fresh reconnect binding/`SessionId`, rejection of old proof/session/operation authority, signed revocation termination and reconnect denial, bounded saturation/cancellation/close/joined shutdown, and `Constraint::LocalOnly` rejection under immutable `NetworkClass::Remote`.
- Route changes must not silently mutate channel binding, `SessionId`, sequence state, policy classification, or operation authority. A new transport connection requires fresh Cross-Lab authentication and authorization state.
- Security authority remains local and fail-closed.

## Deferred / External Risks

- Durable rollback-resistant persistence for `OwnerAuthorityState` and accepted authority epochs.
- Platform key-store / secure-enclave integration and real mobile lifecycle evidence.
- Explicit authorization/subscription design for future authority-sensitive system-event families.
- `main` branch protection and other repository-administration controls.
- Windows/macOS/mobile CI matrices as their platform slices land.
- `paste 1.0.15` / `RUSTSEC-2024-0436` in the isolated Iroh experiment, to re-evaluate before ADR-0009 production promotion.
- Remote owner-relay, NAT/path-change, benchmark, and controlled topology evidence required before ADR-0009 can select a production remote-connectivity architecture.

## Exact Next Task

Start **M9 remote-networking Task 7 — Owner-controlled relay and path-change invariants** from `docs/plans/phase-1/M9-remote-networking.md` on a fresh feature branch from this checkpoint.

Task 7 must keep owner control and the existing security model: add the self-hosted Iroh relay dependency only to the isolated experiment; use no public relay infrastructure for required tests; retain immutable `NetworkClass::Remote`; preserve the authenticated exporter binding and logical `SessionId` across relay-to-direct path changes; and keep relay/path observation outside domain authorization state.

Iroh 1.2.0 exposes the required self-hosted relay and path-observation surfaces. Because `iroh_relay::server::ServerConfig` is non-exhaustive, construct it with `ServerConfig::default()` and assign public fields rather than using an external struct literal. This is an implementation compatibility detail, not an architecture change.

Required execution order: RED relay/path tests -> verify intended failure -> minimal owner-relay/path orchestration -> focused relay/session/lifecycle gate -> rustfmt/check/Clippy/full workspace tests -> `CURRENT.md` checkpoint -> PR integration -> post-merge verification.

## Resume Procedure

1. verify `main`, open PRs, this file, the active M9 plan, Master Architecture, and relevant session/transport ADRs against the repository;
2. create a fresh Task 7 feature branch from this documentation checkpoint;
3. inspect the current Iroh candidate endpoint/auth/lifecycle code and the uploaded/local Iroh reference before implementing relay behavior;
4. add the experiment-only relay dependency and write RED tests for owner-relay-only authenticated control/data plus relay-to-direct invariants;
5. verify RED fails for the intended missing relay surface;
6. implement the smallest self-hosted relay lifecycle and path observation required by the plan, with no public relay fallback;
7. run the focused Task 7 gate, then the full workspace gate before integration.
