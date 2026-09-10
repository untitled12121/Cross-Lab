# Cross-Lab

Cross-Lab is an open-source, local-first, owner-controlled cross-device ecosystem for Linux, Windows, macOS, Android, and iOS/iPadOS.

**Phase 0 — Architecture and Security Specification is complete.** The project is ready to begin Phase 1 implementation with the deterministic Core Simulator foundation.

## Architecture

The authoritative architecture is maintained in `docs/architecture/MASTER-ARCHITECTURE.md`. Architectural invariants and approved boundaries must not be changed silently; material changes require an ADR and approval.

Core principles include:

- local-first and peer-to-peer-first operation;
- no mandatory Cross-Lab cloud account;
- maximum owner control with least-privilege internal boundaries;
- transport-independent identity and logical sessions;
- explicit separation of capabilities from authorization;
- control-plane authorization before data-plane operations;
- isolated privileged helpers and independent recovery authority.

## Development State

The current milestone, active branch, verification state, and exact resume task are recorded in `docs/development/CURRENT.md`.

Development workflow and branch rules are defined in `docs/development/WORKFLOW.md`. The roadmap is summarized in `docs/development/ROADMAP.md`.

The completed Phase 0 gate is recorded in `docs/plans/phase-0/PHASE-0-CLOSEOUT.md`.

## Contributing and Security

Read `CONTRIBUTING.md` before proposing code or architectural changes. Security handling is documented in `SECURITY.md`.

Research repositories are references, not automatic dependencies or source donors. Reuse requires license, maintenance, security, platform, and architectural review.

## License

Cross-Lab is licensed under **MIT OR Apache-2.0**, at your option. See `LICENSE-MIT` and `LICENSE-APACHE`.
