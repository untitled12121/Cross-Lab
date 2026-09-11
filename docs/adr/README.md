# Architecture Decision Records

Architecture Decision Records (ADRs) preserve material Cross-Lab design decisions and the reasoning behind them. They supplement the Master Architecture and never silently override it.

## Current Baseline

Cross-Lab has accepted the following ADRs:

| ADR | Decision | Status |
|---|---|---|
| ADR-0001 | Repository license: `MIT OR Apache-2.0` | Accepted |
| ADR-0002 | Identity cryptographic profile v1: Ed25519, stable random IDs, BLAKE3 identifiers/transcripts | Accepted |
| ADR-0003 | Pairing bootstrap profile v1: single-use 256-bit secret with HMAC-SHA-256 confirmation | Accepted |
| ADR-0004 | Protocol Buffers wire encoding with independent canonical signing transcripts | Accepted |
| ADR-0005 | TUF-style update trust with separated roles and production root thresholding | Accepted |
| ADR-0006 | Focused shared `crosslab-crypto` crate boundary | Accepted |
| ADR-0007 | Protocol v1 event namespace and session-close wire registry | Accepted |

The governing architecture is `docs/architecture/MASTER-ARCHITECTURE.md`, Revision 2.1.

## Naming

Use:

```text
ADR-NNNN-short-title.md
```

Numbers are assigned monotonically. Once committed, an ADR remains in the repository even when rejected or superseded.

## Status

Use one of:

- Proposed
- Accepted
- Rejected
- Superseded

When an ADR is superseded, link to the replacement ADR instead of deleting or rewriting the historical decision.

## Required Sections

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

## When an ADR Is Required

Follow the Master Architecture change-control policy. An ADR is required when a decision materially changes public protocol compatibility, identity or trust semantics, cryptographic formats, privilege boundaries, recovery authority, update trust, persistent data formats, transport/session abstractions, mandatory infrastructure, cross-platform API contracts, plugin authority, or major repository structure.

Implementation details that stay within an already approved boundary generally do not require an ADR.

## Approval and Architecture Updates

A Proposed ADR does not authorize implementation by itself when explicit approval is required.

After an ADR is accepted:

1. update the Master Architecture when the decision changes the normative baseline;
2. update affected focused specifications in the same reviewed milestone;
3. update `docs/development/CURRENT.md` when the active implementation path changes;
4. preserve the ADR as the durable rationale for the decision.

## Licensing

Cross-Lab is licensed under **MIT OR Apache-2.0**, at the recipient's option, as accepted by ADR-0001. Third-party source remains subject to its own license and compatibility review.
