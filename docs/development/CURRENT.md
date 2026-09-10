# Current Development State

This file is the resume guide for active Cross-Lab development. Git history, repository contents, and tests are the factual implementation state when they conflict with this document.

## Current phase

**Phase 0 — Architecture and Security Specification**

## Current milestone

**P0.2 — Threat Model and Trust Boundaries**

Status: architectural design written and committed; awaiting design/spec review before implementation planning.

## Active branch

`phase-0/threat-model`

## Base state

P0.1 was merged to `main` through Pull Request #1 on 2026-09-10.

Merge commit:

`dba24fb77a8b3b478d1be7b48f8bfc8260911ece`

## Last verified content checkpoint

`b8d19b10095cb169209494a46d0a8185ef0a071d` — P0.2 threat-model and trust-boundary design committed for review.

The authoritative architecture remains `docs/architecture/MASTER-ARCHITECTURE.md`, Revision 2.0.

## Completed

- Verified that Pull Request #1 is merged into `main`.
- Created the clean `phase-0/threat-model` branch from the merged `main` baseline.
- Reconciled the P0.2 scope against the Master Architecture and P0.1 handoff.
- Selected a foundation-first threat-model scope rather than an exhaustive future-feature model or a simulator-only model.
- Defined P0.2 deliverables, protected assets, actors, attacker classes, trust zones, authority boundaries, mandatory security invariants, threat categories, authorization flow, compromised-device assumptions, reconnect/revocation requirements, privileged-service boundary, recovery boundary, relay/hub assumptions, audit/privacy requirements, resource-safety requirements, and threat-to-test traceability.
- Kept cryptographic formats, pairing transcripts, wire serialization, platform-specific privilege APIs, production Rust, and dependency selection out of P0.2 design scope.

## In progress

- User review of `docs/plans/phase-0/P0.2-threat-model-design.md`.

## Exact next task

After the P0.2 design is approved, create the detailed P0.2 implementation plan and then produce:

- `security/THREAT-MODEL.md`
- `docs/architecture/SECURITY-BOUNDARIES.md`

The implementation plan must preserve the approved design and verify both documents against the Master Architecture without silently choosing deferred cryptographic or wire-format details.

## Known issues / open decisions

- The Cross-Lab repository license is not yet selected. No third-party research source code may be adapted while this remains unresolved.
- Exact cryptographic algorithms, credential formats, pairing transcript, key schedule, and protocol serialization remain later Phase 0 decisions.
- Remote NAT/relay architecture remains intentionally unresolved until the later networking evaluation checkpoint.
- Platform-specific privileged IPC mechanisms remain deferred to platform milestones.

## Verification

P0.2 remains documentation/specification-only; no Rust build/test suite exists yet.

Completed checks:

- P0.1 integration state: **PASS** — Pull Request #1 is merged to `main`.
- P0.2 branch origin: **PASS** — `phase-0/threat-model` was created from merged `main`.
- Scope review: **PASS** — design is limited to foundational security boundaries and explicitly defers feature-specific/platform-specific details.
- Architecture alignment review: **PASS** — transport-independent identity, capability/permission separation, control/data-plane authorization, privilege isolation, recovery separation, route-security precedence, third-party boundary isolation, and fail-closed behavior are represented.
- Placeholder scan: **PASS** — no TBD/TODO placeholders or unstated implementation requirements remain in the design.
- Premature-decision review: **PASS** — no exact cryptographic primitive, pairing transcript, wire format, or platform IPC mechanism is selected.

Not applicable in P0.2 design stage:

- `cargo fmt --check`
- `cargo clippy`
- `cargo test`

These become mandatory when the Rust workspace is introduced.

## Active design

`docs/plans/phase-0/P0.2-threat-model-design.md`

## Relevant ADRs

None accepted yet for P0.2. The design remains within the approved Master Architecture and does not change an architectural invariant.

## Resume procedure

Before continuing in any new session:

1. Read `docs/architecture/MASTER-ARCHITECTURE.md`.
2. Read this file.
3. Read `docs/plans/phase-0/P0.2-threat-model-design.md`.
4. Read relevant ADRs if any have been added.
5. Inspect `phase-0/threat-model`, recent commits, and repository state.
6. Reconcile this document with actual repository state.
7. Continue only after the P0.2 design review state is clear.
