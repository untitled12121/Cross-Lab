# Current Development State

**Phase 2 Linux + Android MVP is active, not released.** Git code/tests and actual CI results are authoritative. The owner is performing physical Linux + arm64 Android verification separately and will provide redacted results. No device-acceptance results have been supplied yet.

## Integrated software

- **`main` merge `0b64e2137ffd60814e60cd61d3c4f50a10be8e16` (PR #105):** Android protected owner-audit refresh/clear/export share a single action gate across the export document picker, reject stale/duplicate callbacks, and recover safely from Activity destruction and storage failures. Six JVM regressions; exact head `333dd316` passed Android and CI Gate in run `39354380816`.
- PR #104 merge `1d5b4523`: bounded Android owner policy/revocation/inventory/audit-export worker with visible backpressure and three JVM tests; Android and gate passed run `39320248627`.
- PR #103 merge `4790da9b`: Android UI callback lifetime and exceptional observer teardown; Android and gate passed run `39319156471`.
- PR #102 merge `47efe5db`: Linux GPUI verified-session status, safe reconnect and revoked trust controls; source format, Rust and gate passed run `39318461588`.
- PR #101 merge `eb6932c7`: Android verified-session recovery UX and accessibility; Android and gate passed run `39316816232`.
- PRs #99/#100 merges `33f33610`/`59e2b37b`: fail-closed Android native owner permissions and notification consent/redaction, Android/gate passed runs `39315312331`/`39314114314`.
- Phase 2 #87–98 established Linux/Android secure QR/LAN pairing, exact session clipboard/files, per-peer permission/trust/revocation, resumable transfers, protected metadata audit, Android runtime lifecycle reliability. Software checks are not hardware release evidence.

## Active milestone — Android file transfer action and adapter fault recovery

- **Branch:** `feat/phase2-android-file-transfer-action-faults`, based on merged PR #105. Source checkpoint before this handoff `a0c13ed08a92bb611d5f15b602401e7b02214233`; the current branch head after this handoff supersedes this checkpoint.
- **Fail closed:** `FileTransferController` now traps exceptions from native send/retry/accept/resume/decline/cancel, initial adapter state and observation subscription, returning false without crashing the UI. It publishes redacted `FAILED/UNAVAILABLE` state with cleared pending request ID, file metadata, retry and cancel actions rather than preserving apparent active transfer authority.
- **Fresh native evidence:** native publication revision prevents a failed older action from overwriting newer verified native state. Subsequent real native status can restore availability; returned `false` without exception does not fabricate a transfer failure. Shutdown disables capability presentation and ignores late native events, isolating exceptional subscription close and synchronizing observer registration.
- **UX and tests:** the file-transfer panel offers specific adapter failure recovery guidance. Six JVM regressions cover thrown native actions, restart-by-publication, native false-return semantics, stale action failure vs new state, late callbacks/exceptional teardown, and failed native observation. **Exact-head Android CI and physical Linux + Android file-transfer checks pending.**
- No changes to ADR-0020 wire/profile, permissions, saved partial state, owner trust or privileged boundaries.

## Next work

1. Run Android compile/JVM and CI Gate for this consolidated transfer recovery branch; correct any confirmed failures and merge only after exact-head checks.
2. Continue approved Phase 2 reliability with bounded streaming/worker backpressure, cancel/storage failure recovery and native accessibility on Linux and Android. Keep release-sensitive cryptographic and authorization contracts unchanged without an approved ADR.
3. **Owner physical acceptance remains open:** normal Linux + arm64 Android QR/LAN bootstrap/replay rejection, trust persistence and reconnect, exact peer default deny, clipboard both ways, single-file resume/checksum/cancel/storage failure, Android OS notification consent/content redaction, protected audit faults, signed terminal owner revocation **last** with disposable identities. Record only redacted outcomes and tested commit/build; Phase 2/Cross-Lab 0.1 cannot be declared complete from CI.

See [AGENTS.md](../../AGENTS.md), [WORKFLOW.md](WORKFLOW.md), [CI.md](CI.md), [Phase 2 completion gates](../plans/phase-2/MVP-completion-gates.md), and accepted ADR-0020/0023. Phase 3 remains deferred.
