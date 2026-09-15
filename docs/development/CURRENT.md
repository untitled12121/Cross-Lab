# Current Development State

This file is the durable resume guide for active Cross-Lab development. Git/code/tests are factual state; this file records the intended handoff.

## Current Phase

**Phase 1 — Core Simulator / Real Transport Transition**

## Current Milestone

**M9 — Remote Networking ADR. Foundation authority/replay remediation and remote-networking Task 4 are complete, merged, and post-merge verified. Task 5 is next.**

M1–M8 are complete. M9 networking research Tasks 1–3 and implementation Task 4 are complete. ADR-0009 remains undecided.

## Canonical Baseline

- Current `main` before this documentation checkpoint: `3a403661ed02bcaf59f6cdfe45ad8425d7e2275a`.
- Task 4 merge: `3a403661ed02bcaf59f6cdfe45ad8425d7e2275a` via PR **#24 — M9 Task 4: bounded Iroh stream transport**.
- Task 4 verified cleanup head: `7b076587dec6de3da5cfc02f52284ac6692d0d20`.
- Task 4 cleanup-head Rust CI `35003226351` passed lockfile verification, dependency audit, rustfmt, full workspace check, Clippy with `-D warnings`, and complete workspace tests.
- Task 4 post-merge `main` Rust CI `35003931830` passed the same full gate.
- Foundation remediation merge: `450615a7c361384cfbe68bc8991b7ca03e4482fa` via PR #23.
- Foundation exact-head Rust CI `34974781151` and Fuzz Smoke `34974781112` passed; foundation post-merge Rust CI `34979066740` passed.
- `paste 1.0.15` / `RUSTSEC-2024-0436` remains a maintenance warning isolated to the Iroh experiment and must be re-evaluated before production networking promotion.

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

No Master Architecture revision, protobuf schema change, canonical transcript change, signature-format change, or Quinn channel-binding change was introduced by this remediation.

## Foundation Remediation Status

Tasks 1–8 are complete. Security assessment and whole-project audit are reconciled with the accepted ADR-0010/ADR-0011 implementation. PR #23 merged only after exact-head Rust CI and Fuzz Smoke passed, and the resulting `main` merge commit passed the full Rust CI gate.

## M9 Networking State

- Quinn remains the verified local/LAN baseline.
- Iroh `1.2.0` remains isolated under `experiments/m9-networking` until ADR-0009 selects the remote-networking architecture.
- M9 networking Tasks 1–4 are complete on `main`.
- Task 4 RED evidence: focused run `34993804521` failed as expected because the planned `candidate::connection` surface did not yet exist.
- Task 4 implementation head `08f45c8974de5213277249c946be4c12cb0d1f90`: focused run `35002480457` and full Rust CI `35002483529` passed.
- Task 4 final cleanup head `7b076587dec6de3da5cfc02f52284ac6692d0d20`: Rust CI `35003226351` passed.
- Task 4 merge commit `3a403661ed02bcaf59f6cdfe45ad8425d7e2275a`: post-merge Rust CI `35003931830` passed.
- `IrohTransportConnection` remains crate-private and implements only the existing `TransportConnection` semantic seam; no Iroh type is exposed through Cross-Lab domain/public APIs.
- Uni-stream opening/chunk limits, stream-slot and chunk-queue bounds, ordered delivery, clean FIN, reset/drop/cancel, connection close, and joined task shutdown are covered.
- ADR-0009 remains reserved for the final M9 remote-networking decision; Task 4 does not promote Iroh to production.
- Remote sessions remain `NetworkClass::Remote` for their lifetime.
- Iroh identity/address/path/relay/transport metadata must never become Cross-Lab identity/trust/policy authority.
- No 0-RTT authority.
- Route changes must not silently mutate channel binding, `SessionId`, sequence state, policy classification, or operation authority.
- A new transport connection requires fresh Cross-Lab authentication/authorization state.
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

Start **M9 remote-networking Task 5 — Existing Cross-Lab session authentication over Iroh** from `docs/plans/phase-1/M9-remote-networking.md` on a fresh branch from verified `main`.

Task 5 must:

1. reuse the existing Cross-Lab hello/proof/session activation protocol unchanged;
2. derive authority from Cross-Lab owner/device credentials and `TrustRecord`, never Iroh endpoint identity;
3. use the ADR-0008 exporter-derived channel binding already proven by the candidate;
4. keep `NetworkClass::Remote` fixed for the authenticated pair;
5. reject a proof bound to another connection before either session becomes active;
6. reject replay of an old proof after reconnect and require a fresh `SessionId`;
7. refuse promotion to `IrohTransportConnection` until both `LogicalSession`s are active;
8. execute RED -> verified failure -> minimal GREEN -> focused tests -> full workspace gate before integration.

## Resume Procedure

1. verify `main`, open PRs, and this file against actual repository state;
2. read the active M9 plan and relevant session/transport ADRs;
3. inspect existing Quinn session-auth code, the isolated Iroh candidate, and uploaded Iroh/Quinn research before Task 5 implementation;
4. create the Task 5 branch from current verified `main`;
5. execute Task 5 RED -> verified failure -> minimal GREEN -> focused tests -> full gate;
6. update this file after meaningful progress with exact commits, verification status, unfinished work, and the next task.
