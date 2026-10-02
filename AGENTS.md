# Cross-Lab Agent Rules

Applies to every coding, review, and planning session in this repository. Read these instructions before changes, including in a new chat. Repository code, Git history, tests, and `docs/development/CURRENT.md` are durable state; prior chat messages are not.

## Architecture and quality

- Use `docs/architecture/MASTER-ARCHITECTURE.md` as the accepted governing architecture and read relevant plans and ADRs. Project-uploaded master architecture is primary reference material; reconcile it with explicitly approved repository revisions, not conjecture.
- Respect Rust-first monorepo/workspace boundaries, feature-first vertical slices, Linux/macOS/Windows GPUI + GPUI Kit desktop, Kotlin + shared Rust core on Android, Swift + shared core on iOS.
- Keep UI free of privileged/security/storage/transport business logic. Reuse, compose and refactor existing work; prefer concise, strongly typed, maintained code and bounded, event-driven resources.
- Identity, credential, trust, policy, privilege, session binding, cryptographic and protocol changes require explicit security review and approved ADRs when they change architectural contracts. Never log sensitive contents or keys; keep least privilege, owner control, and fail-closed behavior.
- Research repositories are reference material, not authority. Evaluate licensing, platform support, performance, security and maintenance before reuse.
- Match the sharp/radius-0 token-based native UI system; preserve accessibility, progressive disclosure and visible real state. Do not present placeholders as implemented capabilities.

## Efficient batch-first implementation (owner approved 2026-10-02)

**Default to completed, coherent feature batches over one-push-per-tiny-change.** For substantial work, prioritize roughly **60 minutes of active coding and review** per batch where practical; this is a guideline, never a deadline to cut security corners or claim completion.

1. **Start/resume once:** Inspect branch, status, recent commits, Master Architecture, `docs/development/CURRENT.md`, active plans/ADRs, and relevant source/reference repositories. Decide the smallest coherent end-to-end batch.
2. **Code and review first:** Implement related Rust core, platform adapter, UI integration, failure handling and meaningful tests together. Inspect issues as you go. Run lightweight targeted checks where useful. Test sensitive trust/privilege/protocol invariants at the point of change; never defer necessary security checks blindly.
3. **Local checkpoints:** Use small local Git commits to preserve completed progress; do not create many branches, PRs or pushes for minor steps. Keep recovery checkpoints durable. If session length, context or interruption threatens work, push a safe checkpoint rather than leaving the only copy in chat or stash.
4. **One consolidated verification pass:** Near the end of a meaningful batch run `cargo fmt --check`, applicable Clippy, affected Rust unit/integration/security tests, Android tests/Gradle and required desktop/Android builds. Run the full repository gate when integration warrants it, especially before a release/merge. Do not claim unrun checks passed.
5. **One consolidated push/CI:** Push the verified batch once (unless an earlier durable checkpoint is needed); let GitHub CI run, review the result after completion rather than repeatedly polling or retriggering full builds. If red, diagnose first and group fixes into another deliberate pass. Avoid speculative CI reruns. Documentation-only checkpoint commits may use `[skip ci]` when code is unchanged.
6. **Handoff/closure:** Update `CURRENT.md` at meaningful checkpoints with completed work, exact branch/commit/PR, tests and their actual status, blockers, pending work and next command/task. Never call Linux↔Android device behavior verified without physical observations.

CI duration (e.g., 30 minutes) is an estimate, not a timer. Do not idle needlessly or promise unattended/background work. Batching improves cost but does not eliminate meaningful testing or Git continuity.

## Current priorities

Phase 2 Linux + Android MVP is active. Fix real-product pairing and hardware blockers; complete notifications, privacy-safe audit/history, owner-facing production revocation/removal, integration, permission/transfer correctness and native UX. Broader visual polish follows functional reliability but must be completed before Phase 2 release criteria are claimed. Check `docs/development/ROADMAP.md`, `CURRENT.md`, and the latest approved ADRs for actual state. No Phase 3 until Phase 2 is complete.

## New-chat resume

Always start with `AGENTS.md` and `docs/development/CURRENT.md`, then inspect open PRs and CI status before changing implementation. If they disagree, Git/code/tests take precedence, reconcile the guide, and continue the exact unfinished task rather than restarting plans or assuming chat context.
