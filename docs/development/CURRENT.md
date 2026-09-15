# Current Development State

This file is the durable resume guide for active Cross-Lab development. Git/code/tests are factual state; this file records the intended handoff.

## Current Phase

**Phase 1 — Core Simulator / Real Transport Transition**

## Current Milestone

**M9 — Remote Networking ADR. Foundation authority/replay remediation and remote-networking Tasks 1–4 are merged and post-merge verified. Task 5 implementation is complete and verified on PR #25; integration and post-merge verification are next.**

M1–M8 are complete. ADR-0009 remains undecided; Iroh stays isolated under `experiments/m9-networking` until the M9 evidence gate selects a remote-networking architecture.

## Canonical Baseline

- Task 4 verified `main` documentation baseline: `3560d7ec186b8a9af4a860b4faca4d37a2c6f9b3`.
- Task 4 merge: `3a403661ed02bcaf59f6cdfe45ad8425d7e2275a` via PR **#24 — M9 Task 4: bounded Iroh stream transport**.
- Task 4 post-merge Rust CI `35003931830` passed lockfile verification, dependency audit, rustfmt, full workspace check, Clippy with `-D warnings`, and complete workspace tests.
- Task 5 branch: `m9-task5-iroh-session-auth`, PR **#25 — M9 Task 5: authenticate Cross-Lab sessions over Iroh**.
- Task 5 RED commit `b899aa7f1bc8314082ca1697abfdbac9b9d840b2`; focused run `35006163082` failed for the intended missing `scenarios::auth` surface.
- Task 5 implementation commit `747009e000f14ec13309e234fe21528bbac9f9f9`; focused run `35006844765` passed.
- Task 5 formatter head `c9a2b4dcc7ce4029a70283f4e26d1740cc306362`; focused run `35007473194` passed.
- Task 5 large-error Clippy remediation commit `8b5504a1be409beba78bec94d5f3bba023012efe`; focused fix gate `35009506284` passed session tests, rustfmt, and workspace Clippy with `-D warnings`.
- Task 5 verified cleanup head before this documentation checkpoint: `517c5afc3d4b2b84af16d2ea8928711457863cfd`.
- Task 5 cleanup-head Rust CI `35009906786` passed lockfile verification, dependency audit, rustfmt, full workspace check, Clippy with `-D warnings`, and complete workspace tests.
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
- M9 networking Tasks 1–4 are complete on `main`; Task 5 is implementation-complete and verified on PR #25.
- Task 4 established bounded Iroh control/uni-stream semantics behind the existing `TransportConnection` seam; no Iroh type entered Cross-Lab domain/public APIs.
- Task 5 reuses the existing Cross-Lab hello/proof/session activation protocol over the Iroh candidate without changing wire/schema/signature formats.
- Task 5 authority comes from `OwnerAuthorityState`, `DeviceCredential`, and `TrustRecord`; Iroh endpoint identity is routing metadata only.
- Task 5 uses the existing ADR-0008 exporter-derived `quic-tls-exporter-v1` channel binding after the full Iroh handshake.
- The reserved bidirectional stream is promoted into `IrohTransportConnection` only after both `LogicalSession`s are Active.
- Authenticated Iroh pairs are fixed to `NetworkClass::Remote` with no setter.
- Focused tests prove trusted activation, wrong-binding rejection before Active, old-proof rejection after reconnect with a fresh `SessionId`, and refusal to promote before both sessions are Active.
- Temporary Task 5 CI workflows were removed from the final diff.
- ADR-0009 remains reserved for the final evidence-backed M9 decision; Task 5 does not promote Iroh to production.
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

Finish **M9 remote-networking Task 5 integration**, then start **Task 6 — Direct-Iroh control/data/reconnect/revocation semantics** from `docs/plans/phase-1/M9-remote-networking.md`.

Integration sequence:

1. require exact-head full Rust CI on this Task 5 documentation checkpoint;
2. review the final PR #25 diff for experiment-only scope and no unintended schema/canonical-vector changes;
3. mark PR #25 ready and merge only while its expected head is unchanged;
4. require the resulting `main` merge commit to pass the full Rust CI gate;
5. checkpoint `CURRENT.md` on verified `main` and branch Task 6 from that durable baseline.

Task 6 must reuse the existing `AuthenticatedIrohPair`, `SimNode`, `SimStreamRuntime`, policy/trust/operation APIs and add orchestration only. Its RED coverage must include capability advertisement and request/response/event, operation-bound uni streams, unknown-operation rejection, fresh reconnect binding/`SessionId`, rejection of old proof/session/operation authority, signed revocation and reconnect denial, saturation/cancellation/shutdown, and the `LocalOnly` policy regression under fixed `NetworkClass::Remote`.

## Resume Procedure

1. verify PR #25 head, exact-head CI, open PRs, and this file against actual repository state;
2. if PR #25 exact-head CI is green, review the final diff, mark it ready, and merge with expected-head protection;
3. verify the resulting `main` merge commit with the full Rust CI gate;
4. update this file on verified `main` with the Task 5 merge/post-merge run and exact Task 6 handoff;
5. create a fresh Task 6 feature branch from that verified documentation baseline;
6. execute Task 6 RED -> verified failure -> minimal orchestration GREEN -> focused lifecycle tests -> full workspace gate -> integration.
