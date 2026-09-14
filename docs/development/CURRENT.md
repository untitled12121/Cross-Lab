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

Architecture branch:

`m9-foundation-authority-replay-design`

PR:

`#22 — Propose authoritative authority currentness and bounded replay semantics`

The original design head `1065818bb1d8b528d96901004ea2ea9b95de7a45` passed Rust CI run `34815054120` through dependency audit, format, check, Clippy, and the complete workspace tests.

The owner explicitly approved the written design on 2026-09-14. The following are now **Accepted** on the architecture branch:

- `docs/adr/ADR-0010-authoritative-owner-authority-currentness.md`
- `docs/adr/ADR-0011-bounded-request-replay-semantics.md`
- `docs/superpowers/specs/2026-09-14-foundation-currentness-replay-design.md`

Affected normative focused specifications are reconciled:

- `docs/architecture/IDENTITY-AND-KEYS.md`
- `docs/architecture/SESSION-TRANSPORT.md`
- `docs/protocol/PROTOCOL-V1.md`

`POLICY-AUTHORIZATION.md` already requires trust/policy currentness to come from local authoritative state, so the stream API change is implementation hardening of the existing policy contract rather than a new policy-architecture decision.

The Master Architecture was not revised because ADR-0010 enforces its existing local-authority/fail-closed invariants and ADR-0011 changes focused request-lifecycle semantics without changing the Master-level wire/transport architecture. ADR-0009 remains reserved for the M9 remote-networking decision.

## Accepted ADR-0010 Semantics

`crosslab-identity` will own an in-memory `OwnerAuthorityState` containing:

- one active `OwnerRootRecord`;
- Device Signing, Administrative, and Recovery role slots;
- the highest accepted epoch for each delegated role;
- the exact active delegation for each role when one is valid under the active root.

Security rules:

1. root successor is fully verified before state mutation;
2. after root succession, the old root becomes historical;
3. root succession clears active delegated-role objects but retains role epoch floors;
4. a new-root delegation must strictly advance the retained role epoch;
5. high-level authority-bearing APIs resolve current authority from `OwnerAuthorityState`, not caller-selected root/delegation/floor inputs;
6. root or Device Signing replacement invalidates ordinary active sessions authenticated under superseded authority;
7. Administrative/Recovery-only rotation does not invalidate ordinary device sessions;
8. fresh ordinary authentication fails while Device Signing has no active delegation after root rotation;
9. Phase 1 authority state may remain in memory; production restart durability/rollback resistance requires later atomic local persistence and load-time revalidation;
10. CRDT, relay, transport, and peer-majority state never establish owner authority currentness.

## Accepted ADR-0011 Semantics

- authenticated `(SessionId, message_seq)` is the Phase 1 exact-envelope replay/order boundary;
- `RequestId` is a random session-scoped correlation/duplicate/retry key with bounded recent history;
- retained duplicate IDs fail closed unless a locally authorized capability-specific idempotency path exists;
- peer-declared `RetryClass::Idempotent` never grants duplicate execution authority;
- active request state is never evicted to preserve completed history;
- an ancient completed ID that legitimately ages out is a new authenticated request attempt and still requires current trust/capability/policy/operation authority;
- exact old-envelope replay remains rejected by its old `message_seq`;
- stronger single-use/idempotency semantics belong to locally authoritative capability/`AuthorizedOperation` state;
- no generic durable exactly-once behavior or protobuf/wire change is introduced.

## Stream Currentness Hardening

High-level stream admission will stop accepting raw `current_trust_revision` / `current_policy_revision` numbers. It will accept local `TrustRecord` and `PolicyState`, validate the authenticated peer/trust state, and derive current revisions internally before `AuthorizedOperation` validation.

Low-level policy operation validation may keep explicit revision values for focused internal tests; less-trusted platform/runtime callers must not declare which security revision is current.

## Implementation Plan

Accepted implementation plan:

`docs/superpowers/plans/2026-09-14-foundation-authority-replay-remediation.md`

The plan is regression-first and contains eight verifiable tasks:

1. add `OwnerAuthorityState` with monotonic role floors and root-successor behavior;
2. route device credential issue/verify/rotation through current authority;
3. migrate policy approval/pairing/trust transitions to current authority;
4. migrate pairing/session authentication and authority-triggered session cancellation;
5. lock ADR-0011 with bounded-replay characterization tests;
6. derive stream currentness from local trust/policy state;
7. remove obsolete high-level caller-selected authority APIs and verify golden compatibility;
8. run the complete security/CI/Fuzz gate and reconcile audit/CURRENT evidence.

No production Rust changes are part of PR #22. Implementation begins only from a fresh branch based on the verified `main` commit after this accepted architecture checkpoint is merged.

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

**Finish and merge the accepted architecture/implementation-plan checkpoint on PR #22 after its exact current head passes CI.**

The architecture branch head after ADR acceptance, normative spec reconciliation, plan self-review, and this durable resume update is:

`0ffda0ad7ff567e95c24d8c5ca7b904e13cc3a96`

A later metadata-only PR update does not change this Git head. If any repository-content commit is added after this line, replace the recorded head before merge.

Then:

1. verify PR #22 exact-head CI;
2. merge PR #22 with the expected exact head SHA;
3. verify the resulting merge commit on `main` with the full Rust CI gate;
4. create a fresh implementation branch from that verified `main` commit;
5. execute Task 1 of `2026-09-14-foundation-authority-replay-remediation.md` regression-first;
6. continue in small reviewed/verified tasks through Task 8;
7. keep M9 Task 4 blocked until the complete implementation is merged and post-merge `main` is green.

## Repository / M9 Invariants

- Quinn remains the verified local/LAN baseline.
- Iroh `1.2.0` remains an isolated M9 remote/NAT/relay candidate under `experiments/m9-networking` until ADR-0009 selects an architecture.
- Iroh identities, addresses, paths, relay metadata, and transport credentials never become Cross-Lab identity/trust/policy authority.
- No 0-RTT authority.
- M9 remote sessions remain `NetworkClass::Remote` for their lifetime.
- Transport/path changes do not silently mutate binding, `SessionId`, sequence, policy classification, or operation authority.
- A new transport connection requires fresh Cross-Lab authentication/authorization state.
- Security authority state is locally authoritative and fails closed on ambiguity.
- `main` branch protection remains an external repository-administration item.

## Resume Procedure

1. inspect canonical `main`, PR #22, recent workflows, and this file;
2. verify PR #22 exact-head CI before merge;
3. merge accepted architecture/docs only when that gate is green;
4. verify the resulting `main` merge commit;
5. create a fresh implementation branch from verified `main`;
6. read ADR-0010, ADR-0011, the accepted design, and the implementation plan;
7. execute the plan regression-first in small verified milestones;
8. update this file with exact commits/workflow evidence at meaningful checkpoints;
9. do not begin M9 Task 4 until the entire foundation remediation is merged and post-merge verified.
