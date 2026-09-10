# Contributing to Cross-Lab

Cross-Lab is in early architecture and foundation development. Contributions should preserve the project's local-first, owner-controlled, least-privilege design rather than optimize for short-term feature delivery.

## Before making changes

Read, in order:

1. `docs/architecture/MASTER-ARCHITECTURE.md`
2. `docs/development/CURRENT.md`
3. the active plan under `docs/plans/`
4. relevant ADRs under `docs/adr/`
5. `docs/development/WORKFLOW.md`

Inspect the current branch, recent commits, and repository state before modifying files. Repository state and tests take precedence over stale progress notes.

## Development rules

- Work in small, independently reviewable changes.
- Prefer feature-first organization and focused files.
- Reuse maintained dependencies where appropriate; do not copy research repositories blindly.
- Verify dependency versions, maintenance, platform support, license compatibility, security impact, and transitive cost before adding them.
- Keep domain crates independent from UI, platform APIs, concrete transports, databases, and privileged implementations unless the architecture explicitly requires otherwise.
- Do not introduce speculative crates, services, abstractions, or dependencies.
- Never silently change approved architecture. Material changes require an ADR and approval.
- Comments should explain non-obvious reasons, not restate code.
- Never commit secrets, private keys, credentials, tokens, signing material, recovery material, or sensitive captured payloads.

## Verification

Run the checks relevant to the current milestone before claiming completion. Once the Rust workspace exists, the normal baseline is expected to include:

```text
cargo fmt --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace
```

Security, protocol, integration, fuzz, or platform checks are added when the active milestone requires them. Record failed or unrun verification honestly in `docs/development/CURRENT.md`.

## Architecture decisions

ADRs live in `docs/adr/`. The ADR policy is defined in the Master Architecture and summarized in `docs/adr/README.md`. Superseded decisions remain in Git history and in the ADR directory.

## Licensing status

The repository license is currently unresolved. Until it is selected and contribution terms are documented, do not submit third-party source adaptations or assume that public repository visibility grants a reusable source license.
