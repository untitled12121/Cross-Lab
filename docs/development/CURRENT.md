# Current Development State

This file is the durable resume guide for active Cross-Lab development. Git, code, and tests are the factual implementation state; this file records the intended handoff.

## Current Phase

**Phase 1 — Core Simulator / Real Transport Transition**

## Current Milestone

**M9 — Remote Networking ADR, paused before M9 Task 4 until the second foundation security remediation is fully reconciled.**

M1–M8 are complete. M9 research Tasks 1–3 are complete. Foundation remediation Tasks 1–8 are integrated into `main`. The remaining work is architecture-sensitive foundation currentness/replay hardening plus the final reconciliation gate.

## Canonical Baseline

- `main` integration commit: `7448eb7b3978c3563b6c61552d423c3d2f5d748f` (`Merge M9 foundation remediation Task 8`).
- Post-merge Rust CI `34806566405` passed lockfile verification, dependency audit, rustfmt, workspace check, Clippy with `-D warnings`, and the complete workspace test suite.
- PR #20 is merged; Task 8 secure policy-ID generation is durable on `main`.
- Exact PR #20 head Fuzz Smoke `34806180058` passed before merge.
- The documented `paste 1.0.15` / `RUSTSEC-2024-0436` maintenance warning remains isolated to the Iroh experiment dependency graph and is not accepted for production promotion without re-evaluation.

## Active Architecture Checkpoint

Branch:

`m9-foundation-authority-replay-design`

Written design:

`docs/superpowers/specs/2026-09-14-foundation-currentness-replay-design.md`

Proposed ADRs:

- `docs/adr/ADR-0010-authoritative-owner-authority-currentness.md`
- `docs/adr/ADR-0011-bounded-request-replay-semantics.md`

ADR-0009 remains reserved for the M9 remote-networking decision.

The two new ADRs are deliberately **Proposed** until the owner reviews the exact written specification. No production implementation is authorized from this branch yet.

## Foundation Findings Requiring Durable Resolution

### 1. Active root and delegated-role currentness

Current high-level APIs can still receive raw root/delegation objects and, in some paths, caller-selected delegation epoch floors. Cryptographic validity of a supplied historical authority is not equivalent to current local authority after rotation.

Proposed ADR-0010 establishes identity-owned `OwnerAuthorityState` as the local source of truth for:

- active owner root;
- current Device Signing delegation;
- current Administrative delegation;
- current Recovery delegation.

Required semantics:

1. normal root successor is fully verified before active-root mutation;
2. superseded root records cannot authorize new high-level ordinary owner operations;
3. first delegated-role acceptance verifies completely under the active root and may first appear above epoch zero;
4. replacement requires a strictly higher role-specific epoch and validates before mutation;
5. sensitive high-level APIs obtain authority from local state rather than caller-selected currentness;
6. root or Device Signing replacement invalidates ordinary active sessions authenticated under superseded authority;
7. Administrative/Recovery rotation alone does not tear down ordinary device sessions;
8. Phase 1 state may be in-memory; production restart durability requires later atomic local persistence and load-time revalidation;
9. authority currentness is never established by CRDT, relay, transport, or peer-majority state.

### 2. Request replay versus bounded state

Protocol V1 currently overstates `RequestId` lifetime semantics relative to the bounded implementation requirement.

Proposed ADR-0011 makes the distinction explicit:

- authenticated `(SessionId, message_seq)` is the full-session ordered control anti-replay boundary;
- `RequestId` is a random session-scoped correlation/retry key with bounded recent history;
- peer-declared `Idempotent` never authorizes duplicate execution;
- local capability metadata must explicitly allow any future duplicate-result/re-execution path;
- active request state is never silently evicted to preserve completed history;
- no generic durable exactly-once guarantee is introduced.

No protobuf/wire change is proposed.

### 3. Stream currentness misuse resistance

The Policy/Authorization architecture already says trust and policy revision come from local authoritative state, but high-level stream admission currently accepts raw numeric `current_trust_revision` and `current_policy_revision` values.

After the ADRs are accepted, implementation should harden the high-level boundary so stream admission derives current trust/policy state from local `TrustRecord` / `PolicyState` (or the eventual authoritative local stores) rather than asking callers to declare which numeric revisions are current.

This is treated as implementation hardening of the existing policy architecture, not a new ADR.

### 4. System-event guardrail

Generic system events remain opaque protocol data. No new speculative registry is required now. Before a real system-event family can trigger privileged or policy-sensitive behavior, that family must define explicit local authorization/subscription semantics and fail-closed handling.

## Completed Foundation Remediation

Tasks 1–8 are merged to `main`:

1. pairing currentness and fixed initial credential epoch;
2. verified credential rotation and active-session invalidation;
3. trusted-state and approval provenance;
4. receiver-local request replay authority;
5. explicit event subscription/authorization boundary;
6. Quinn plain-`Drop` connection/task ownership;
7. Debug/privacy hardening for session/pairing/auth material;
8. OS-CSPRNG-backed typed generation for `RuleId`, `TransitionId`, and `OperationId`.

Exact historical RED/GREEN commits and workflow evidence remain in git history, the remediation plan, and the whole-project audit record.

## Exact Next Task

**Owner review of the written foundation design/ADR checkpoint.**

Do not implement production code yet.

After written approval:

1. mark ADR-0010 and ADR-0011 Accepted;
2. update affected normative focused specs (`IDENTITY-AND-KEYS.md`, `SESSION-TRANSPORT.md`, `PROTOCOL-V1.md`, and any policy wording required for local-currentness provenance);
3. update the Master Architecture only if the accepted wording materially changes rather than clarifies its existing invariants;
4. create the detailed regression-first implementation plan;
5. implement authority state, authority-triggered session currentness, bounded request semantics reconciliation, and stream local-currentness hardening;
6. run exact-head dependency/audit/fmt/check/Clippy/test/Fuzz gates;
7. reconcile `security/M9-FOUNDATION-HARDENING-ASSESSMENT.md`, the whole-project audit, and this file;
8. merge and verify on `main`;
9. only then resume M9 Task 4.

## Repository / M9 Invariants

- Quinn remains the verified local/LAN baseline.
- Iroh `1.2.0` remains an isolated M9 remote/NAT/relay candidate under `experiments/m9-networking` until ADR-0009 selects an architecture.
- Iroh identities, addresses, paths, relay metadata, and transport credentials never become Cross-Lab identity/trust/policy authority.
- No 0-RTT authority.
- M9 remote sessions remain `NetworkClass::Remote` for their lifetime.
- Transport/path changes do not silently mutate binding, `SessionId`, sequence, policy classification, or operation authority.
- A new transport connection requires fresh Cross-Lab authentication/authorization state.
- Security authority state is locally authoritative and fail-closed on ambiguity.
- `main` branch protection remains an external repository-administration item.

## Resume Procedure

1. inspect canonical `main`, this design branch, recent commits/workflows, and this file;
2. read the Master Architecture plus the written foundation design and proposed ADR-0010/0011;
3. do not begin implementation until the written design is explicitly approved;
4. after approval, update normative specs and write the implementation plan before code;
5. implement regression-first in small verified milestones;
6. checkpoint exact commits/workflow evidence in this file;
7. keep M9 Task 4 blocked until the entire foundation remediation is merged and verified on `main`.
