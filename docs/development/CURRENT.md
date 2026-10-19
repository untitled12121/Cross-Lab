# Current Development State

**Phase 2 Linux + Android MVP remains active, not released.** Git code and exact CI results are authoritative. Normal Linux + arm64 Android physical acceptance will be provided separately by the owner; no real-device evidence has been received.

## Integrated Phase 2 software

- **`main` merge `4790da9b775fcd1b2b3a9c265ddbc5f5506e48d2` (PR #103):** Activity-local callback lifetime guard prevents queued audit/runtime/clipboard/transfer/pairing UI updates after teardown. Subscription closes are independent, including exceptional closes; pending picker binding cleared. Six JVM regressions; source `90ebda38` passed Android and CI Gate in run `39319156471`.
- PR #102 merge `47efe5db`: Linux GPUI verified-session vs untrusted online presence, revoked control prevention and recovery status; source format, Rust and gate passed run `39318461588`.
- PR #101 merge `eb6932c7`: Android verified-session recovery/action UI and accessibility, Android/gate passed run `39316816232`.
- PR #99 merge `33f33610`: Android observer isolation and fail-closed owner permission/session recovery; Android/gate passed run `39315312331`.
- PR #100 merge `59e2b37b`: Android notification callback consent gating and redaction failure recovery; Android/gate passed run `39314114314`.
- PR #98 merge `a8eed54a`: Android network/foreground/reconnect/teardown native fault recovery; Android/gate passed run `39301328776`.
- PRs #97 and #96: Android/Linux exact peer/session clipboard and file picker guards, CI passed runs `39299050701` and `39298337209`. Earlier Phase 2 #87–95 include production QR/LAN pairing, owner trust/permissions, durable terminal revocation, resumable transfer, notification v3.0 and protected audit. Software CI is not hardware acceptance.

## Active milestone — bounded Android owner-operation admission

- **Branch:** `feat/phase2-android-bounded-owner-operations`, based on merged PR #103. Source checkpoint before this handoff `954df344537805b803fb42c24dcd44d4fc09a733`; use live branch/PR head after this merge and documentation update.
- **Owner work:** `OwnerWorkQueue` replaces the Activity's unbounded single-thread executor with one native-independent, serial worker and a bounded queue of eight pending jobs. Rejected/saturated/shutdown submissions never execute privileged work on the UI thread.
- **Recovery UX:** owner policy mutations, signed trust revocation, device inventory loading and redacted audit export now return visible retry/busy messages when queue admission fails; no false owner permission success. Accepted tasks remain serialized across Activity teardown, and the worker shuts down without dropping accepted jobs. UI results use the already-merged `UiCallbackGate`.
- **Tests:** three pure Kotlin JVM cases for bounded admission/backpressure, rejected post-close work and task-failure isolation. **Exact-head Android CI and real-device acceptance pending.**
- No approved wire, crypto, trust/identity or privilege architecture change.

## Exact next work

1. Run affected Android JVM/compile and CI Gate for the owner work queue; fix actual failures as a batch and merge verified changes.
2. Continue Phase 2 transfer, permission, notification and protected audit lifecycle recovery and UX with meaningful Rust/Android regression checks.
3. **Owner-manual hardware acceptance (open):** normal Linux + arm64 Android QR/LAN pairing with replay/expiration rejection, trust persistence/restart and authenticated reconnect, precise default-deny owner permissions, clipboard both directions, transfer integrity/resume/cancel/storage faults, OS notification consent withdrawal and protected-content redaction, protected audit recovery, and signed terminal peer revocation **last** on disposable identities. Record only redacted outcomes and exact tested commits/build. No Phase 2 complete/Cross-Lab 0.1 release claim until actual acceptance.

See [AGENTS.md](../../AGENTS.md), [WORKFLOW.md](WORKFLOW.md), [CI.md](CI.md), accepted ADRs, and [Phase 2 gates](../plans/phase-2/MVP-completion-gates.md). Phase 3 remains deferred.
