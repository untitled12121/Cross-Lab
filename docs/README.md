# Cross-Lab Documentation

This directory contains the durable architecture, protocol, decision, development, and milestone documentation for Cross-Lab.

## Start Here

For implementation or architecture work, read in this order:

1. [`architecture/MASTER-ARCHITECTURE.md`](./architecture/MASTER-ARCHITECTURE.md) — authoritative architecture baseline.
2. [`development/CURRENT.md`](./development/CURRENT.md) — current milestone, verification state, and exact next task.
3. The active milestone document under [`plans/`](./plans/).
4. Relevant accepted ADRs under [`adr/`](./adr/).
5. [`development/WORKFLOW.md`](./development/WORKFLOW.md) — repository continuity, branch, verification, and handoff rules.

Git history, code, and verification results remain factual if a progress document becomes stale.

## Document Authority

Cross-Lab documentation has clear responsibilities:

1. **Master Architecture** defines the governing architecture and dependency boundaries.
2. **Accepted ADRs** record approved material decisions and amendments.
3. **Focused architecture/security/protocol specifications** define normative behavior within their scope.
4. **Development state and roadmap documents** describe current execution state and sequencing; they do not override architecture.
5. **Milestone plans** define bounded delivery and acceptance criteria for a phase or milestone; they do not redefine normative security or protocol behavior.

Material changes to trust, protocol, privilege, recovery, transport, update, compatibility, or major repository boundaries require an ADR and explicit approval.

## Architecture

[`architecture/`](./architecture/) contains the system design and focused normative specifications:

- [`MASTER-ARCHITECTURE.md`](./architecture/MASTER-ARCHITECTURE.md) — Revision 2.1 source of truth.
- [`CORE-SIMULATOR.md`](./architecture/CORE-SIMULATOR.md) — Phase 1 deterministic simulator specification.
- [`IDENTITY-AND-KEYS.md`](./architecture/IDENTITY-AND-KEYS.md) — owner/device identities, authority hierarchy, credentials, rotation, and epochs.
- [`PAIRING-TRUST-REVOCATION.md`](./architecture/PAIRING-TRUST-REVOCATION.md) — pairing bootstrap, trust lifecycle, and revocation.
- [`POLICY-AUTHORIZATION.md`](./architecture/POLICY-AUTHORIZATION.md) — capabilities, policy evaluation, obligations, and operation-scoped authority.
- [`SESSION-TRANSPORT.md`](./architecture/SESSION-TRANSPORT.md) — logical sessions, authentication, channel binding, reconnect, and transport contract.
- [`SECURITY-BOUNDARIES.md`](./architecture/SECURITY-BOUNDARIES.md) — trust zones and privilege boundaries.
- [`RECOVERY-UPDATE-SECURITY.md`](./architecture/RECOVERY-UPDATE-SECURITY.md) — independent recovery authority and secure update model.
- [`AUDIT-PRIVACY.md`](./architecture/AUDIT-PRIVACY.md) — audit, privacy, and redaction requirements.
- [`PLUGIN-SECURITY.md`](./architecture/PLUGIN-SECURITY.md) — reserved future plugin authority boundary.

Phase 0 specifications were authored during the Revision 2.0 planning baseline and then reviewed and accepted into Master Architecture Revision 2.1 through ADR-0001 through ADR-0006 and the Phase 0 closeout. Where a completed Phase 0 document retains its original milestone/revision provenance or conditional proposal wording, the accepted ADR and Revision 2.1 baseline are authoritative for implementation.

## Protocol

[`protocol/PROTOCOL-V1.md`](./protocol/PROTOCOL-V1.md) defines the initial control-plane protocol contract, including:

- version and feature negotiation;
- bounded framing and parser limits;
- request, response, event, retry, cancellation, and replay semantics;
- operation-bound data-stream headers;
- canonical signing transcripts independent of protobuf serialization.

## Security

The foundation threat model lives at [`../security/THREAT-MODEL.md`](../security/THREAT-MODEL.md).

Security-sensitive design is additionally distributed across the focused architecture documents for identity, pairing/trust, policy, sessions, recovery/update, audit/privacy, and plugin boundaries.

Security issue reporting is defined in [`../SECURITY.md`](../SECURITY.md).

## Architecture Decision Records

[`adr/`](./adr/) contains durable decisions that materially affect the architecture. ADRs remain in the repository when superseded so design history is auditable.

The Phase 0 baseline currently includes accepted ADR-0001 through ADR-0006. See [`adr/README.md`](./adr/README.md) for status and policy.

## Development

[`development/`](./development/) contains execution-state documentation:

- [`CURRENT.md`](./development/CURRENT.md) — exact current resume point.
- [`ROADMAP.md`](./development/ROADMAP.md) — concise phase/milestone navigation.
- [`WORKFLOW.md`](./development/WORKFLOW.md) — repository continuity and verification workflow.

These files should stay concise and operational. Completed design history belongs in plans, ADRs, or focused specifications rather than accumulating in `CURRENT.md`.

## Plans

[`plans/`](./plans/) is organized by implementation phase:

```text
docs/plans/
├── phase-0/   # completed architecture/security milestone records and closeout
└── phase-1/   # active Core Simulator implementation milestones
```

Phase 0 is closed. New implementation planning belongs under `phase-1/` until the Core Simulator phase is complete.

## Research Repositories

External/open-source repositories used by the project are research and reference material unless explicitly adopted. Before reuse, review:

- license compatibility;
- maintenance and upstream health;
- security implications;
- performance and dependency cost;
- supported platforms;
- fit with Cross-Lab boundaries.

Cross-Lab does not inherit another project's architecture merely because its code is available.
