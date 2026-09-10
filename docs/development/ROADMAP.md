# Development Roadmap

This file is a navigation view of the roadmap defined in `docs/architecture/MASTER-ARCHITECTURE.md`. The Master Architecture remains authoritative.

## Phase 0 — Specification

Freeze the architecture and security contracts before production feature work.

Foundation milestones:

- P0.1 Project foundation and continuity workflow
- P0.2 Threat model and trust boundaries
- P0.3 Identity hierarchy, credentials, roles, rotation, and epochs
- P0.4 Pairing, trust establishment, and revocation
- P0.5 Capability and authorization model
- P0.6 Protocol envelope, compatibility, errors, retries, cancellation, and replay semantics
- P0.7 Secure logical sessions, channel binding, control/data-plane authorization, and transport contract
- P0.8 Recovery authority, update trust, canonical signing, audit/redaction, and reserved plugin boundary
- P0.9 Core Simulator specification and Phase 0 review

## Phase 1 — Core Simulator

Prove identity, pairing, authentication, trust, capabilities, authorization, logical sessions, control messages, events, authorized data streams, reconnect, and revocation with deterministic simulated peers.

Foundation implementation sequence from the Master Architecture:

- M1 Repository foundation / minimal Rust workspace
- M2 Identity
- M3 Trust and policy
- M4 Protocol
- M5 Deterministic two-peer simulator
- M6 Authorized data streams
- M7 Failure and security lifecycle
- M8 Quinn transport
- M9 Remote networking evaluation ADR
- M10 First Linux desktop + Android vertical slice

## Later phases

- Phase 2 — Linux + Android MVP
- Phase 3 — Adaptive networking
- Phase 4 — Windows
- Phase 5 — Realtime media and peripherals
- Phase 6 — Display and remote control
- Phase 7 — Owner-hosted infrastructure
- Phase 8 — Cross-device authentication
- Phase 9 — Recovery implementation
- Phase 10 — Apple mobile completion
- Phase 11 — Advanced platform integration
- Phase 12 — Distributed compute
- Phase 13 — SDK and ecosystem

Do not pull later-phase dependencies or abstractions into an earlier milestone without a concrete requirement and, where architecture changes, an ADR.
