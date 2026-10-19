# Current Development State

**Phase:** Phase 2 Linux + Android MVP, active. **No release/physical acceptance claim.** Git source and verified tests remain authoritative.

## Merged software checkpoints

- `main` merge `b359b5f15f911879a6f4e1075a82758e38415019` (PR #92): Android notification delivery enforces current owner/content consent at final Rust-agent handoff, negotiated authenticated online trusted session, content redaction and immediate native subscription-state clearing. Final PR #92 head `d9e5bfba` passed Android unit CI and gate `39273346371`.
- PR #91 merge `5183b6054d53f63afc9bcb18bb5d94e9965f6fd1`: Linux and Android file-transfer integrity recovery, bounded provider read stalls and actionable transfer errors. Final head `f4de37af` passed format, Rust, Android and gate `39271444969`.
- PR #90 merge `459f4818`: protected Linux/Android audit interrupted-write recovery, run `39242293529` passed Rust, Android, format and gate. Previous #87–89 integrated exact peer permission, notification consent and transfer retained-state durability.
- Previously integrated Phase 2 foundation includes QR/LAN pairing, trusted sessions, clipboard, resumable single-file transfers, owner-signed revocation, saved inventory and metadata-only audit. Software CI **does not** validate hardware behavior.

## Active milestone: owner audit storage recovery

- **PR:** [#93](https://github.com/untitled12121/Cross-Lab/pull/93); branch `feat/phase2-audit-owner-retry`, originally based on main `5183b605`. Code batch checkpoint `6bcd8b193999d0da35e215fe2a1720e74488947a`; use the active PR head for the exact latest source.
- **Android:** bounded serialized owner-requested audit reload with narrow persistence seam, failure-injection JVM regression, native retry/error state, stale protected-history masking and export/clear disabling when persistence is unavailable. Clear confirmation cancels when storage access fails.
- **Linux:** owner-visible protected-audit retry via bounded existing command queue; failed initial/history/record/clear and export reads hide previously shown history and prevent destructive actions; revalidated read restores presentation. Added `OwnerFeatureState` regression for fail-closed masking/recovery.
- **Verification:** Android-only commit `822438df` passed Android and CI Gate in run `39273530188`, **before** Linux and subsequent audit UX changes. The complete Rust + Android batch has NOT yet passed exact-source CI or hardware acceptance. No protocol, trust, privileged-service or approved architecture change.

## Exact next work

1. Push the consolidated audit owner-recovery head to PR #93, execute the affected Rust formatting/check/tests and Android unit checks once, correct confirmed failures as a batch, then merge when passing.
2. Continue remaining Phase 2 platform UX, transfer/publication fault recovery and audit/notification lifecycle reliability, focusing on code rather than recurring CI.
3. **Physical release gate remains open:** normal Linux + arm64 Android QR/LAN pairing, trusted restart/reconnect, per-peer default deny and owner Allow/Deny, clipboard, file resume/cancel/checksum, notification OS withdrawal/redaction, protected audit storage faults, owner-signed terminal revocation **last** using disposable identities. Record only redacted results, exact versions and real pass/fail evidence.

See [AGENTS.md](../../AGENTS.md), [WORKFLOW.md](WORKFLOW.md), [CI.md](CI.md), and [Phase 2 completion gates](../plans/phase-2/MVP-completion-gates.md). Phase 3 remains deferred.
