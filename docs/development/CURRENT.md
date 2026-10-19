# Current Development State

**Active phase:** Phase 2 Linux + Android MVP. **Not released.** Git source and verified tests are authoritative.

## Integrated software

- **Main:** `c739ee6af7013bb5c4394bcda7219da20628b91c` (PR #95). Linux GPUI native clipboard/file controls use online, exact trusted peer, active authenticated session and negotiated capability, with stale-session UI gating. Final head `f2ad12c` passed format, Rust and CI Gate in run `39296128635`.
- PR #94 merge `09e1d810`: Android authenticated native clipboard/file-transfer operations, revalidation across worker handoffs and cancellation/publication safety; merged head `0c44f64` passed Android and gate in run `39288679882`.
- PR #93 merge `e79028e8`: protected audit recovery, fail-closed history masking, native retry and cancellation of pending destructive audit actions on Linux/Android; run `39287198643` passed format, Rust, Android and gate.
- PRs #87–92 integrated QR/LAN/pairing lifecycle, owner capability controls, permission persistence, notification owner/content and final-delivery privacy, Android/Linux file-transfer retention/integrity, bounded document streams and audit storage fault recovery.
- These are software checks. **Physical Linux + arm64 Android product acceptance is still outstanding.**

## Active milestone: exact-session native handoffs

- **Branch:** `feat/phase2-linux-exact-session-handoffs`, based on the verified PR #95 source `f2ad12c`. Implementation code commit `eca51ebc396894a54de68d82d0fe930058499af1`; this document follows that commit and the PR head is authoritative after pushing.
- Linux native devices feature retains an opaque `(DeviceId, SessionId)` binding only for live session handoffs. Custom debug rendering redacts session identity. Reconnect, peer handover, loss of authenticated capability or offline presence make old bindings unusable.
- Outgoing clipboard results, fetched text and error state, file-picker selections, and inbound peer clipboard OS access revalidate authority before changing local clipboard or executing protected native actions. Existing shared Rust authorization remains the final enforcement boundary.
- Focused Rust regression demonstrates stale-session/peer rejection, capability gating and zero raw session IDs in debug state.
- **Verification:** this new batch has **not yet passed exact-head Rust CI** or physical testing. No wire/protocol, identity, privilege or accepted architecture change.

## Next task

1. Push the complete exact-session native handoff batch and run one applicable Rust formatting/check/Clippy/desktop test checkpoint; correct confirmed failures together and merge once verified.
2. Continue Phase 2 Linux/Android recovery, native status/accessibility polish and owner control fault handling without redundant CI cycles.
3. **Release gate:** normal Linux + arm64 Android QR/LAN pairing, restart/reconnect and default deny, both clipboard directions, file resume/cancel/checksum, notification OS withdrawal/content redaction, audit storage corruption/recovery, and owner-signed terminal revocation last on disposable identities. Record redacted test evidence and exact versions. No 0.1 release claim until genuine hardware acceptance.

Follow [AGENTS.md](../../AGENTS.md), [WORKFLOW.md](WORKFLOW.md), [CI.md](CI.md) and [Phase 2 gates](../plans/phase-2/MVP-completion-gates.md). Phase 3 remains deferred.
