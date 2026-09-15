# Current Development State

This file is the durable resume guide for active Cross-Lab development. Git/code/tests are factual state; this file records the intended handoff.

## Current Phase

**Phase 1 — Core Simulator / Real Transport Transition**

## Current Milestone

**M9 — Remote Networking ADR, paused before networking Task 4 until the second foundation security remediation is merged and post-merge verified.**

M1–M8 are complete. M9 networking research Tasks 1–3 are complete. The accepted authority-currentness/replay remediation is in implementation on PR #23.

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

## Remediation Progress

### Task 1 — Complete

Implemented identity-owned `OwnerAuthorityState` with active root, independent Device Signing/Administrative/Recovery slots, monotonic epoch floors, root-successor handling, and failed-transition atomicity.

Evidence: GREEN head `6e61ef944f46d9610d06c0d6093a6a6a8d15573f`; Rust CI `34828834320` passed.

### Task 2 — Complete

Device credential issuance, public-key issuance, verification, and rotation resolve current Device Signing authority from `OwnerAuthorityState`. Golden credential bytes remain unchanged.

Evidence: final GREEN head `248de3ad55a9f75cbe3c5aa50974bea8a0bbbd46`; Rust CI `34836766158` passed.

### Task 3 — Complete

Owner approvals, pairing trust/credential rotation, and ordinary delegated revocation resolve current local authority from `OwnerAuthorityState`. Policy golden vectors remain unchanged.

Evidence: exact code head `d117323754a91a845450fb3467bf76cb9d52c2a6`; Rust CI `34865890032` and Fuzz Smoke `34865890030` passed.

### Task 4 — Complete

Pairing/session authentication and lifecycle now use current owner authority. Session context snapshots owner-root and Device Signing authority, `LogicalSession::revalidate_authority` closes on root/Device Signing replacement, Administrative/Recovery-only rotation preserves ordinary sessions, and simulator control/stream runtimes cancel session authority before transport close. Quinn fixtures remain transport-only consumers of Cross-Lab authority state rather than authority owners.

Focused exact-branch verification run `34930872045` passed `crosslab-core`, `crosslab-sim`, `crosslab-transport-quic`, and formatting.

### Task 5 — Complete

ADR-0011 is locked with characterization tests. A legitimately aged-out `RequestId` is accepted as a new authenticated request attempt only if current policy permits it; current policy denial still fails closed; exact old `message_seq` replay remains rejected. No production replay code change was required.

Task 5 test commit: `654cfad19108df15ccccbe1c84e9f4116f9a5178`. Focused verification run `34930994570` passed the core/simulator/Quinn/fmt gate.

### Task 6 — Complete, exact-head CI confirmation pending

High-level stream admission accepts local `TrustRecord` and `PolicyState`, validates authenticated-peer trust ownership/state, and derives trust/policy revisions internally. `SimStreamRuntime::accept_one` no longer accepts caller-selected revision integers.

TDD RED: `cb37c0b93c327cda8685bbd25f075c6bf2620870`; CI `34931103468` failed exactly because the old API still required two `u64` revisions and the new trust errors did not exist.

Implementation includes:

- `StreamAdmissionError::PeerTrustMismatch` and `PeerNotTrusted`;
- local trust owner/device/state validation before operation reservation;
- internally derived `TrustRecord::trust_revision()` and `PolicyState::revision()`;
- simulator, Quinn, lifecycle, and resource-hardening fixture migration to local trust/policy state;
- tests for changed policy revision, locally revoked peer, and mismatched peer trust.

Task 6 final code head before this documentation checkpoint: `6952b9dbec62a4c00e469e87bc589e3bd3e989e2`. Focused workflow `34960055976` passed the resource-hardening regression, full Quinn tests, complete workspace `cargo check --all-targets --all-features`, and `cargo fmt --check`. Earlier focused workflow `34959306875` passed core stream admission, full simulator tests, full Quinn transport tests, and formatting. Fuzz Smoke `34959517230` passed all five bounded targets on the immediately preceding checkpoint.

## Remaining Plan

7. remove obsolete caller-selected high-level authority/currentness APIs, migrate remaining call sites, and prove identity/policy/protocol golden compatibility;
8. run final full security/CI/Fuzz reconciliation, update durable evidence, merge PR #23, and verify the resulting `main` commit.

## Exact Next Task

1. Require fresh exact-head Rust CI and Fuzz Smoke success after this checkpoint.
2. When green, execute Task 7 using compiler/search output as the migration checklist; do not add compatibility shims that recreate caller-selected currentness.
3. Preserve `AuthorityDelegation::verify(root, minimum_epoch)` only as the low-level cryptographic/import primitive and preserve `DeviceCredential::from_unverified_signed_parts`.
4. Verify identity, policy, and protocol golden vectors byte-for-byte.
5. Execute Task 8 and keep M9 networking Task 4 blocked until PR #23 is merged and post-merge `main` is green.

## M9 Invariants

- Quinn remains the verified local/LAN baseline.
- Iroh `1.2.0` remains isolated under `experiments/m9-networking` until ADR-0009 selects an architecture.
- Iroh identity/address/path/relay/transport metadata never becomes Cross-Lab identity/trust/policy authority.
- No 0-RTT authority.
- M9 remote sessions remain `NetworkClass::Remote` for their lifetime.
- Route changes do not silently mutate binding, `SessionId`, sequence, policy classification, or operation authority.
- New transport connections require fresh Cross-Lab authentication/authorization state.
- Security authority state is local and fail-closed.
- `main` branch protection remains an external repository-administration item.

## Resume Procedure

1. inspect `main`, PR #23, recent workflows, and this file;
2. require the prior exact head to be green before starting the next remediation task;
3. execute remaining tasks in small verified checkpoints;
4. update this file with exact evidence after meaningful progress;
5. do not resume M9 networking Task 4 until the entire remediation is merged and post-merge verified.
