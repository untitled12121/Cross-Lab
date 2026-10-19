# Current Development State

**Phase 2 Linux + Android MVP remains active, not released.** Git code/tests are authoritative. The owner is handling normal real-device Linux + arm64 Android acceptance independently and will provide results; no physical acceptance evidence has been received.

## Integrated software

- **`main` merge `47efe5dbdb8fbb0c2867879db9c943734ed6b43c` (PR #102):** Linux GPUI shows verified live sessions distinctly from untrusted Online/discovery, paused/failed and terminal revoked peers; action labels and reconnect control no longer equate cached trust with session authority. Exact PR head `4b8affa7` passed Source format, Rust and CI Gate in run `39318461588`.
- PR #101 merge `eb6932c7`: Android actionable recovery status, authenticated-session controls, terminal revocation/shutdown handling and polite accessibility updates; Android/CI Gate passed run `39316816232`.
- PR #99 merge `33f33610`: Android observer failure isolation, owner policy mutation/snapshot fail-closed recovery and native session admission; Android/CI Gate passed run `39315312331`.
- PR #100 merge `59e2b37b`: Android OS listener capture gating, notification text redaction and lifecycle recovery; Android/CI Gate passed run `39314114314`.
- PR #98 merge `a8eed54a`: native Android network, foreground/background/reconnect and teardown failures; Android/CI Gate passed run `39301328776`.
- PRs #97 and #96: exact-session platform clipboard/file selection on Android and Linux; Android/Rust gate results passed runs `39299050701` and `39298337209`.
- Prior Phase 2 work #87–95 includes trust and QR/LAN pairing, precise capability permissions, resumable file transfer, metadata-only protected audit, notification v3.0, and durable signed owner revocation. All software checkpoints remain independent of physical acceptance.

## Active milestone — Android Activity UI observer lifetime isolation

- **Branch:** `feat/phase2-android-activity-observer-lifetime`. Source checkpoint before this handoff `a69eb81bde6bb0361d1626119b3d91d0f711bda2` (use active PR head for exact tree after this merge/docs commit).
- **Callback lifetime:** `UiCallbackGate` checks Activity ownership both before scheduling observer-driven UI work and after it reaches the UI thread, so a late audit/runtime/clipboard/file/pairing callback cannot mutate the disposed Activity.
- **Teardown:** `closeObservers` closes all observer handles even when an individual close throws, drops references, clears the pending file-picker session ticket, and permits the Activity worker to shut down. Observers are page-local, the native agent keeps independent policy/authorization authority, and no user secrets or session identifiers are exposed.
- **Tests:** six pure Kotlin JVM cases for late callbacks, correct ordering, broken observer delivery, rejected dispatcher, independent close and idempotent gate. **Exact-head Android CI and real-device verification pending**. No approved ADR, wire, trust or privilege contract changed.

## Exact next work

1. Run Android compile and JVM regressions for the Activity observer-lifetime batch, inspect CI Gate and fix actual failures, then merge verified code.
2. Continue remaining Phase 2 Linux/Android reliability and owner-facing recovery/accessibility, especially transfer storage/cancel and protected audit/notification OS lifecycle behavior.
3. **Owner physical release gate (open):** normal Linux + arm64 Android QR/LAN pairing and signed trust persistence, expired/replayed QR rejection, restart/reconnect and peer default deny, clipboard both ways, transfer resume/cancel/checksum/storage loss, notification OS withdrawal/redaction, audit protected faults, and owner signed terminal revocation **last** on disposable identities. Record only redacted outcomes and exact tested commit/OS build. No Phase 2 completion or Cross-Lab 0.1 release until accepted on real devices.

See [AGENTS.md](../../AGENTS.md), [WORKFLOW.md](WORKFLOW.md), [CI.md](CI.md), approved ADRs and [Phase 2 gates](../plans/phase-2/MVP-completion-gates.md). Phase 3 deferred.
