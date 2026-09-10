# Contributing to Cross-Lab

Cross-Lab is an architecture-first, Rust-first cross-device project. Contributions should preserve the project's local-first, owner-controlled, least-privilege design rather than optimize for short-term feature delivery.

## Before Making Changes

Read, in order:

1. `docs/architecture/MASTER-ARCHITECTURE.md`
2. `docs/development/CURRENT.md`
3. the milestone/implementation document relevant to the task
4. relevant ADRs under `docs/adr/`
5. `docs/development/WORKFLOW.md`

Inspect the active branch, recent commits, and repository state before modifying files. Git, code, and verification results take precedence over stale progress notes.

## Development Rules

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

Run the checks relevant to the current milestone before claiming completion. Once the Rust workspace exists, the normal baseline is:

```text
cargo fmt --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace
```

Security, protocol, integration, fuzz, networking, privilege-boundary, or platform checks are added when the active milestone requires them. Record failed or unrun verification honestly in `docs/development/CURRENT.md`.

## Architecture Decisions

ADRs live in `docs/adr/`. The ADR policy is defined in the Master Architecture and summarized in `docs/adr/README.md`. Accepted architectural changes that affect the baseline must update the Master Architecture in the same reviewed change. Superseded ADRs remain in the repository for historical context.

## Licensing

Cross-Lab is licensed under **MIT OR Apache-2.0**, at your option. Contributions are submitted under the same dual-license terms unless explicitly agreed otherwise.

Third-party source remains subject to its own license. Do not adapt source from research repositories merely because Cross-Lab is permissively licensed; verify compatibility and project policy first.
