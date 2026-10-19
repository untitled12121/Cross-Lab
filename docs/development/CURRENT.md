# Current Development State

**Phase:** Phase 2 Linux + Android MVP (active; not released). Git code and test results are authoritative; see [the milestone gates](../plans/phase-2/MVP-completion-gates.md) for outstanding acceptance.

## Implementation baseline

- Integrated `main` at `cfd024270e7e998f9426c08a586260d3830a3f9c` includes PR #79 Phase 2 core/notifications/audit/revocation, PR #80-81 Android connectivity/scanner stability, PR #82-83 durable trusted-device inventory on both platforms, and PR #86 change-aware CI with manual verification scopes.
- **Active feature milestone:** [PR #87](https://github.com/untitled12121/Cross-Lab/pull/87), branch `feat/phase2-owner-permissions-recovery`, from `cfd0242`. Android exact peer capability controls, protected policy commits and tightened runtime guards; Linux serialized persist-before-apply policy edits and GPUI owner controls; Android DNS-SD stale-callback handling and Linux startup/reconnect recovery integrated from previous #84/#85 code.
- **Verification:** previous PR #87 source `a80b01e` and `4df287d5` failed Rust formatting; exact rustfmt diffs were corrected in source. One Android test job was still in progress at this checkpoint. Combined final head still needs exact-source format/Rust/Android verification and physical-device acceptance.

## Verification and blockers

- PR #86 full CI succeeded. Previous original heads for PR #84 and #85 passed CI but have merge conflicts with updated `main`; their product-code changes are being reconciled into the current batch rather than adding separate micro-PRs.
- Owner policy edits must require a fresh authenticated, connected, trusted exact peer and negotiated capability. Android and Linux use existing durable policy stores; no new privileged service, trust schema, capability wire protocol, or permission authority is introduced.
- **Release block:** no recorded successful post-fix physical Linux + arm64 Android QR/LAN pairing, reconnection, clipboard/transfer, notifications, audit fault recovery or terminal revocation acceptance evidence. CI cannot replace that evidence. Permanent signed revocation is tested last using disposable identities.

## Exact next work

1. Push the consolidated Phase 2 owner-permissions/recovery branch. Run **one** applicable integration verification checkpoint on its final head (format, Rust, Android, focused trust/policy regressions); correct confirmed failures together.
2. Merge the coherent batch when required checks are green, close earlier redundant recovery PRs only after source equivalence/integration is verified, and update this handoff with final branch/commit/CI status.
3. Continue Phase 2 permission/transfer/notification UX and error recovery, then use real owner hardware for release evidence. Defer Phase 3 until Phase 2 gate completion.

Use [AGENTS.md](../../AGENTS.md), [WORKFLOW.md](WORKFLOW.md), [CI.md](CI.md) for completion-first development. Uploaded references remain available; none should be removed merely because its feature is scheduled later.
