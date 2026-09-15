# Current Development State

This file is the durable resume guide for active Cross-Lab development. Git/code/tests are factual state; this file records the intended handoff.

## Current Phase

**Phase 1 — Core Simulator / Real Transport Transition**

## Current Milestone

**M9 — Remote Networking ADR. Foundation authority/replay remediation and remote-networking Tasks 1–5 are merged and post-merge verified. Task 6 implementation is complete on PR #26 and is in final integration verification.**

M1–M8 are complete. ADR-0009 remains undecided; Iroh stays isolated under `experiments/m9-networking` until the M9 evidence gate selects a remote-networking architecture.

## Canonical Baseline

- Verified `main` documentation baseline before Task 6: `aa49adf0de669d028be476e9210b5499d1fcbb63`.
- Task 5 merge: `80118866d38525a33ee2f7cae85c2d82c9037f0d` via PR #25; post-merge Rust CI `35011352926` passed the full gate.
- Task 6 branch: `m9-task6-iroh-lifecycle` via PR **#26 — M9 Task 6: prove direct Iroh lifecycle semantics**.
- Task 6 RED run `35014990427` failed for the intended missing `scenarios::lifecycle` surface before lifecycle orchestration existed.
- Task 6 GREEN gate `35015706320` applied the minimal orchestration/auth-fixture changes, formatted them, and passed `cargo test -p crosslab-m9-networking --test lifecycle` before committing implementation `ab5fac2720a594b2fe6cd3411a35f51e0287b06f`.
- Initial full PR CI `35015978881` passed lockfile/audit/rustfmt/workspace-check and failed only because an unused crate-private Task 6 auth accessor was denied by Clippy `-D warnings`.
- Minimal fix `d17aec0c80e434cc6e324bd68a20a4df83fbb615` removes only that unused accessor.
- Focused run `35016582879` passed the lifecycle suite on `d17aec0c80e434cc6e324bd68a20a4df83fbb615`.
- Full PR CI `35016696594` passed lockfile verification, dependency audit, rustfmt, workspace check, Clippy with `-D warnings`, and complete workspace tests on `d17aec0c80e434cc6e324bd68a20a4df83fbb615`.
- This checkpoint removes the temporary Task 6 focused workflow. The resulting exact PR head must receive the normal full CI gate before PR #26 is merged.
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
- Task 5 reuses the existing Cross-Lab hello/proof/session activation protocol over Iroh with ADR-0008 `quic-tls-exporter-v1` binding after the full handshake. Iroh endpoint identity remains routing metadata only.
- Authenticated Iroh pairs are fixed to `NetworkClass::Remote` with no setter; no 0-RTT authority exists.
- Task 6 adds experiment-only lifecycle orchestration over existing `SimNode`, `SimStreamRuntime`, policy, trust, operation, and transport seams; it introduces no new Cross-Lab domain API, wire/schema/signature change, or production transport promotion.
- Task 6 proves capability advertisement and control request/response/event flow over authenticated Iroh.
- Task 6 proves authorized operation-bound unidirectional streams and rejection of unknown operations.
- Task 6 proves reconnect creates a fresh exporter binding and `SessionId`, while an old proof, old session envelope, and old operation authority do not carry into the new connection.
- Task 6 proves accepted signed peer revocation terminates stream/session authority and reconnect using revoked trust is denied.
- Task 6 proves bounded stream saturation, cancellation, transport close, and joined shutdown behavior.
- Task 6 proves `Constraint::LocalOnly` fails under the pair's immutable `NetworkClass::Remote`.
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

Finish **Task 6 integration**: require normal full PR CI on this checkpoint head, review the final PR #26 diff for unintended files, merge only if exact-head CI is green, and verify the merge commit on `main` with the normal full gate.

After the Task 6 merge is post-merge green, update this file on `main` and start **M9 remote-networking Task 7 — Owner-controlled relay and path-change invariants** from `docs/plans/phase-1/M9-remote-networking.md` on a fresh feature branch.

Task 7 must keep owner control and the existing security model: use a self-hosted Iroh relay only in the isolated experiment, retain immutable `NetworkClass::Remote`, preserve the authenticated binding and logical session across relay-to-direct path changes, and do not rely on public relay infrastructure for required tests.

## Resume Procedure

1. verify PR #26 head, exact-head CI, this file, and the active M9 plan against repository state;
2. confirm the Task 6 diff contains only experiment lifecycle/auth orchestration/tests plus this checkpoint and no temporary workflow;
3. merge PR #26 only after exact-head full CI is green;
4. wait for and verify the `main` post-merge full CI gate;
5. update `CURRENT.md` on `main` with the Task 6 merge/post-merge evidence and Task 7 as the exact next task;
6. branch Task 7 from that verified documentation checkpoint and follow RED -> intended failure -> minimal GREEN -> focused gate -> full gate -> integration.
