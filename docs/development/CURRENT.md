# Current Development State

This file is the durable resume guide for active Cross-Lab development. Git/code/tests are factual state; this file records the intended handoff.

## Current Phase

**Phase 1 — Core Simulator / Real Transport Transition**

## Current Milestone

**M9 — Remote Networking ADR. Foundation authority/replay remediation and remote-networking Tasks 1–5 are merged and post-merge verified. Task 6 is next.**

M1–M8 are complete. ADR-0009 remains undecided; Iroh stays isolated under `experiments/m9-networking` until the M9 evidence gate selects a remote-networking architecture.

## Canonical Baseline

- Current verified code baseline: `80118866d38525a33ee2f7cae85c2d82c9037f0d` on `main`.
- Task 5 PR head: `8b1b12f1ca1559ba2da99e4ddc8e3fa443ca00bf` via PR **#25 — M9 Task 5: authenticate Cross-Lab sessions over Iroh**.
- Task 5 exact-head Rust CI `35010576966` passed lockfile verification, dependency audit, rustfmt, full workspace check, Clippy with `-D warnings`, and complete workspace tests.
- Task 5 merge: `80118866d38525a33ee2f7cae85c2d82c9037f0d`.
- Task 5 post-merge Rust CI `35011352926` passed the same full gate.
- Task 5 RED commit `b899aa7f1bc8314082ca1697abfdbac9b9d840b2`; focused run `35006163082` failed for the intended missing `scenarios::auth` surface.
- Task 5 implementation commit `747009e000f14ec13309e234fe21528bbac9f9f9`; focused run `35006844765` passed.
- Task 5 formatter head `c9a2b4dcc7ce4029a70283f4e26d1740cc306362`; focused run `35007473194` passed.
- Task 5 large-error Clippy remediation commit `8b5504a1be409beba78bec94d5f3bba023012efe`; focused fix gate `35009506284` passed session tests, rustfmt, and workspace Clippy with `-D warnings`.
- Task 4 merge: `3a403661ed02bcaf59f6cdfe45ad8425d7e2275a` via PR **#24 — M9 Task 4: bounded Iroh stream transport**; post-merge Rust CI `35003931830` passed.
- Foundation remediation merge: `450615a7c361384cfbe68bc8991b7ca03e4482fa` via PR #23; exact-head Rust CI `34974781151`, Fuzz Smoke `34974781112`, and post-merge Rust CI `34979066740` passed.
- `paste 1.0.15` / `RUSTSEC-2024-0436` remains an allowed maintenance warning isolated to the Iroh experiment and must be re-evaluated before production networking promotion.

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
- M9 networking Tasks 1–5 are complete on `main`.
- Task 4 established bounded Iroh control/uni-stream semantics behind the existing `TransportConnection` seam; no Iroh type entered Cross-Lab domain/public APIs.
- Task 5 reuses the existing Cross-Lab hello/proof/session activation protocol over the Iroh candidate without changing wire/schema/signature formats.
- Task 5 authority comes from `OwnerAuthorityState`, `DeviceCredential`, and `TrustRecord`; Iroh endpoint identity is routing metadata only.
- Task 5 uses the existing ADR-0008 exporter-derived `quic-tls-exporter-v1` channel binding after the full Iroh handshake.
- The reserved bidirectional stream is promoted into `IrohTransportConnection` only after both `LogicalSession`s are Active.
- Authenticated Iroh pairs are fixed to `NetworkClass::Remote` with no setter.
- Task 5 tests prove trusted activation, wrong-binding rejection before Active, old-proof rejection after reconnect with a fresh `SessionId`, and refusal to promote before both sessions are Active.
- No 0-RTT authority.
- Route changes must not silently mutate channel binding, `SessionId`, sequence state, policy classification, or operation authority.
- A new transport connection requires fresh Cross-Lab authentication and authorization state.
- Security authority remains local and fail-closed.

## Deferred / External Risks

- Durable rollback-resistant persistence for `OwnerAuthorityState` and accepted authority epochs.
- Platform key-store / secure-enclave integration and real mobile lifecycle evidence.
- Explicit authorization/subscription design for future authority-sensitive system-event families.
- `main` branch protection and other repository-administration controls.
- Windows/macOS/mobile CI matrices as their platform slices land.
- `paste 1.0.15` / `RUSTSEC-2024-0436` in the isolated Iroh experiment, to re-evaluate before ADR-0009 production promotion.
- Remote NAT/relay/path/measurement evidence required before ADR-0009 can select a production remote-connectivity architecture.

## Exact Next Task

Start **M9 remote-networking Task 6 — Direct-Iroh control/data/reconnect/revocation semantics** from `docs/plans/phase-1/M9-remote-networking.md` on a fresh feature branch from this documentation checkpoint.

Task 6 must reuse the existing `AuthenticatedIrohPair`, `SimNode`, `SimStreamRuntime`, policy/trust/operation APIs and add orchestration only. Do not add Iroh-specific policy exceptions or duplicate domain lifecycle logic.

Required RED coverage:

1. capability advertisement plus request/response/event over the authenticated Iroh pair;
2. authorized operation-bound uni stream and unknown-operation rejection;
3. reconnect changes exporter binding and `SessionId`;
4. old proof, old session envelope, and old operation authority fail on the new connection;
5. accepted signed revocation terminates authority and reconnect-after-revocation is denied;
6. bounded saturation, cancellation, connection-close, and joined shutdown behavior remains intact;
7. `Constraint::LocalOnly` fails under the pair's immutable `NetworkClass::Remote`.

Execution order is RED test -> verify intended failure -> minimal orchestration GREEN -> focused lifecycle tests -> rustfmt/check/Clippy/full workspace tests -> `CURRENT.md` checkpoint -> PR integration -> post-merge verification.

## Resume Procedure

1. verify `main`, open PRs, this file, the active M9 plan, and relevant session/transport ADRs against the repository;
2. create the fresh Task 6 branch from this checkpoint;
3. inspect `AuthenticatedIrohPair`, existing simulator lifecycle tests, and Quinn lifecycle evidence for reusable orchestration patterns;
4. write `experiments/m9-networking/tests/lifecycle.rs` RED coverage before adding `src/scenarios/lifecycle.rs`;
5. verify RED fails for the intended missing lifecycle surface;
6. implement only the smallest orchestration needed to reuse existing domain behavior;
7. run the focused Task 6 gate, then the full workspace gate before integration.
