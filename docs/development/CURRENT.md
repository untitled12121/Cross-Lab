# Current Development State

**Phase 2 Linux + Android MVP remains active; Cross-Lab 0.1 not released.** Git code and exact CI results are authoritative. Physical normal Linux + arm64 Android acceptance is owner-run and **has not yet been reported**.

## Integrated software

- **`main` merge `4d0214028c2287fe80bbf23ff063b5c341af5bd1` (PR #106):** Android transfer adapter exceptions fail safely, reject late native publications after controller shutdown and do not overwrite newer native state. Six JVM regressions; exact-head Android and CI Gate passed run `39355015967`.
- PR #105 merge `0b64e213`: Android protected owner audit read/clear/export are serialized across document-picker ownership and stale callbacks, with recovery UI and six JVM regressions; Android/gate passed run `39354380816`.
- PR #104 merge `1d5b4523`: bounded owner work queue for policies/revocations/audit export; Android/gate passed run `39320248627`.
- PR #103 merge `4790da9b`: Activity UI callback lifetime and exceptional close isolation; Android/gate passed run `39319156471`.
- PR #102 merge `47efe5db`: Linux verified-session status and safe native presence controls; source format/Rust/gate passed run `39318461588`.
- PRs #101, #100 and #99: authenticated status/accessibility, OS notification consent capture/redaction and observer/policy failure recovery; Android/gate passed runs `39316816232`, `39314114314` and `39315312331`.
- Phase 2 #87–98 delivered QR/LAN bootstrap, exact-session clipboard/file handoffs, per-peer owner permissions and durable revocation, resumable transfers, protected owner audit, v3.0 notification subscription and Android runtime lifecycle recovery. CI is software evidence, not a physical release gate.

## Active milestone — Linux transfer lifecycle and accurate loss reporting

- **Branch:** `feat/phase2-linux-transfer-disconnect-status` based on merged PR #106. Source checkpoint before this merge/docs commit `40580f05ef99df8de4de92cb8f9c605e22fb415b`; live PR head supersedes it afterward.
- **Native loss:** `FileTransferFeatureState` turns interrupted active sends/receives into `Failed(Connection)`, not owner `Cancelled`. Existing bounded progress remains visible, completed transfers are not rewritten, and no new offer/progress/completion/cancellation can revive an unavailable native transfer service.
- **Owner recovery:** desktop panel now explains disconnect/reconnect/retry steps; percentages use bounded u128 arithmetic so a counter exceeding file size or a large `u64` transfer cannot report impossible progress or overflow.
- **Audit contract:** added focused Rust coverage proving unexpected native service loss generates `TransferEnded/Failed` metadata, never `Cancelled` owner audit. No file names, private paths, stream IDs or content are logged.
- **Verification:** initial Linux PR #107 source `cd65abc2` failed Source format only (Rust skipped). Exact rustfmt corrections at `40580f05` passed Source format and Rust in run `39355478849`; merged-base checkpoint and additional audit regression are **not yet verified**. Physical Linux+Android acceptance remains pending. No ADR-0020 transfer wire/owner-policy authority revision.

## Next work

1. Verify exact-head Linux Source format, Rust and CI Gate for this consolidated audit/status batch; fix real failures and merge only when verified.
2. Continue bounded cancellation/backpressure, recovery and owner permission UX across native Linux/Android transport and services as approved in Phase 2, with focused regression tests.
3. **Owner physical acceptance gate remains open:** normal Linux + arm64 Android builds, replay-resistant QR/LAN bootstrap, durable trust/restart, reconnect and exact peer default deny, clipboard both ways, resumable file transfer including checksum/storage/cancel failure, notification OS consent withdrawal/text redaction, protected audit corrupt/rollback/clear recovery, and signed terminal owner revocation **last** using disposable identities. Preserve redacted outcomes, tested commit and coarse OS version. No 0.1 release/Phase 3 until real device gates pass.

See [AGENTS.md](../../AGENTS.md), [WORKFLOW.md](WORKFLOW.md), [CI.md](CI.md), [Phase 2 completion gates](../plans/phase-2/MVP-completion-gates.md), and accepted ADR-0020/0023.
