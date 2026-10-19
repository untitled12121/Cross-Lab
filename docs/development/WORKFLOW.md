# Development Workflow

This document provides the executable workflow behind [AGENTS.md](../../AGENTS.md). Prefer complete working functionality over commit count, CI run count and process ceremony.

## Resume and choose the next batch

Start once from the actual Git branch, changes, latest commits, open PRs, [CURRENT.md](CURRENT.md), [Master Architecture](../architecture/MASTER-ARCHITECTURE.md) and relevant ADRs. Git source and tests override stale handoff prose.

Identify the **next substantial unfinished Phase 2 milestone**. Outline only the necessary cross-layer work and risks, then implement directly. Do not read all research archives or redo repository-wide planning for each fix. Choose references as a specific code problem requires them.

## Develop the vertical slice

Implement the related Rust core, native platform adapters, feature state, UI, lifecycle, failure recovery and focused regression tests in one longer batch. Reuse existing code and remove obsolete duplication where appropriate.

- Keep authority/policy/cryptography/persistence outside views; review security-sensitive invariants as they change.
- Prefer small typed modules, bounded queues and event-driven work, not new background polling or speculative abstractions.
- Make focused **local** commits where useful for recoverability, but don't turn each edit into a new branch, PR or CI run.
- Do not wait idly for CI if independent useful work remains. Avoid working against a branch that is concurrently being rewritten.

## Consolidate verification and integration

Run fast affected checks during implementation as appropriate; keep identity/trust, permission, protocol and privilege-boundary regression checks close to the change. At a coherent integration checkpoint, run the applicable consolidated checks (not every command after every line):

```text
cargo fmt --check
cargo check --workspace --all-targets --all-features
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-features
```

Use targeted Rust package tests and relevant Gradle Android tests/builds for the changed slice; add integration/fuzz/platform checks when relevant. Prefer one final CI/review cycle per substantial PR. Diagnose actual failures and fix them together; do not suppress checks. State exactly which checks ran and whether they passed.

`main` is canonical. Use one substantive purpose-named feature branch when isolation helps. The `planning` branch is optional for substantial unmerged planning. Never rewrite/delete unique commits or force-push over someone else's work. Push durable partial work before interruption, clearly marking incomplete status.

## Architecture and source reuse

The accepted Master Architecture and ADRs are binding. A material trust, protocol, privilege, recovery, transport, compatibility, or update change requires an approved ADR before that changed contract is implemented. Keep the unaffected work moving.

Uploaded source archives are reference material, not runtime dependencies. Inspect a relevant upstream implementation only when it will improve current work. Check licensing, version, maintenance, security, platforms and transitive cost; prefer narrow wrappers over copying large codebases. Archived references can be re-cloned on demand.

## Handoff and release gate

Update [CURRENT.md](CURRENT.md) **at milestones, interruption checkpoints or handoffs**, not after tiny edits. Keep it short and current: active phase, branch + exact commit, completed functionality, actual verification, blockers, and next implementation task. Detailed historical logs belong in Git, merged PRs and ADRs.

Physical Linux + arm64 Android product acceptance remains necessary before releasing Cross-Lab 0.1: normal QR pairing, authenticated reconnect, explicit permissions, clipboard, resumable file transfer, notification consent, audit recovery and irreversible revocation last with disposable credentials. Do not substitute CI or synthetic provisioning for physical evidence. Keep Phase 3 deferred.
