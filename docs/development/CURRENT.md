# Current Development State

**Phase 2 Linux + Android MVP is active, not released.** Git and tests are the implementation source of truth; this is the durable resume guide. The owner is performing separate physical-device verification and will provide redacted outcomes. **No real Linux + arm64 Android acceptance evidence has been received.** Phase 3 remains deferred.

## Latest integrated source and verified CI

- **`main` source merge `e29c08b1db0c0b4a102be5493e9229c0538f8e87` (PR #107):** Linux transfer-service loss now records interrupted sending/receiving as `Failed(Connection)`, not owner `Cancelled`; late progress and terminal callbacks cannot resurrect an unavailable transfer. File transfer status clamps byte counts, computes large percentages with `u128`, and offers explicit reconnect guidance. GPUI emits metadata-only failure audit transitions on service loss. Latest consolidated PR source `015eee05e609005f6bbdd673e9e113a26e036739` passed Source format, Rust, and CI Gate in run `39356181216`.
- PR #106 merge `4d0214028c2287fe80bbf23ff063b5c341af5bd1`: Android file transfer UI controller isolates adapter exceptions, presents redacted unavailable/failure state, rejects late native callbacks on shutdown, avoids stale failure overwrite, and offers recovery guidance; Android and CI Gate passed run `39355015967`.
- PR #105 merge `0b64e2137ffd60814e60cd61d3c4f50a10be8e16`: protected Android owner-audit refresh/clear/export serialize across the document picker, prevent stale/duplicate completions and hide history after protected-store read/clear faults; Android and CI Gate passed run `39354380816`.
- PR #104 merge `1d5b4523`: bounded Android owner work queue with explicit admission failure; Android and gate passed run `39320248627`.
- PR #103 merge `4790da9b`: Android Activity callback lifetime and teardown fault isolation; Android and gate passed run `39319156471`.
- PR #102 merge `47efe5db`: verified-session Linux GPUI status, reconnect and revoked controls; Source format, Rust and gate passed run `39318461588`.
- PR #101 merge `eb6932c7`: Android owner-facing verified-session recovery and accessibility; Android and gate passed run `39316816232`.
- PRs #99/#100 integrate Android observer/owner-permission fault isolation and OS notification consent/redaction; Android and gate passed runs `39315312331`/`39314114314`.
- Earlier Phase 2 #87–98 integrated product QR/LAN pairing/trust, per-peer capability/permission control, terminal owner revocation, exact-session clipboard/file-picker authority on Linux/Android, resumable single-file transfer v2, notification mirroring v3, protected owner audit, and Android runtime lifecycle fault recovery. Their software CI results are not physical release acceptance.

## Next approved Phase 2 implementation milestone

- **Branch:** `main` at source merge `e29c08b1db0c0b4a102be5493e9229c0538f8e87` before this handoff update; inspect the actual latest branch SHA and working status before starting a new feature branch.
- **Next coherent code batch:** tighten **Android file-transfer event/I/O/control worker backpressure and cancellation recovery**. Inspect `AndroidFileTransferService` (currently uses unbounded `Executors.newFixedThreadPool` / `newSingleThreadExecutor` queues), `AndroidFileTransferAdapter`, `FileTransferController`, relevant v2 source/receive state, and ADR-0020. Introduce bounded, event-driven admission without running I/O on the UI thread; rejected/closed queue tasks must not leave transfers stuck in PREPARING or retain apparent session authority. Preserve native owner approval, current-session binding, resumable checkpoints and final digest verification. Cover reconnect storms, storage/cancel errors and queue exhaustion with regression tests before merging.
- Continue Linux/Android UX and protected audit/notification lifecycle hardening where tied to this milestone; do not expand scope into Phase 3 or change approved wire/crypto/privilege contracts without an ADR.
- **Verification completed this handoff:** exact PR #107 Source format/Rust/CI Gate passed run `39356181216`; PR #106 Android/CI Gate passed run `39355015967`; PR #105 Android/CI Gate passed run `39354380816`. No local shell build or physical Linux/Android acceptance was performed in this environment.

## Owner-managed manual release gate (still open)

Test **normal non-development** Linux and arm64 Android builds on actual LAN-reachable devices: real QR bootstrap with expiration/forgery/replay rejection; trust persistence across restart; authenticated reconnect; exact peer default-deny permissions; clipboard both directions; resumable one-file transfer (cancel, retry, checksum, insufficient storage and permission withdrawal); Android OS notification access withdrawal and content redaction; protected audit corruption/rollback/recovery and clear/export; owner-signed terminal revocation **last**, using disposable identities. Record only redacted pass/fail evidence with the exact tested commit and coarse OS/build version—no QR payloads, IPs, private device identifiers, file contents or keys.

**Cross-Lab 0.1 and Phase 2 are not complete** until software/release gates and actual physical acceptance are satisfied.

See [AGENTS.md](../../AGENTS.md), [WORKFLOW.md](WORKFLOW.md), [CI.md](CI.md), [Phase 2 completion gates](../plans/phase-2/MVP-completion-gates.md), accepted ADR-0020, ADR-0021, ADR-0022, ADR-0023 and the Master Architecture.
