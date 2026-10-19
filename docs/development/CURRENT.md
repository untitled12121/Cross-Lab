# Current Development State

**Phase 2 Linux + Android MVP is active, not released.** Git source and tests are authoritative. The owner is carrying out physical Linux + arm64 Android acceptance separately; **no real-device acceptance has been reported**.

## Integrated software checkpoint

- **`main` merge `33f33610b37d33d8f4888e30b6faff8f5bb7c8fe` (PR #99):** Android observer failures no longer interrupt runtime, pairing, clipboard, transfer or native presence publication. Owner permission/snapshot faults fail closed; permission errors do not falsely report success; native session ticket errors deny admission. Exact merged PR source `c2c4cbdd` passed Android and CI Gate in run `39315312331`. PR #99 has been merged.
- **PR #100 merge `59e2b37b803c2f4d522f691d4b5f88e0d8108b06`:** Android notification listener callbacks and application teardown now isolate OS/native faults; capture and content consent are checked before payload inspection, and redaction remains enforced. Android and CI Gate passed run `39314114314`; merge push run `39315150856` also passed.
- PR #98 merge `a8eed54a`: Android foreground/background, network loss/recovery, reconnect, trust reload and native teardown fault recovery. Android and gate passed run `39301328776`.
- PR #97 merge `2948be3c`: Android exact peer/session clipboard and file picker handoffs, Android and gate passed `39299050701`.
- PR #96 merge `87a6eaea`: Linux GPUI exact-session callback and OS clipboard/file picker guards, Rust and gate passed `39298337209`.
- PRs #87–95 supplied authenticated product pairing and LAN discovery, native capability permissions, owner/OS notification consent, durable trust/revocation/policy, resumable transfers and protected metadata-only audit. Software checkpoints do not satisfy physical release acceptance.

## Active milestone — Android native status, recovery UX and accessibility

- **Branch:** `feat/phase2-android-device-recovery-ux`, based on the merged PR #99. Implementation tree before this handoff: `d953de59a7dd6ce58f083c51c1e1ea85d9c36695` (the document update changes the PR head; use the live Git ref for exact SHA).
- **State model:** `RuntimeRecoveryPresentation` distinguishes authenticated sessions, unverified/forged online presence, trust or session revocation, failed discovery, stopped native runtime, terminal fail-closed shutdown, LAN loss, and genuine authentication-in-progress. Only exact trusted, connected, active authenticated session snapshots offer Disconnect. Revoked peers and terminal shutdown never offer reconnect.
- **Android controls:** Devices screen gives concrete recovery instructions and bounded Retry/Reconnect/Disconnect affordances; Add Device is enabled only when native runtime is running and LAN is available. Scanner state clears when background or network/lifecycle disables pairing. Header and device panel share a single status classification.
- **Accessibility:** live polite status announcements, explicit accessible status descriptions, independent recovery-action control semantics and existing minimum 48dp touch target on ControlButton.
- **Tests:** seven pure JVM state regression cases for authenticated proof, revocation, retry, network loss, reconnect classes, terminal shutdown and missing peer-control adapter. **Exact-head Android CI and real-device testing are pending.** No wire/identity/trust/privileged contract changed.

## Next implementation actions

1. Review the Android compile/JVM results and CI Gate for the new recovery UX branch; fix confirmed failures together and merge only when verified.
2. Continue approved Phase 2 Linux/Android reliability and usability, especially native permission/error presentation, file transfer/notification/audit lifecycle recovery and accessible keyboard navigation. Keep batches coherent and source/tests authoritative.
3. **Owner physical release gate remains open:** real Linux + arm64 Android normal builds, QR/LAN pairing/replay rejection and reconnect, exact peer default-deny controls, clipboard both directions, file resume/cancel/checksum/storage failure, OS notification consent and redaction, protected audit recovery, then owner-signed terminal peer revocation **last** on disposable identities. Save only redacted outcomes and exact tested commit/build. Never mark Phase 2 or Cross-Lab 0.1 complete from CI alone.

See [AGENTS.md](../../AGENTS.md), [WORKFLOW.md](WORKFLOW.md), [CI.md](CI.md), [Phase 2 gates](../plans/phase-2/MVP-completion-gates.md), and accepted ADRs. Phase 3 remains deferred.
