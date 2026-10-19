# Current Development State

**Phase 2 — Linux + Android MVP remains active. Not released.** Source and actual tests are authoritative; this is the durable resume guide.

## Integrated software baseline

- `main` merge `b62d9c8366e478dea54c336f762841861d92b666` (PR #89): Android file-transfer retained state now stages, persists and only then publishes mutations; Linux expiration cleanup commits metadata before removing resumable partial files. Exact PR #89 head `bb04dc5f` passed Source format, Rust desktop and Android unit tests, plus CI Gate in run `39241224235`.
- PR #88 `8564fc4`: Android notification owner/content consent confirmed and synchronized storage writes, in-process fail-closed storage/read errors, native feedback, and unit tests. Exact-head Android CI passed `39240156393`.
- PR #87 `be8afff`: Linux/Android owner capability permissions, Android DNS-SD request lifecycle, Android startup Retry and Linux reconnect/retry integrated; original #84/#85 closed as superseded. Exact-head format/Rust/Android CI passed `39238620079`.
- Earlier PRs cover QR/LAN transport, identity/trust and policy, clipboard, resumable file transfer, notifications, bounded audit and terminal signed revocation. Phase 2 software is substantial but physical Linux↔Android acceptance is **not verified**.

## Active batch: protected audit crash recovery

- **Branch:** `feat/phase2-audit-storage-recovery` from `b62d9c8`; code checkpoint `b92c5f310e3349c4b748cf9a17243ac9fe0da51e` (this handoff follows the code).
- **Android:** always let AtomicFile attempt backup recovery before treating missing base file as absent; only FileNotFoundException means an absent first-run file, other storage faults remain errors. If startWrite fails, clean up its newly generated audit anchor and propagate the failure. New pure-JVM tests cover read/absence/failure classification.
- **Linux:** under its existing exclusive audit lock, remove stale abandoned temporary writes before creating a new private file; clean temporary data after failed writes without weakening identity-anchor validation. New Rust tests cover crash-leftover replacement and failure that must not overwrite prior history.
- **Verification:** this audit batch is **not yet checked** by exact-head CI or physical audit storage faults. No trust, protocol, protected currentness policy, or approved ADR is changed. Ambiguous multiple anchors still fail closed.

## Exact next task and release blockers

1. Push and verify this consolidated Rust/Android audit-recovery batch with affected format/lint/unit checks, correct failures together, merge when green.
2. Continue Phase 2 product error handling/accessibility, file publishing/transfer fault recovery and notification/audit lifecycle correctness; avoid micro-PRs and unnecessary CI polling.
3. Before Cross-Lab 0.1, collect normal-product physical Linux + arm64 Android evidence for QR pairing, secure reconnect, permissions/default-deny, clipboard, file transfer/resume/cancel/integrity, notification OS withdrawal and redaction, protected audit corruption/write faults, and signed terminal revocation **last** on disposable identities. Redact logs; no release claim yet.

Follow [AGENTS.md](../../AGENTS.md), [WORKFLOW.md](WORKFLOW.md), [CI.md](CI.md) and [Phase 2 completion gates](../plans/phase-2/MVP-completion-gates.md). Phase 3 remains deferred. Keep uploaded reference repositories until their functionality is complete and they are no longer needed.
