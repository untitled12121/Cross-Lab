# Architecture Decision Records

Architecture Decision Records preserve material Cross-Lab design decisions and the reasoning behind them. They supplement the Master Architecture; they do not silently override it.

## Naming

Use:

```text
ADR-NNNN-short-title.md
```

Numbers are assigned monotonically. Once committed, an ADR remains in the repository even when superseded.

## Status

Use one of:

- Proposed
- Accepted
- Rejected
- Superseded

When an ADR is superseded, link to the replacement ADR instead of deleting or rewriting the historical decision.

## Required sections

Each ADR contains:

```text
# ADR-NNNN: Title

Status:
Date:

## Context
## Decision
## Alternatives considered
## Security impact
## Compatibility impact
## Operational impact
## Consequences
```

Keep ADRs focused on one material decision. Prefer concise reasoning and explicit consequences over broad design essays.

## When an ADR is required

Follow Section 52 of the Master Architecture. An ADR is required when a decision materially changes public protocol compatibility, identity or trust semantics, cryptographic formats, privilege boundaries, recovery authority, update trust, persistent data formats, transport/session abstractions, mandatory infrastructure, cross-platform API contracts, plugin authority, or major repository structure.

Implementation details that stay within an already approved boundary generally do not require an ADR.

## Approval and architecture updates

A proposed ADR does not authorize implementation by itself when the Master Architecture requires explicit approval. After approval, update the Master Architecture when the accepted decision changes its normative baseline.

## Current unresolved decision

The Cross-Lab repository license remains unresolved. Public repository visibility must not be treated as a license grant. Do not adapt third-party research source code until the project license and the relevant third-party license compatibility have been reviewed and approved.
