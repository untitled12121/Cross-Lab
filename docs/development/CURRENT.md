# Current Development State

This file is the durable resume guide for active Cross-Lab development. Git/code/tests are factual state; this file records the intended handoff.

## Current Phase

**Phase 1 — Core Simulator / Real Transport Transition**

## Current Milestone

**M9 — Remote Networking ADR, paused before networking Task 4 until the second foundation security remediation is merged and post-merge verified.**

M1–M8 are complete. M9 networking research Tasks 1–3 are complete. PR #23 contains the accepted ADR-0010/ADR-0011 foundation remediation and is in its final exact-head verification gate.

## Canonical Baseline

- `main`: `1426e70c6db169a37a89dbae0565a36a619f2651`, merging accepted architecture PR #22.
- Post-merge Rust CI `34826806366` passed dependency audit, rustfmt, workspace check, Clippy with `-D warnings`, and complete workspace tests.
- `paste 1.0.15` / `RUSTSEC-2024-0436` remains an isolated Iroh-experiment maintenance warning and must be re-evaluated before production networking promotion.

## Accepted Architecture

Accepted artifacts:

- `docs/adr/ADR-0010-authoritative-owner-authority-currentness.md`
- `docs/adr/ADR-0011-bounded-request-replay-semantics.md`
- `docs/superpowers/specs/2026-09-14-foundation-currentness-replay-design.md`
- `docs/superpowers/plans/2026-09-14-foundation-authority-replay-remediation.md`

ADR-0010 makes identity-owned `OwnerAuthorityState` the local source of truth for active owner root and delegated-role currentness. Root or Device Signing replacement invalidates ordinary sessions; Administrative/Recovery-only rotation does not.

ADR-0011 keeps `(SessionId, message_seq)` as the exact-envelope replay/order boundary while `RequestId` is bounded correlation/duplicate/retry state. An aged-out request ID is a new authenticated request attempt and still requires current local authorization.

No Master Architecture revision, protobuf schema change, canonical transcript change, signature-format change, or Quinn channel-binding change is part of this remediation.

## Implementation Branch

Branch: `m9-authority-replay-remediation`  
Draft implementation PR: **#23**

Final code-bearing Task 7 commit: `cf48aaa26fb4099864c6afbd5218043e551745f1`.

Final reconciliation commit before this human verification checkpoint: `4f515d19f99aee3aac069b3c86d66ca6daf739b1`.

## Remediation Progress

### Tasks 1–3 — Complete

- Task 1 introduced identity-owned `OwnerAuthorityState` with active root, independent Device Signing/Administrative/Recovery slots, monotonic epoch floors, root-successor handling, and failed-transition atomicity. GREEN head `6e61ef944f46d9610d06c0d6093a6a6a8d15573f`; Rust CI `34828834320` passed.
- Task 2 routed device credential issuance, public-key issuance, verification, and rotation through current Device Signing authority without changing credential vectors. Final GREEN head `248de3ad55a9f75cbe3c5aa50974bea8a0bbbd46`; Rust CI `34836766158` passed.
- Task 3 routed owner approval, pairing trust/credential rotation, and ordinary delegated revocation through current local authority. Exact code head `d117323754a91a845450fb3467bf76cb9d52c2a6`; Rust CI `34865890032` and Fuzz Smoke `34865890030` passed.

### Task 4 — Complete

Pairing/session authentication and lifecycle use current owner authority. Session context snapshots owner-root and Device Signing authority; root/Device Signing replacement closes ordinary sessions and cancels control/stream authority before transport close; Administrative/Recovery-only rotation preserves ordinary sessions. Focused verification run `34930872045` passed core/simulator/Quinn/fmt checks.

### Task 5 — Complete

ADR-0011 bounded replay semantics are locked with characterization tests. A legitimately aged-out `RequestId` is a new authenticated request attempt only after current authorization; exact old `message_seq` replay remains rejected. No production replay-code change was required. Test commit `654cfad19108df15ccccbe1c84e9f4116f9a5178`; focused verification `34930994570` passed.

### Task 6 — Complete

High-level stream admission accepts local `TrustRecord` and `PolicyState`, validates authenticated-peer trust owner/device/state, and derives trust/policy revisions internally. `SimStreamRuntime::accept_one` no longer accepts caller-selected revision integers. Focused runs `34959306875` and `34960055976` passed core/simulator/Quinn/workspace checks; predecessor Fuzz Smoke `34959517230` passed. The full exact-head gates below also cover all Task 6 regressions.

