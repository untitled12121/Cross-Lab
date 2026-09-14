# Current Development State

This file is the durable resume guide for active Cross-Lab development. Git/code/tests are factual state; this file records the intended handoff.

## Current Phase

**Phase 1 — Core Simulator / Real Transport Transition**

## Current Milestone

**M9 — Remote Networking ADR, paused before Task 4 until the second foundation security remediation is implemented, merged, and verified.**

M1–M8 are complete. M9 research Tasks 1–3 are complete. Foundation remediation Tasks 1–8 are integrated into canonical `main`. The accepted authority-currentness/replay remediation is now in implementation on PR #23.

## Canonical Baseline

- `main`: `1426e70c6db169a37a89dbae0565a36a619f2651` after merging accepted architecture PR #22.
- Post-merge Rust CI `34826806366` passed dependency audit, rustfmt, workspace check, Clippy with `-D warnings`, and complete workspace tests.
- `paste 1.0.15` / `RUSTSEC-2024-0436` remains an isolated Iroh-experiment maintenance warning and must be re-evaluated before production networking promotion.

## Accepted Architecture Checkpoint

Architecture PR #22 is merged. The owner explicitly approved the written design on 2026-09-14. Accepted artifacts:

- `docs/adr/ADR-0010-authoritative-owner-authority-currentness.md`
- `docs/adr/ADR-0011-bounded-request-replay-semantics.md`
- `docs/superpowers/specs/2026-09-14-foundation-currentness-replay-design.md`

Normative focused specs reconciled:

- `docs/architecture/IDENTITY-AND-KEYS.md`
- `docs/architecture/SESSION-TRANSPORT.md`
- `docs/protocol/PROTOCOL-V1.md`

The Master Architecture was not revised: ADR-0010 enforces existing local-authority/fail-closed invariants, while ADR-0011 changes focused request-lifecycle semantics without changing Master-level wire/transport architecture. ADR-0009 remains reserved for M9 remote networking.

## Accepted Security Semantics

ADR-0010: `crosslab-identity` owns `OwnerAuthorityState` with active root plus Device Signing, Administrative, and Recovery role slots. Each delegated role retains its highest accepted epoch and exact active delegation. Root succession makes the old root historical, clears active delegated objects, retains role epoch floors, and requires new-root delegations to strictly advance those floors. High-level authority-bearing APIs resolve current authority from this state. Root/Device Signing replacement invalidates ordinary sessions; Administrative/Recovery-only rotation does not. Fresh ordinary auth fails while Device Signing is inactive after root rotation. Phase 1 remains in-memory; durable rollback-resistant persistence is later work. CRDT/relay/transport/peer-majority state never establishes authority currentness.

ADR-0011: `(SessionId, message_seq)` is the exact-envelope replay/order boundary. `RequestId` is bounded correlation/duplicate/retry state. Retained duplicates fail closed unless a locally authorized capability-specific idempotency path exists. Peer-declared `Idempotent` grants no authority. A legitimately aged-out ID becomes a new authenticated request attempt and still requires current authorization/operation validity. No generic exactly-once or wire change is introduced.

Stream hardening: high-level stream admission takes local `TrustRecord` and `PolicyState`, validates peer trust, and derives current revisions internally instead of accepting caller-selected revision integers.

## Implementation Branch

Branch: `m9-authority-replay-remediation`  
Draft implementation PR: **#23**

### Task 1 — Complete

`OwnerAuthorityState` is implemented in `crosslab-identity` with:

- active owner root;
- independent Device Signing, Administrative, and Recovery role slots;
- highest accepted epoch floors;
- strictly monotonic delegated-role replacement;
- failed-transition atomicity;
- root succession that clears active delegated objects while retaining epoch floors;
- no `Copy` or `Clone` on the authoritative state object.

TDD evidence:

- RED commit `be46b70d90874ec31e74072a259b88f010bf122f` failed at workspace check because `OwnerAuthorityState` did not exist.
- GREEN exact head `6e61ef944f46d9610d06c0d6093a6a6a8d15573f` passed Rust CI `34828834320` including dependency audit, rustfmt, workspace check, Clippy, and complete tests.

## Implementation Plan

`docs/superpowers/plans/2026-09-14-foundation-authority-replay-remediation.md`

Eight regression-first tasks:

1. **complete** — add `OwnerAuthorityState`;
2. **next** — route device credentials through current authority;
3. migrate approval/pairing/trust transitions;
4. migrate pairing/session auth and authority-triggered cancellation;
5. lock ADR-0011 with replay characterization tests;
6. derive stream currentness from local trust/policy state;
7. remove obsolete caller-selected authority APIs and verify golden compatibility;
8. run full security/CI/Fuzz reconciliation.

## Exact Next Task

Execute Task 2 regression-first:

1. add a credential regression proving a credential tied to superseded Device Signing authority fails `verify_current` and the superseded signing key cannot issue through `issue_current`;
2. capture the expected RED failure because the current-authority credential APIs do not yet exist;
3. implement the minimal state-backed credential methods without changing transcript/signature/wire bytes;
4. run identity tests including golden vectors and Clippy;
5. checkpoint the verified Task 2 commit before starting Task 3.

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
2. continue from the first incomplete remediation task only after the prior exact head is green;
3. execute each task regression-first with small verified checkpoints;
4. update this file with exact implementation commits/workflow evidence after meaningful progress;
5. do not begin M9 Task 4 until the entire remediation is merged and post-merge verified.
