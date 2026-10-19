# Current Development State

**Phase 2 Linux + Android MVP remains active, not released.** Git source, tests and merged commits are authoritative; this file is the durable resume guide.

## Integrated software checkpoint

- **`main` merge `2948be3c47c58f16d3a9f964a26aff082bba4226` (PR #97):** Android clipboard and owner file-picker work is bound to the exact authenticated peer/session exposed by Rust UniFFI, without storing session IDs in Compose state or diagnostics. Android tests and CI Gate passed at head `0ac5a5c6` (run `39299050701`).
- PR #96 merge `87a6eaea`: Linux GPUI clipboard callbacks, file-picker returns and incoming clipboard OS accesses now require live authenticated capability and exact session binding. Source format, Rust and CI Gate passed on head `3c2bbb00` (run `39298337209`).
- PR #95 merge `c739ee6a`: Linux trusted native capability controls; exact final run `39296128635` passed. PR #94 merge `09e1d810`: Android native trusted clipboard/file actions and verified-publication cancellation gate; run `39288679882` passed Android.
- PR #93 merge `e79028e8`: cross-platform protected audit recovery and owner retry; Rust/Android/format passed run `39287198643`. PRs #87–92 established pairing/transport, trusted permission editing, notification consent/event privacy and resumable transfer/integrity recovery. Physical product acceptance is **not** implied by these software checks.

## Active milestone: Android lifecycle failure recovery

- **Branch:** `feat/phase2-android-runtime-transition-fault-recovery`, based on PR #97 code head `0ac5a5c6`. Source checkpoint immediately preceding this document: `bf80f2ab222999cf051ca33099ed2614fa343dcb`.
- **Implemented:** Android runtime network loss/restore, peer disconnect/reconnect, foreground startup, background teardown, trust reload and shutdown now handle native exceptions by resetting observable active session state and attempting cleanup. Failed runtime stop terminates controller admission and invokes platform shutdown rather than misreporting a safe pause. Product presence isolates individual discovery, transfer and agent cleanup failures so teardown continues.
- **Lifecycle guard:** native status callbacks while stopped or terminal cannot resurrect apparently online trusted session UI.
- **Tests:** focused JVM regressions simulate network/reconnect failures, stop failure, retry and stale native snapshots. New batch **not yet exact-head CI verified** and not physically accepted.
- No approved architecture, wire, trust, privilege or capability profile changed.

## Exact next work

1. Run one affected Android unit/compile check for the complete lifecycle-fault batch, correct confirmed errors together and merge when verified. Keep source branch/head precise.
2. Continue remaining Phase 2 native UX/error recovery and practical file transfer/notification/audit lifecycle reliability, with code-first substantial batches rather than micro-PRs or broad CI reruns.
3. **Release acceptance gate:** normal Linux + arm64 Android real QR/LAN pairing, fresh trust, restart/reconnect, exact peer permissions/default deny, clipboard both directions, file resume/cancel/checksum and storage failures, Android OS notification withdrawal/redaction, protected audit faults, and terminal owner revocation **last** with disposable identities. Record only redacted evidence and actual versions; never mark Cross-Lab 0.1 ready from CI alone.

Refer to [AGENTS.md](../../AGENTS.md), [WORKFLOW.md](WORKFLOW.md), [CI.md](CI.md) and [Phase 2 gates](../plans/phase-2/MVP-completion-gates.md). Phase 3 remains deferred.
