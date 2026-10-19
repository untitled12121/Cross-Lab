# Current Development State

**Phase:** Phase 2 Linux + Android MVP, active and **not released**. Source and tests are authoritative. Owner hardware acceptance remains a release gate.

## Integrated software

- `main` baseline `8564fc439f7353d18cfbc86fa03452654b1358ea`: PR #87 owner capability policies and Android/Linux discovery/startup recovery, merged as `be8afff`; PR #88 Android notification owner-consent atomicity and fail-closed in-process persistence, merged as `8564fc4`.
- PR #87 final source `a0b2fd0` passed format/Rust/Android and CI Gate in run `39238620079`. PRs #84/#85 closed as superseded by #87.
- PR #88 final source `add17b2c` passed focused Android and CI Gate in run `39240156393`.
- Earlier Phase 2 components include normal QR/LAN pairing, trusted-session transport, clipboard, resumable single-file transfer, notification mirroring, protected audit, signed terminal revocation, platform runtime recovery and saved device inventory. **Software CI does not establish real-device behavior.**

## Active vertical slice: file-transfer retention recovery

- **Branch:** `feat/phase2-file-transfer-retention-recovery` from `8564fc4`. Implemented source commit `005c020ccf56d8756a378288f76775eb3de075ca` (this handoff follows the code commit).
- **Android:** staged retained-state mutations (clone, mutate, persist, publish), so failed AtomicFile writes cannot expose an in-memory checkpoint or completion tombstone that was never stored; cleanup deletes expired partial files only after durable metadata commit.
- **Linux:** durable retained-state cleanup commits the removal before deleting partial files, preventing a failed state commit from destroying resumable data. Exact-file tests include injected write failure and subsequent successful retry.
- **Focused tests:** JVM model tests for staged commit success/rejection, and Rust regression for failed Linux retention commit preserving the partial.
- **Verification:** combined source **not yet CI-verified** at this checkpoint. No cross-platform hardware or storage-fault acceptance evidence claimed. No wire/protocol/trust architecture change.

## Next concrete task

1. Push this coherent branch and run the change-aware Rust + Android CI once; correct confirmed failures and merge after required checks.
2. Continue Phase 2 file-transfer publish/cancel/integrity recovery, notifications/audit lifecycle fault UX and permission state handling while avoiding repeated CI polling.
3. Before Cross-Lab 0.1, collect normal-product **physical Linux + arm64 Android** QR pairing, trusted restart/reconnect, per-peer default-deny, clipboard both directions, file transfer/resume/cancel/integrity, OS notification withdrawal/redaction, audit storage recovery, and irreversible revocation evidence using disposable identities (revocation last). Keep logs redacted; do not claim release until gates pass.

See [Phase 2 gates](../plans/phase-2/MVP-completion-gates.md), [AGENTS.md](../../AGENTS.md), [WORKFLOW.md](WORKFLOW.md), [CI.md](CI.md). Phase 3 remains deferred; uploaded reference archives are not removed merely for being future-phase material.
