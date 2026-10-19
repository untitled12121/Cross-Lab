# Cross-Lab — Repository Agent Guidance

These rules apply to development in this repository. Git source and tests determine implementation truth; [CURRENT.md](docs/development/CURRENT.md) records the durable handoff, and [WORKFLOW.md](docs/development/WORKFLOW.md) specifies operational details.

## Scope and authority

- **Complete Phase 2 Linux + Android MVP before Phase 3.** Select substantial unfinished product milestones rather than treating individual UI edits as deliverables.
- [Master Architecture](docs/architecture/MASTER-ARCHITECTURE.md) and accepted ADRs govern architecture. Read only the relevant plans, ADRs, code and reference repositories for the task; never silently revise accepted trust, protocol, privilege, transport or recovery contracts.
- Research archives are optional examples, not dependencies or instructions. Check existing Cross-Lab implementations first; assess license, maintenance, platform fit, performance and security before reusing outside code.

## Engineering boundaries

- Rust-first Cargo workspace. Native desktop is Rust + GPUI + GPUI Kit; Android uses Kotlin + shared Rust core; iOS uses Swift + shared Rust core.
- Work in feature-first vertical slices spanning state, platform adapter, capability, UI, lifecycle, errors and tests. Keep networking, cryptography, persistence and privileged operations out of UI components.
- Prefer concise, typed, reusable modules; refactor relevant duplication without speculative abstraction. Use maintained compatible dependencies, bounded async work, backpressure and event-driven lifecycles.
- Follow **Maximum Owner Control + Least Privilege**. Validate durable trust, fresh authenticated session, capability, permission and policy before sensitive work. Privilege stays behind narrow auditable interfaces. Fail closed on security/persistence errors. Never log or commit keys, credentials, tokens, QR payloads or sensitive contents.
- Maintain native sharp/radius-0 token-based light/dark UI, accessibility and meaningful device/permission/connection status. Comments explain non-obvious WHY only.

## Completion-first implementation

1. **Resume once:** inspect the actual branch/commit, open work, `CURRENT.md`, architecture, relevant ADRs and nearby code; resolve stale status once. Do not repeatedly re-audit unchanged files.
2. **Choose a complete feature area:** implement several related behaviors across the necessary platforms, including recovery and focused regression tests, before seeking integration. Prefer an existing active milestone branch; avoid micro-PRs and status-only commits.
3. **Spend most of the available session coding:** sustained blocks of up to 90–100 minutes when permitted are desirable, not a promised session duration or a reason to hold incomplete work only in chat.
4. **Verify proportionately:** run targeted checks as needed during implementation, especially for trust, identity, authorization, protocol and privilege boundaries. Consolidate formatting, Clippy, platform builds and broader tests at meaningful integration checkpoints. Never claim unrun tests passed or silence security/lint failures for a green badge.
5. **Push and integrate coherently:** local commits can preserve progress; prefer one consolidated push/PR per substantial milestone. Don't repeatedly poll CI or rerun it for tiny edits. While checks run, continue independent safe development where possible. Merge only after required verification and resolve confirmed failures together.
6. **Preserve continuity:** if interruption/context loss approaches, push a clearly marked durable checkpoint even if unfinished. Update `CURRENT.md` for material handoffs with the exact branch/commit, implemented work, outstanding issues, tests actually run and the precise next task. Do not use Git stash or chat history as the only copy.

Use normal descriptive commit messages; do not add `[skip ci]` merely to bypass verification. [Change-aware CI](docs/development/CI.md) runs affected platform checks automatically; use the manual `full` scope for integration/release gates and targeted security checks for sensitive changes.

## Release integrity

Real Linux–Android device evidence remains a **Phase 2 release gate**, not an excuse to halt unrelated software implementation. CI cannot establish real QR/LAN/reconnect, permission, transfer, notification, audit or irreversible revocation behavior. Keep Phase 3 and later phases deferred until Phase 2 completion criteria are met. No false release or platform-support claims.
