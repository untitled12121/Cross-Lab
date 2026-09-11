# Development Roadmap

This file is a concise navigation view. `docs/architecture/MASTER-ARCHITECTURE.md` remains authoritative.

## Phase 0 — Architecture and Security Specification

**Status: Complete**

Completed milestones:

- P0.1 — Project foundation and continuity workflow
- P0.2 — Threat model and trust boundaries
- P0.3 — Identity hierarchy, credentials, roles, rotation, and epochs
- P0.4 — Pairing, trust establishment, and revocation
- P0.5 — Capability and authorization model
- P0.6 — Protocol envelope, compatibility, errors, retries, cancellation, replay, and canonical transcripts
- P0.7 — Secure logical sessions, channel binding, control/data-plane authorization, and transport contract
- P0.8 — Recovery authority, update trust, audit/privacy, and reserved plugin boundary
- P0.9 — Core Simulator specification and Phase 0 closeout

Closeout record: `docs/plans/phase-0/PHASE-0-CLOSEOUT.md`.

## Phase 1 — Core Simulator

**Status: Active**

Implementation sequence:

- **M1 — Repository Foundation**: minimal Rust workspace, tooling, package shells, and verification baseline
- M2 — Crypto + Identity
- M3 — Trust + Policy
- M4 — Protocol
- M5 — Pairing + Authenticated Logical Session Simulator
- M6 — Authorized Data Streams
- M7 — Failure and Security Lifecycle
- M8 — Quinn Transport
- M9 — Remote Networking Evaluation ADR
- M10 — First Linux Desktop + Android Vertical Slice

Active milestone: `docs/plans/phase-1/M1-repository-foundation.md`.

`main` remains the canonical integrated branch. Use short-lived purpose branches only when a unit of work benefits from isolation; do not create branches merely to represent phase transitions or chat handoffs.

## Later Phases

- Phase 2 — Linux + Android MVP
- Phase 3 — Adaptive Networking
- Phase 4 — Windows
- Phase 5 — Realtime Media and Peripherals
- Phase 6 — Display and Remote Control
- Phase 7 — Owner-Hosted Infrastructure
- Phase 8 — Cross-Device Authentication
- Phase 9 — Recovery Implementation
- Phase 10 — Apple Mobile Completion
- Phase 11 — Advanced Platform Integration
- Phase 12 — Distributed Compute
- Phase 13 — SDK and Ecosystem

Later-phase dependencies and abstractions are not introduced early without a concrete requirement and, where architecture changes, an ADR.
