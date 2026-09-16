# Current Development State

This file is the durable resume guide for active Cross-Lab development. Git/code/tests are factual state; this file records the intended handoff.

## Current Phase

**Phase 1 — Core Simulator / Real Transport Transition**

## Current Milestone

**M9 — Remote Networking ADR. Foundation authority/replay remediation and remote-networking Tasks 1–6 are merged and post-merge verified. Task 7 implementation and invariant repair are complete on `m9-task7-owner-relay-path`; the repaired exact-head full gate is green and Task 7 is in final PR integration.**

M1–M8 are complete. ADR-0009 remains undecided/reserved; Iroh stays isolated under `experiments/m9-networking` until the M9 evidence gate selects a remote-networking architecture.

## Canonical Baseline

- Verified `main` documentation baseline before Task 7: `c4b78174121e5ea2aa9901f53a7bba58840e8e24`; Rust CI `35036981684` passed the full gate.
- Task 6 merge: `ec213a20176787c6da18217603e163b8259eae7e` via PR #26; post-merge Rust CI `35018281624` passed the full gate.
- Task 7 branch: `m9-task7-owner-relay-path`.
- Task 7 RED run `35037785170` failed for the intended missing `scenarios::relay` surface before relay orchestration existed.
- Task 7 GREEN run `35038289013` passed the focused relay tests and committed implementation `365932cd5a640cb763556dd69c5c9150ed8f5f06`.
- Task 7 widened semantic gate `35038546481` passed `relay`, `session`, and `lifecycle` on `18e31b2caa94688d10e0767f2527378777411c38`.
- Task 7 pre-PR full gate `35039448229` passed lockfile verification, dependency audit, rustfmt, full workspace check, Clippy with `-D warnings`, and complete workspace tests on `f565d08bbe54c039bb20b80a9aceb81bd2c86374`.
- Final requirements review found that the original Task 7 relay-to-direct evidence did not explicitly prove control-sequence continuity or survival of already-issued operation authority across the same live Iroh path change, despite both being M9 invariants.
- Task 7 invariant RED run `35040355796` failed for the intended missing scenario helpers after tests were added for sequence progression and pre-path-change operation use.
- Task 7 invariant GREEN run `35040628969` passed the focused `relay + session + lifecycle` gate and committed the repair as `d38a6d49afdee7aa7b455cdb3da517c5249e9853`.
- Task 7 repaired pre-PR full gate `35040810828` passed lockfile verification, dependency audit, rustfmt, `cargo check --workspace --all-targets --all-features`, Clippy with `-D warnings`, and complete workspace tests on `97e892dbd2554c4c17cc9cd9da888d16e8521a67`.
- The remaining Task 7 integration sequence is: remove all temporary Task 7 workflows, open the Task 7 PR, require normal full PR CI on the exact final head, review the final diff, merge, and verify `main` with the normal full gate.
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
- M9 networking Tasks 1–6 are complete on `main`; Task 7 implementation plus invariant repair are complete on its integration branch pending PR merge/post-merge verification.
- Task 4 established bounded Iroh control/uni-stream semantics behind the existing `TransportConnection` seam; no Iroh type entered Cross-Lab domain/public APIs.
- Task 5 reuses the existing Cross-Lab hello/proof/session activation protocol over Iroh with ADR-0008 `quic-tls-exporter-v1` binding after the full handshake. Iroh endpoint identity remains routing metadata only.
- Authenticated Iroh pairs are fixed to `NetworkClass::Remote` with no setter; no 0-RTT authority exists.
- Task 6 proves capability/control request-response-event flow, authorized operation-bound streams, unknown-operation rejection, fresh reconnect binding/`SessionId`, rejection of old proof/session/operation authority, signed revocation termination and reconnect denial, bounded saturation/cancellation/close/joined shutdown, and `Constraint::LocalOnly` rejection under immutable `NetworkClass::Remote`.
- Task 7 adds `iroh-relay 1.2.0` only to the isolated M9 experiment and runs a self-hosted relay bound locally; required tests do not depend on public n0 relay infrastructure.
- Task 7 proves an authenticated Cross-Lab session carries bounded control and unidirectional data through an owner-controlled relay with IP transports disabled.
- Task 7 proves a relay-assisted connection can observe a direct IP path without changing immutable `NetworkClass::Remote`, the authenticated ADR-0008 exporter binding, or the logical Cross-Lab `SessionId`.
- Task 7 now explicitly proves control `message_seq` continues across the live relay-to-direct path observation rather than resetting or creating a new session sequence domain.
- Task 7 now explicitly proves an operation grant issued before the live path change remains usable afterward on that same authenticated Iroh connection; the route observation does not mint, replace, or revoke authority.
- Task 7 path observation remains experiment orchestration only; it does not write policy, trust, operation, or session authority and does not promote Iroh into production/domain APIs.
- A new Iroh transport connection still requires fresh Cross-Lab authentication and authorization state; route changes within one live authenticated connection do not create new authority.
- Security authority remains local and fail-closed.

## Deferred / External Risks

- Durable rollback-resistant persistence for `OwnerAuthorityState` and accepted authority epochs.
- Platform key-store / secure-enclave integration and real mobile lifecycle evidence.
- Explicit authorization/subscription design for future authority-sensitive system-event families.
- `main` branch protection and other repository-administration controls.
- Windows/macOS/mobile CI matrices as their platform slices land.
- `paste 1.0.15` / `RUSTSEC-2024-0436` in the isolated Iroh experiment, to re-evaluate before ADR-0009 production promotion.
- Reproducible benchmark evidence, controlled Linux NAT/relay topology evidence, and recovery/resource measurements required before ADR-0009 can select a production remote-connectivity architecture.

## Exact Next Task

Finish **M9 remote-networking Task 7 integration**: remove all temporary Task 7 workflows from the branch, open the Task 7 PR, require normal full PR CI on the exact final head, review the final diff, merge only when exact-head CI is green, and verify the merge commit on `main` with the normal full gate.

After the Task 7 merge is post-merge green, update this file on `main` and start **M9 remote-networking Task 8 — Reproducible benchmarks and controlled Linux NAT/relay gate** from `docs/plans/phase-1/M9-remote-networking.md` on a fresh feature branch.

Task 8 must keep measurements reproducible and authority-neutral: typed local benchmark modes, safe rendezvous metadata only, no secret/exporter/proof persistence, owner-controlled relay, deterministic namespace cleanup, explicit relay-required then direct-path topology evidence, bounded lifecycle/resource accounting, and no production Iroh promotion before ADR-0009.

## Resume Procedure

1. verify branch head `97e892dbd2554c4c17cc9cd9da888d16e8521a67`, this checkpoint, the active M9 plan, and successful full gate `35040810828`;
2. remove `.github/workflows/m9-task7-focused.yml`, `.github/workflows/m9-task7-full-temp.yml`, and `.github/workflows/m9-task7-green-repair.yml` so no temporary Task 7 workflow reaches `main`;
3. open the Task 7 PR and require the normal `CI` workflow on the exact final head;
4. review the final changed-file set for experiment/lockfile/CURRENT scope only and confirm no temporary workflow remains;
5. merge Task 7 only if the exact final head is green, then verify the `main` post-merge full CI gate;
6. update `CURRENT.md` on `main` with Task 7 merge/post-merge evidence and Task 8 as the exact next task;
7. create a fresh Task 8 feature branch from that verified documentation checkpoint and follow RED -> intended failure -> minimal GREEN -> deterministic full gate -> controlled evidence -> integration.
