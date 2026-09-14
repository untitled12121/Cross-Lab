# Current Development State

This file is the durable resume guide for active Cross-Lab development. Git, code, and tests are the factual implementation state; this file records the intended handoff.

## Current Phase

**Phase 1 — Core Simulator / Real Transport Transition**

## Current Milestone

**M9 — Remote Networking ADR, paused before M9 Task 4 until the second foundation security remediation is fully reconciled.**

M1–M8 are complete. M9 research Tasks 1–3 plus foundation remediation Tasks 1–7 are now integrated into canonical `main`. Task 8 is implemented and verified on PR #20. M10 remains the first Linux + Android platform vertical slice after the remote-networking architecture is selected.

## Canonical Baseline

- `main` integration commit: `e45fa89d32efa185186ebb9d48e1cdbd985220fb`.
- Post-merge Rust CI `34805125848` passed lockfile verification, dependency audit, rustfmt, workspace check, Clippy with `-D warnings`, and the complete workspace test suite.
- PR #19 is merged. Its M9 Tasks 1–3 and foundation remediation Tasks 1–7 are durable on `main`.
- M9 Task 4 must not begin until delegated-role epoch authority is explicitly designed/resolved and the final remediation reconciliation/gate is complete.

## Active Plans

Primary M9 plans remain under `docs/plans/phase-1/`.

Active remediation plan:

`docs/superpowers/plans/2026-09-13-foundation-security-remediation.md`

Discovery/audit record:

`docs/plans/phase-1/M9-whole-project-security-quality-audit.md`

Existing Master Architecture, Identity/Keys, Pairing/Trust/Revocation, Policy/Authorization, Session/Transport, Protocol V1, Security Boundaries, Threat Model, and accepted ADRs remain authoritative.

## Completed Foundation Remediation

### Tasks 1–6

Completed and individually verified before the PR #19 integration checkpoint:

1. pairing currentness and fixed initial credential epoch;
2. verified credential rotation and active-session invalidation;
3. trusted-state and approval provenance;
4. receiver-local request replay authority;
5. explicit event subscription/authorization boundary;
6. Quinn plain-`Drop` connection/task ownership.

Their exact checkpoints and verification runs remain in git history and the remediation plan/evidence.

### Task 7 — Debug/privacy hardening

Regression-first RED commit:

`822775f12cf7d48a8142926d26f0dfae6365dc1e`

- CI `34803032034` passed the earlier gates and failed only the new privacy regressions because ordinary derived `Debug` output exposed protected session/pairing material.
- Fuzz remained GREEN.

Verified GREEN code head before integration:

`80c9facad7dc5f05cd5a948ff6ad91a35d61c3ad`

Implemented:

- removed derived `Debug` from `SessionAuthProof`, `SessionAuthTranscriptV1`, and `PairingTranscript`;
- added deliberately redacted custom `Debug` implementations;
- retained useful role/profile/protocol/length metadata only;
- signatures, transcript digests, nonces, binding-derived material, bootstrap/security bytes, and payload-like material are not emitted;
- transcript bytes, equality, digests, signatures, verification, and protocol behavior are unchanged.

Verification:

- Rust CI `34804692285` passed the complete gate;
- Fuzz Smoke `34804692267` passed;
- the integration was subsequently merged in PR #19 and post-merge `main` CI `34805125848` passed.

### Task 8 — Production secure-random identifier construction

Active branch / PR:

- branch: `m9-foundation-remediation-task8`;
- PR: #20;
- verified production code head: `d443637c3c7c8a7993e316c13aa67832d46a996e`.

Regression-first RED commit:

`673031dc9b27cf505378719b414a483d17d8839a`

RED evidence:

- CI `34805367644` passed lockfile, dependency audit, and formatting, then failed at workspace check with exactly three `E0599` errors for missing `RuleId::generate()`, `TransitionId::generate()`, and `OperationId::generate()`;
- Fuzz Smoke `34805367727` passed.

Implemented:

- added a policy-owned secure identifier generation helper backed by the existing OS-CSPRNG source in `crosslab-crypto`;
- added typed `generate()` constructors for `RuleId`, `TransitionId`, and `OperationId`;
- preserved `from_bytes`/byte conversion for verified parsing, wire conversion, and deterministic tests;
- kept `SessionId` derived rather than randomly generated;
- routed `AuthorizedOperation::issue` through `OperationId::generate()` instead of bypassing the typed construction API;
- no dependency, protocol, or architecture change was required.

Exact code-head verification:

- Rust CI `34805704494` passed lockfile verification, dependency audit, rustfmt, workspace check, Clippy with `-D warnings`, and complete workspace tests;
- Fuzz Smoke `34805704487` passed all bounded fuzz checks;
- dependency audit retains only the already documented allowed `paste 1.0.15` / `RUSTSEC-2024-0436` maintenance warning in the isolated Iroh experiment dependency graph.

## Exact Next Task

**Resolve authoritative delegated-role epoch state before the final remediation gate.**

The current identity APIs can verify a supplied `AuthorityDelegation` against a caller-provided minimum epoch, but that does not itself establish which delegation epoch local durable authority state has accepted.

Required design properties:

1. local durable state is authoritative for the accepted epoch of each delegated role slot;
2. accepting a replacement delegation verifies owner, role, root authority/signature, and strictly newer role epoch before mutating local state;
3. once epoch `N+1` is accepted, delegation epoch `N` fails closed everywhere that role authorizes sensitive work;
4. callers cannot widen authority by supplying their own minimum epoch/current delegation object;
5. identity/policy/core/storage boundaries remain clean and policy evaluation remains pure;
6. no networking-candidate type enters the authority model;
7. ADR-0009 remains reserved for the M9 networking decision, so this authority-state decision must use ADR-0010 or a later monotonic number.

Do not implement this authority-state change before recording and approving the architecture decision.

## Final Remediation Work After Authority-State Resolution

- implement the approved authority-state design regression-first;
- run the complete dependency/audit/format/check/Clippy/test/Fuzz gate on the exact final remediation head;
- reconcile `security/M9-FOUNDATION-HARDENING-ASSESSMENT.md` and this file;
- make M9 Task 4 the exact next implementation task only after the hardening gate is fully GREEN.

## Repository / M9 Invariants

- Quinn remains the verified local/LAN baseline.
- Iroh `1.2.0` remains an isolated M9 remote/NAT/relay candidate under `experiments/m9-networking` until ADR-0009 selects an architecture.
- Iroh identities, addresses, paths, relay metadata, and transport credentials never become Cross-Lab identity/trust/policy authority.
- No 0-RTT authority.
- M9 remote sessions remain `NetworkClass::Remote` for their lifetime.
- Transport/path changes do not silently mutate binding, `SessionId`, sequence, policy classification, or operation authority.
- A new transport connection requires fresh Cross-Lab authentication/authorization state.
- `main` branch protection remains an external repository-administration item.

## Resume Procedure

1. inspect canonical `main`, PR #20/current active branch, recent commits/workflows, and this file;
2. finish PR #20 integration only after its final exact-head gate is GREEN;
3. read the Master Architecture, Identity/Keys specification, relevant authority call paths, active remediation plan, and accepted ADRs before designing delegated-role epoch authority state;
4. record/approve that design in the next available ADR before production implementation;
5. implement authority-state behavior RED -> verified failure -> minimal GREEN -> focused verification -> full gate;
6. update this file at every meaningful verified checkpoint;
7. keep M9 Task 4 blocked until the entire remediation gate is durably resolved.