### Task 7 — Complete and verified

Obsolete caller-selected high-level authority/currentness APIs are removed from ordinary public paths and all ordinary callers use `OwnerAuthorityState`. Clean method names resolve current authority internally. Raw root/delegation/floor validation remains private/low-level; `AuthorityDelegation::verify(root, minimum_epoch)` remains the cryptographic/import primitive and `DeviceCredential::from_unverified_signed_parts` remains the signed-object import boundary.

Migration runner `34972799429` passed formatting and `cargo check --workspace --all-targets --all-features`, producing code commit `cf48aaa26fb4099864c6afbd5218043e551745f1`.

Human-authored verification checkpoint `b3d2b35d8e95103f7e283585ff1951737eae07ab` passed:

- Rust CI `34973222111`: lockfile verification, `cargo audit`, `cargo fmt --check`, workspace check with all targets/features, Clippy with `-D warnings`, and `cargo test --workspace --all-features`.
- Fuzz Smoke `34973222116`: exact five-target gate under `nightly-2026-09-12`, 256 runs each.
- Identity, policy, and protocol golden/wire vectors unchanged.
- Authority-currentness, replay, stream-currentness, debug-redaction, resource-hardening, simulator lifecycle/reconnect, and Quinn lifecycle regressions.

The dependency audit exited successfully with only the accepted unsuppressed Iroh-experiment `paste 1.0.15` maintenance warning.

### Task 8 — Reconciled; exact-final-head verification pending

Task 8 Steps 1–4 and 6 are complete and checked in the active plan. The security assessment and whole-project audit now reflect the verified ADR-0010/ADR-0011 implementation and explicit deferred risks.

The PR changed-file inventory contains no `.proto` or schema files. Golden-vector source patches only migrate fixture authority lookup to `OwnerAuthorityState`; expected bytes/constants were not rewritten, and all golden/wire-vector tests pass unchanged.

Two stale Task 7 helper workflows (`task7-authority-api-migration.yml` and `task7-check.yml`) were removed during final diff review. The temporary Task 8 plan-reconciliation workflow also removed itself after producing `4f515d19f99aee3aac069b3c86d66ca6daf739b1`.

This human-authored checkpoint exists to trigger normal PR Rust CI and Fuzz Smoke on the complete reconciled state. Task 8 Step 5 remains incomplete until those exact-head runs are green. Task 8 Step 7 remains incomplete until PR #23 is merged at that verified head and post-merge `main` Rust CI is green.

## Deferred / External Risks

- Durable, rollback-resistant persistence for `OwnerAuthorityState` and accepted authority epochs.
- Platform key-store / secure-enclave integration and real mobile lifecycle evidence.
- Explicit authorization/subscription design for future authority-sensitive system-event families.
- `main` branch protection and other repository-administration controls.
- Windows/macOS/mobile CI matrices as their platform slices land.
- `paste 1.0.15` / `RUSTSEC-2024-0436` in the isolated Iroh experiment, to re-evaluate before ADR-0009 production promotion.
- M9 remote NAT/relay/path/measurement work, which remains blocked until this remediation is merged and post-merge verified.

## Exact Next Task

1. Require Rust CI and Fuzz Smoke success on this exact human-authored PR head.
2. Re-check PR #23 head/mergeability and the final changed-file inventory.
3. Merge only the verified expected head SHA.
4. Require post-merge `main` Rust CI green.
5. Update durable state with the final PR and post-merge evidence; only then make **M9 remote-networking Task 4** the next implementation task.

## M9 Invariants

- Quinn remains the verified local/LAN baseline.
- Iroh `1.2.0` remains isolated under `experiments/m9-networking` until ADR-0009 selects an architecture.
- Iroh identity/address/path/relay/transport metadata never becomes Cross-Lab identity/trust/policy authority.
- No 0-RTT authority.
- M9 remote sessions remain `NetworkClass::Remote` for their lifetime.
- Route changes do not silently mutate binding, `SessionId`, sequence, policy classification, or operation authority.
- New transport connections require fresh Cross-Lab authentication/authorization state.
- Security authority state is local and fail-closed.

## Resume Procedure

1. inspect `main`, PR #23, recent workflows, and this file;
2. require the current exact head to be green before merge or the next remediation step;
3. merge PR #23 only at its expected verified head;
4. require post-merge `main` Rust CI green;
5. do not resume M9 networking Task 4 until the remediation is merged and post-merge verified.
