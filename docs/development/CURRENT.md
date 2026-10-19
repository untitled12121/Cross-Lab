# Current Development State

**Active phase:** Phase 2 Linux + Android MVP. **Not released.** Git source and actual tests govern implementation truth; this document is the resume guide.

## Integrated baseline

- `main` at `459f4818393343b5360bc6e168541b848f52a750` (PR #90 audit storage recovery). Exact source `5548092a` passed Rust, Android, source format and CI Gate in run `39242293529`. Crash-leftover Linux audit temp files and Android AtomicFile backup recovery improved without weakening protected anchor currentness.
- PR #89 `b62d9c8`: Android and Linux retained file-transfer metadata commits before publishing changes/deleting resumable partials; run `39241224235` passed Rust, Android and format.
- PR #88 `8564fc4`: Android owner/content notification consent atomic confirmed updates with in-process failure blocking; run `39240156393` passed Android.
- PR #87 `be8afff`: Linux+Android peer capability permission controls, trusted discovery/session recovery and startup retry; run `39238620079` passed Rust/Android/format. Original #84/#85 closed as superseded.
- Earlier work includes QR/LAN pairing, saved device inventory, transport/clipboard, single-file resume, notifications, metadata audit and signed terminal revocation. Software integration is substantial; **real-device acceptance is not yet verified**.

## Active vertical slice

- **Branch:** `feat/phase2-transfer-integrity-recovery`, based on merged `main` `459f4818`.
- **Implementation checkpoint:** `eedc16ee5693e650e778f94ac62be047323e5744` (this handoff follows that code).
- Android transfer source hashing, resumed source reads, publication and digest verification now use bounded no-progress reads; repeated zero-byte document-provider replies fail as safe local storage errors instead of keeping a transfer worker spinning indefinitely.
- Android final-digest rejection durably invalidates retained state **before** deleting the failed partial; a failed retention commit leaves the existing snapshot/data consistent. Linux integrity rejection likewise commits removal before destructive truncation/deletion.
- Focused JVM tests cover temporary zero reads, exhausted read retries and actionable native file-transfer error hints. Linux tests inject a protected checkpoint cleanup failure then confirm safe retained state, subsequent digest failure and clean retransmission with verified content.
- **Verification:** this new batch has **not yet passed exact-head CI** at the time of this handoff. No physical-device testing claimed, no accepted ADR or wire protocol changes.

## Next task and release blockers

1. Push and integrate this coherent file-integrity recovery batch after affected Rust format/check/Clippy/tests and Android unit tests. Fix confirmed failures together; avoid unrelated CI reruns.
2. Continue Phase 2 product completeness: destination publication/cancel fault recovery, native owner error/accessibility polish and audit/notification lifecycle reliability.
3. **Physical release gate remains open:** normal Linux and arm64 Android QR/LAN pairing, durable trust/reconnect, per-peer default deny and owner Allow/Deny, clipboard both directions, resumable file cancellation/checksum, OS notification withdrawal/redaction, protected audit storage failure and signed revocation **last** on disposable identities. Do not claim Cross-Lab 0.1 release readiness from CI alone.

Follow [AGENTS.md](../../AGENTS.md), [WORKFLOW.md](WORKFLOW.md), [CI.md](CI.md) and [Phase 2 gates](../plans/phase-2/MVP-completion-gates.md). Phase 3 deferred. Keep uploaded reference archives until completed features no longer need their source.
