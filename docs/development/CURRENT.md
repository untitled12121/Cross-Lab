# Current Development State

This file is the durable resume guide for active Cross-Lab development. Git, code, and tests are the factual implementation state; this file records the intended handoff.

## Current Phase

**Phase 1 — Core Simulator / Real Transport Transition**

## Current Milestone

**M9 — Remote Networking ADR, paused before M9 Task 4 until foundation security remediation is fully reconciled.**

M1–M8 are complete. M9 research Tasks 1–3 and foundation remediation Tasks 1–8 are integrated into canonical `main`. The remaining foundation item is authoritative delegated-role currentness plus the final full hardening gate/reconciliation. M10 remains the first Linux + Android platform vertical slice after the remote-networking architecture is selected.

## Canonical Baseline

- Canonical `main` head after Task 8 integration: `7448eb7b3978c3563b6c61552d423c3d2f5d748f`.
- PR #19 is merged; M9 research Tasks 1–3 and foundation remediation Tasks 1–7 are durable on `main`.
- PR #20 is merged; foundation remediation Task 8 is durable on `main`.
- Post-merge Rust CI `34806566405` on exact `main` head `7448eb7b3978c3563b6c61552d423c3d2f5d748f` passed lockfile verification, dependency audit, rustfmt, workspace check, Clippy with `-D warnings`, and the complete workspace test suite.
- M9 Task 4 must not begin until delegated-role currentness is accepted/implemented and the final remediation reconciliation/gate is complete.

## Active Plans and Design

Primary M9 plans remain under `docs/plans/phase-1/`.

Active remediation plan:

`docs/superpowers/plans/2026-09-13-foundation-security-remediation.md`

Discovery/audit record:

`docs/plans/phase-1/M9-whole-project-security-quality-audit.md`

Current architecture-design branch:

`m9-authority-currentness-design`

Proposed authority decision:

`docs/adr/ADR-0010-authoritative-delegated-role-currentness.md`

Focused design specification:

`docs/superpowers/specs/2026-09-14-authoritative-delegated-role-currentness-design.md`

ADR-0010 remains **Proposed** until explicit owner review/acceptance of the written ADR/spec. No production authority-state implementation is authorized before that review gate.

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
- RED Fuzz Smoke `34803032038` passed.

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
- the integration was merged through PR #19 and its post-merge `main` CI passed.

### Task 8 — Production secure-random identifier construction

Regression-first RED commit:

`673031dc9b27cf505378719b414a483d17d8839a`

RED evidence:

- CI `34805367644` passed lockfile, dependency audit, and formatting, then failed at workspace check with exactly three `E0599` errors for missing `RuleId::generate()`, `TransitionId::generate()`, and `OperationId::generate()`;
- Fuzz Smoke `34805367727` passed.

Verified production code head:

`d443637c3c7c8a7993e316c13aa67832d46a996e`

Implemented:

- added a policy-owned secure identifier generation helper backed by the existing OS-CSPRNG source in `crosslab-crypto`;
- added typed `generate()` constructors for `RuleId`, `TransitionId`, and `OperationId`;
- preserved byte conversion/construction for parsing, wire conversion, and deterministic tests;
- kept `SessionId` derived rather than randomly generated;
- routed `AuthorizedOperation::issue` through `OperationId::generate()`;
- no dependency, protocol, or architecture change was required.

Final PR #20 documentation/head verification:

- final PR head: `a377d088dc06a1cca1b2c138b130eb94b91b14f7`;
- Rust CI `34806180048` passed the complete gate;
- Fuzz Smoke `34806180058` passed;
- PR #20 merged as `7448eb7b3978c3563b6c61552d423c3d2f5d748f`;
- post-merge `main` CI `34806566405` passed the complete Rust gate.

The dependency audit retains only the already documented allowed `paste 1.0.15` / `RUSTSEC-2024-0436` maintenance warning in the isolated Iroh experiment dependency graph.

## Delegated-Role Currentness Design

The authority audit confirmed a real currentness gap: `AuthorityDelegation::verify` validates against a supplied minimum epoch, but production callers can choose that value. Session authentication currently verifies a supplied issuer using the issuer's own epoch as the minimum, so a stale historical delegation can prove its own floor unless authoritative local state is consulted.

The approved design direction is **Option A: identity-owned `OwnerAuthorityState`**.

Proposed semantics:

- `crosslab-identity` owns the active `OwnerRootRecord` plus the currently accepted Device Signing, Administrative, and Recovery delegation slots;
- first local acceptance verifies owner/role/root signature and establishes the local role anchor;
- replacement requires a strictly higher role-specific epoch; equal/lower epochs fail closed;
- high-level authority-bearing APIs stop treating caller-supplied minimum epochs as currentness authority;
- Device Signing currentness applies to credential issuance/verification, pairing/trust establishment, credential rotation, and fresh session authentication;
- Administrative currentness applies to owner approval evidence;
- delegated trust transitions resolve their permitted signed issuer role through current local authority state;
- root succession uses existing `RootSuccessor` continuity and clears all delegated slots after successful root replacement;
- delegated-role rotation does not silently redefine an already Active session as revoked; explicit trust/revocation remains the active-session termination mechanism, while reconnect performs fresh authority validation;
- no database, daemon, CRDT authority merge, protocol field, signing transcript, or networking-candidate type is added by this design;
- durable rollback-resistant persistence remains a later platform requirement and must not be claimed solved by the in-memory Phase 1 state.

The written ADR/spec also records that credentials depending only on a superseded Device Signing delegation fail fresh identity validation after the newer delegation is locally accepted. This remediation does not invent a bulk credential-reissuance protocol or same-epoch device-key replacement rule.

## Exact Next Task

**Owner review/acceptance of proposed ADR-0010 and its focused design specification.**

Do not begin production implementation until the written design is explicitly accepted.

After acceptance:

1. invoke the implementation-planning workflow and create the regression-first authority-currentness implementation plan;
2. implement `OwnerAuthorityState` in `crosslab-identity` with atomic/fail-closed role replacement tests;
3. migrate credential, pairing/trust, approval, delegated revocation, and fresh-session authentication call paths away from caller-selected authority currentness;
4. verify stale/equal delegation rejection, role independence, root-successor clearing, failure atomicity, active-session non-reclassification, and unchanged signing/protocol vectors;
5. run the complete dependency/audit/format/check/Clippy/test/Fuzz gate on the exact final remediation head;
6. reconcile `security/M9-FOUNDATION-HARDENING-ASSESSMENT.md`, this file, and the accepted ADR/spec;
7. make M9 Task 4 the exact next implementation task only after the hardening gate is fully GREEN.

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

1. inspect canonical `main`, active branch `m9-authority-currentness-design`, open design PR if present, recent commits/workflows, and this file;
2. read the Master Architecture, Identity/Keys, Session/Transport, Pairing/Trust/Revocation, active remediation plan, proposed ADR-0010, and focused design before changing authority semantics;
3. keep ADR-0010 Proposed and production code unchanged until explicit owner acceptance of the written design;
4. after acceptance, write the detailed implementation plan before coding;
5. implement regression-first with small verifiable milestones and exact-head CI/Fuzz evidence;
6. update this file at every meaningful verified checkpoint;
7. keep M9 Task 4 blocked until the entire remediation gate is durably resolved.
