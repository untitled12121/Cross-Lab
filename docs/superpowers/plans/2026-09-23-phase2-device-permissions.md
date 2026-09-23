# Phase 2 Device Permissions / Capability Controls

**Status:** Active
**Date:** 2026-09-23
**Base:** PR #55 merged as `6abb3984a36be276aa439e9e1dabb42ffaa4a8b3`

## Goal

Expose owner-controlled per-device permission state on top of the existing exact Cross-Lab policy model without conflating capability support, trust, or presence with authorization.

## Constraints

- Keep `Capability = can` and `Policy = may`.
- No matching exact rule remains deny.
- Do not add wildcard/device-wide ambient authority.
- Policy changes must invalidate runtime request/subscription state that depended on the previous policy revision.
- Reconnect must receive the latest local policy state; a fresh authenticated session does not reset owner policy.
- UI receives presentation-safe rule/capability summaries only.
- Do not invent new public capability IDs, operation names, wire formats, or a persistent policy-store format in this slice. Those compatibility/persistence decisions require their own reviewed ADR when a concrete capability/store consumer needs them.
- No clipboard/file/notification implementation is added here.

## Task 1 — Exact policy mutation APIs

- Add typed exact-rule lookup/update/removal to `PolicyState`.
- Preserve rule constraints/obligations when only the effect changes.
- Keep revision changes monotonic and idempotent for no-op updates.
- Add regression coverage for default deny, effect changes, removal, and revision behavior.

## Task 2 — Runtime policy replacement

- Let `RuntimeNode` replace its local policy state.
- Cancel pending request/subscription state when policy changes.
- Add a bounded actor command for policy replacement.
- Ensure reconnect sessions receive the latest policy snapshot.

## Task 3 — Trusted presence policy ownership

- Let `TrustedPresenceAgent` start with a supplied `PolicyState` while keeping the existing default-deny constructor.
- Let the presence runner own the active policy and propagate replacements to the connected runtime.
- Add tests showing policy survives a reconnect within one agent lifetime and stale/no-rule state remains deny.

## Task 4 — Product presentation seam

- Expose safe negotiated capability identifiers and current policy revision/effects needed by Linux/Android feature state.
- Keep the UI generic: only concrete capability features may register editable operation descriptors.
- Until a concrete capability descriptor exists, show the default-deny permission posture rather than inventing capability operations.

## Verification

- `cargo fmt --check`
- `cargo check --workspace --all-targets --all-features`
- `cargo clippy --workspace --all-targets --all-features -- -D warnings`
- `cargo test --workspace --all-features`
- Android JVM tests + debug/development assemblies
- no new secret-bearing UI/FFI fields
