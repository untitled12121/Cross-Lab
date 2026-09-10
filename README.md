# Cross-Lab

Cross-Lab is an open-source, local-first, owner-controlled cross-device ecosystem for Linux, Windows, macOS, Android, and iOS/iPadOS.

The project is currently in **Phase 0 — Architecture and Security Specification**. Production feature implementation has not started.

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

## Development state

The current milestone, branch, verification state, and exact resume task are recorded in `docs/development/CURRENT.md`.

Development workflow and continuity rules are defined in `docs/development/WORKFLOW.md`. The roadmap is summarized in `docs/development/ROADMAP.md`.

## Contributing and security

Read `CONTRIBUTING.md` before proposing code or architectural changes. Security handling is documented in `SECURITY.md`.

The Cross-Lab repository license has not yet been selected. Do not infer a license from repository visibility, and do not adapt third-party research source code until the license and reuse requirements are resolved through the architecture process.
