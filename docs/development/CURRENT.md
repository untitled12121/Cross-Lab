# Current Development State

**Phase 2 Linux + Android MVP remains active, not released.** Git implementation and regression tests are authoritative; CI is a software checkpoint. The owner will provide separate normal Linux + arm64 Android physical acceptance results. **No real-device acceptance evidence has yet been received.**

## Integrated software checkpoint

- **`main` merge `1d5b45238228a3bf4b7b67034f480c8e071b35ad` (PR #104):** Android owner identity, policy, signed revocation and audit export tasks now run on a bounded serialized worker with explicit admission/retry state, never synchronously on the UI thread. Three JVM tests; exact PR source `73529a2f` passed Android and CI Gate in run `39320248627`.
- PR #103 merge `4790da9b`: Android Activity observer lifetime gate and exceptional-subscription teardown; six JVM tests, Android/gate passed run `39319156471`.
- PR #102 merge `47efe5db`: Linux verified-session status and safe reconnect/disconnect UX; source format, Rust and gate passed run `39318461588`.
- PR #101 merge `eb6932c7`: Android verified session, recovery UX and accessibility; Android/gate passed run `39316816232`.
- PRs #99 and #100 merges `33f33610` and `59e2b37b`: fail-closed Android runtime observer/permission handling and notification capture/redaction; Android/gate passed runs `39315312331` and `39314114314`.
- PRs #96–98 integrated exact-session Linux/Android clipboard/file handoffs and Android runtime transition recovery. Phase 2 #87–95 provided QR/LAN pairing, durable trust/revocation, precise owner permission control, resumable file transfer, v3.0 notification consent, and protected audit. Hardware acceptance is independent of these software checkpoints.

## Active milestone — protected Android audit owner-operation lifecycle

- **Branch:** `feat/phase2-android-owner-audit-recovery`, based on the merged PR #104. Source checkpoint immediately before this handoff: `d50faf99b6a10f1cdac840b6849f97b8607b1540`; the live branch head supersedes this after updating this file.
- **One owner action:** `OwnerAuditActionGate` serializes refresh, clear and export (including the Android document picker). Unique tickets make stale/duplicate asynchronous callbacks unable to complete a later operation, and `close()` rejects post-teardown callbacks and admissions.
- **Protected failure recovery:** results route through the Activity-lifetime UI callback gate. Refresh/clear failure hides retained audit rows and counter pending verified recovery; explicit audit error messaging avoids false success. A failed/redacted CSV export or declined document selection releases busy state and offers recovery instead of silently leaving the owner UI blocked.
- **Owner UI:** refresh, redacted export, and irreversible local clear are disabled while an audit action is in progress; action status is visible. Existing signed trust revocations remain outside audit clear.
- **Recorder:** initial audit observer callback failure no longer interrupts subscription; existing bounded serial writer and platform Keystore/currentness contract are unchanged.
- **Tests:** five pure JVM owner-action gate tests (parallel admissions, stale tickets, cancelled export, close invalidation, concurrency) and an audit observer-failure regression. **Exact-head Android CI/JVM and physical testing pending**. No accepted ADR, protected-store encoding, wire/identity or privilege-boundary revision.

## Exact next work

1. Run selected Android JVM compilation and CI Gate for this complete owner-audit batch; fix actual failures together, then merge after exact-head verification.
2. Continue Phase 2 Linux/Android transfer storage/cancel/recovery and notification/audit lifecycle resilience with focused tests. Avoid expanding into Phase 3.
3. **Owner-run physical gate remains open:** normal Linux + arm64 Android production builds, QR/LAN pairing and replay denial, saved trust/restart, fresh reconnect, exact peer default-deny, clipboard both directions, transfer resume/cancel/checksum/storage loss, notification OS consent withdrawal and protected content redaction, protected audit errors/clear, and owner-signed terminal revocation **last** on disposable identities. Record only redacted outcomes and tested commit/OS; do not mark Phase 2 or Cross-Lab 0.1 complete before real-device acceptance.

Refer to [AGENTS.md](../../AGENTS.md), [WORKFLOW.md](WORKFLOW.md), [CI.md](CI.md), approved ADR-0023, and [Phase 2 completion gates](../plans/phase-2/MVP-completion-gates.md).
