# Current Development State

This file is the durable resume guide for active Cross-Lab development. Git/code/tests are factual state; this file records the intended handoff.

## Current Phase

**Phase 1 — Core Simulator / Real Transport Transition**

## Current Milestone

**M9 — Remote Networking ADR, paused before Task 4 until the second foundation security remediation is implemented, merged, and verified.**

M1–M8 are complete. M9 research Tasks 1–3 are complete. Foundation remediation Tasks 1–8 are integrated into canonical `main`. The remaining foundation work is the accepted authority-currentness/replay remediation plus the final security reconciliation gate.

## Canonical Baseline

- `main`: `7448eb7b3978c3563b6c61552d423c3d2f5d748f`.
- Post-merge Rust CI `34806566405` passed dependency audit, rustfmt, workspace check, Clippy with `-D warnings`, and complete workspace tests.
- PR #20 Task 8 is merged; exact-head Fuzz Smoke `34806180058` passed before merge.
- `paste 1.0.15` / `RUSTSEC-2024-0436` remains an isolated Iroh-experiment maintenance warning and must be re-evaluated before production networking promotion.

## Accepted Architecture Checkpoint

Architecture branch: `m9-foundation-authority-replay-design`  
Architecture checkpoint PR: **#22**

The owner explicitly approved the written design on 2026-09-14. Accepted artifacts:

- `docs/adr/ADR-0010-authoritative-owner-authority-currentness.md`
- `docs/adr/ADR-0011-bounded-request-replay-semantics.md`
- `docs/superpowers/specs/2026-09-14-foundation-currentness-replay-design.md`

Normative focused specs reconciled:

- `docs/architecture/IDENTITY-AND-KEYS.md`
- `docs/architecture/SESSION-TRANSPORT.md`
- `docs/protocol/PROTOCOL-V1.md`

The original design head `1065818bb1d8b528d96901004ea2ea9b95de7a45` passed Rust CI `34815054120`. The exact current PR head must be checked from GitHub before merge.

The Master Architecture was not revised: ADR-0010 enforces existing local-authority/fail-closed invariants, while ADR-0011 changes focused request-lifecycle semantics without changing Master-level wire/transport architecture. ADR-0009 remains reserved for M9 remote networking.

## Accepted Security Semantics

ADR-0010: `crosslab-identity` owns `OwnerAuthorityState` with active root plus Device Signing, Administrative, and Recovery role slots. Each delegated role retains its highest accepted epoch and exact active delegation. Root succession makes the old root historical, clears active delegated objects, retains role epoch floors, and requires new-root delegations to strictly advance those floors. High-level authority-bearing APIs resolve current authority from this state. Root/Device Signing replacement invalidates ordinary sessions; Administrative/Recovery-only rotation does not. Fresh ordinary auth fails while Device Signing is inactive after root rotation. Phase 1 remains in-memory; durable rollback-resistant persistence is later work. CRDT/relay/transport/peer-majority state never establishes authority currentness.

ADR-0011: `(SessionId, message_seq)` is the exact-envelope replay/order boundary. `RequestId` is bounded correlation/duplicate/retry state. Retained duplicates fail closed unless a locally authorized capability-specific idempotency path exists. Peer-declared `Idempotent` grants no authority. A legitimately aged-out ID becomes a new authenticated request attempt and still requires current authorization/operation validity. No generic exactly-once or wire change is introduced.

Stream hardening: high-level stream admission takes local `TrustRecord` and `PolicyState`, validates peer trust, and derives current revisions internally instead of accepting caller-selected numbers.

## Implementation Plan

`docs/superpowers/plans/2026-09-14-foundation-authority-replay-remediation.md`

Eight regression-first tasks:

1. add `OwnerAuthorityState`;
2. route device credentials through current authority;
3. migrate approval/pairing/trust transitions;
4. migrate pairing/session auth and authority-triggered cancellation;
5. lock ADR-0011 with replay characterization tests;
6. derive stream currentness from local trust/policy state;
7. remove obsolete caller-selected authority APIs and verify golden compatibility;
8. run full security/CI/Fuzz reconciliation.

No production Rust changes belong in PR #22. Implementation starts on a fresh branch from verified `main` after PR #22 merges.

## Exact Next Task

**Merge PR #22 only after its exact current head passes CI.** Use GitHub's actual PR head SHA for CI verification and expected-head merge protection.

Then:

1. verify the merge commit on `main` with full Rust CI;
2. create a fresh implementation branch from verified `main`;
3. execute Task 1 of the accepted implementation plan regression-first;
4. continue through Task 8 in small verified milestones;
5. keep M9 Task 4 blocked until the implementation is merged and post-merge `main` is green.

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

1. inspect `main`, PR #22, recent workflows, and this file;
2. verify PR #22 exact-head CI and merge only when green;
3. verify post-merge `main`;
4. create a fresh implementation branch;
5. read ADR-0010, ADR-0011, accepted design, and implementation plan;
6. execute the plan regression-first with small verified checkpoints;
7. update this file with exact implementation commits/workflow evidence;
8. do not begin M9 Task 4 until the entire remediation is merged and post-merge verified.
