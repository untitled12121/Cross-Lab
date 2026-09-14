# Current Development State

This file is the durable resume guide for active Cross-Lab development. Git, code, and tests are the factual implementation state; this file records the intended handoff.

## Current Phase

**Phase 1 — Core Simulator / Real Transport Transition**

## Current Milestone

**M9 — Remote Networking ADR, paused before Task 4 until the second foundation security remediation is implemented, merged, and verified.**

M1–M8 are complete. M9 research Tasks 1–3 are complete. Foundation remediation Tasks 1–8 are integrated into canonical `main`. The remaining foundation work is the accepted authority-currentness/replay remediation defined by ADR-0010 and ADR-0011 plus the final security reconciliation gate.

## Canonical Baseline

- `main`: `7448eb7b3978c3563b6c61552d423c3d2f5d748f` (`Merge M9 foundation remediation Task 8`).
- Post-merge Rust CI `34806566405` passed dependency audit, rustfmt, workspace check, Clippy with `-D warnings`, and the complete workspace test suite.
- PR #20 is merged; Task 8 secure policy-ID generation is durable on `main`.
- PR #20 exact-head Fuzz Smoke `34806180058` passed before merge.
- The documented `paste 1.0.15` / `RUSTSEC-2024-0436` maintenance warning remains isolated to the Iroh experiment dependency graph and must be re-evaluated before production networking promotion.

## Accepted Architecture Checkpoint

Architecture branch: `m9-foundation-authority-replay-design`  
Architecture checkpoint PR: **#22**

The original design head `1065818bb1d8b528d96901004ea2ea9b95de7a45` passed Rust CI run `34815054120` through dependency audit, format, check, Clippy, and the complete workspace tests.

The owner explicitly approved the written design on 2026-09-14. The following are **Accepted**:

- `docs/adr/ADR-0010-authoritative-owner-authority-currentness.md`
- `docs/adr/ADR-0011-bounded-request-replay-semantics.md`
- `docs/superpowers/specs/2026-09-14-foundation-currentness-replay-design.md`

Affected normative focused specifications are reconciled:

- `docs/architecture/IDENTITY-AND-KEYS.md`
- `docs/architecture/SESSION-TRANSPORT.md`
- `docs/protocol/PROTOCOL-V1.md`

`POLICY-AUTHORIZATION.md` already requires trust/policy currentness to come from local authoritative state, so stream API hardening remains implementation of that existing contract.

The Master Architecture was not revised because ADR-0010 enforces its existing local-authority/fail-closed invariants and ADR-0011 changes focused request-lifecycle semantics without changing Master-level wire/transport architecture. ADR-0009 remains reserved for M9 remote networking.

## Accepted Security Semantics

ADR-0010 establishes identity-owned `OwnerAuthorityState` with one active owner root plus Device Signing, Administrative, and Recovery role slots. Each delegated role retains its highest accepted epoch and its exact active delegation when valid under the active root. Root succession is validate-then-commit, makes the previous root historical, clears active delegated-role objects, and retains role epoch floors. New-root delegations must strictly advance those floors. Ordinary authority-bearing APIs resolve current authority from this state rather than caller-selected roots, delegations, or epoch floors. Root or Device Signing replacement invalidates ordinary active sessions; Administrative/Recovery-only rotation does not. Fresh ordinary authentication fails while Device Signing has no active delegation after root rotation. Phase 1 state remains in memory; production restart durability/rollback protection is later persistence work. CRDT, relay, transport, or peer-majority state never establishes owner authority currentness.

ADR-0011 establishes authenticated `(SessionId, message_seq)` as the Phase 1 exact-envelope replay/order boundary. `RequestId` is bounded correlation/duplicate/retry state, not a forever-retained second replay log. Retained duplicate IDs fail closed unless a locally authorized capability-specific idempotency path exists; peer-declared `Idempotent` grants no authority. Active request state is never evicted for completed history. An ancient ID that ages out is a new authenticated request attempt and still requires current trust/capability/policy/operation authority. Exact old-envelope replay remains rejected by its old sequence. No generic durable exactly-once guarantee or protobuf/wire change is introduced.

High-level stream admission will accept local `TrustRecord` and `PolicyState`, validate peer trust, and derive current revisions internally. Low-level policy operation validation may retain explicit revision parameters for focused internal use.

## Implementation Plan

Accepted regression-first plan:

`docs/superpowers/plans/2026-09-14-foundation-authority-replay-remediation.md`

It contains eight tasks:

1. add `OwnerAuthorityState` with monotonic role floors and root-successor behavior;
2. route device credential issue/verify/rotation through current authority;
3. migrate policy approval/pairing/trust transitions;
4. migrate pairing/session authentication and authority-triggered cancellation;
5. lock ADR-0011 with bounded-replay characterization tests;
6. derive stream currentness from local trust/policy state;
7. remove obsolete high-level caller-selected authority APIs and verify golden compatibility;
8. run the complete security/CI/Fuzz gate and reconcile audit/CURRENT evidence.

No production Rust changes belong in PR #22. Implementation starts from a fresh branch based on verified `main` after PR #22 is merged.

## Completed Foundation Remediation Before ADR-0010/0011

Tasks 1–8 already on `main`:

1. pairing currentness and fixed initial credential epoch;
2. verified credential rotation and active-session invalidation;
3. trusted-state and approval provenance;
4. receiver-local bounded request replay authority;
5. explicit event subscription/authorization boundary;
6. Quinn plain-`Drop` connection/task ownership;
7. Debug/privacy hardening for session/pairing/auth material;
8. OS-CSPRNG-backed typed generation for `RuleId`, `TransitionId`, and `OperationId`.

Exact historical RED/GREEN commits and workflow evidence remain in git history, the remediation plan, and the whole-project audit record.

## Exact Next Task

**Merge PR #22 only after its exact current head passes CI.** Use the PR's actual head SHA from GitHub for CI verification and expected-head merge protection rather than duplicating a moving branch SHA in this file.

After merge:

1. verify the resulting `main` merge commit with the full Rust CI gate;
2. create a fresh implementation branch from verified `main`;
3. execute Task 1 of `2026-09-14-foundation-authority-replay-remediation.md` regression-first;
4. continue through Task 8 in small verified milestones;
5. keep M9 Task 4 blocked until the full implementation is merged and post-merge `main` is green.

## Repository / M9 Invariants

- Quinn remains the verified local/LAN baseline.
- Iroh `1.2.0` remains an isolated M9 remote/NAT/relay candidate until ADR-0009 selects an architecture.
- Iroh identities, addresses, paths, relay metadata, and transport credentials never become Cross-Lab identity/trust/policy authority.
- No 0-RTT authority.
- M9 remote sessions remain `NetworkClass::Remote` for their lifetime.
- Transport/path changes do not silently mutate binding, `SessionId`, sequence, policy classification, or operation authority.
- A new transport connection requires fresh Cross-Lab authentication/authorization state.
- Security authority state is locally authoritative and fails closed on ambiguity.
- `main` branch protection remains an external repository-administration item.

## Resume Procedure

1. inspect canonical `main`, PR #22, recent workflows, and this file;
2. verify PR #22 exact-head CI and merge only when green;
3. verify the resulting `main` merge commit;
4. create a fresh implementation branch from verified `main`;
5. read ADR-0010, ADR-0011, the accepted design, and the implementation plan;
6. execute the plan regression-first in small verified milestones;
7. update this file with exact implementation commits/workflow evidence at meaningful checkpoints;
8. do not begin M9 Task 4 until the entire foundation remediation is merged and post-merge verified.
