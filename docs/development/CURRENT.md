# Current Development State

**Active phase:** Phase 2 Linux + Android MVP. **Not released.** Git code and tests are implementation truth; see the [Phase 2 gates](../plans/phase-2/MVP-completion-gates.md).

## Integrated baseline

- **`main` software milestone:** `be8afff2ff1452eff3a4a4a102aef5db4e86de59` (PR #87 merge). Unified Linux and Android owner-facing exact peer capability controls; protected per-device policy persistence; Android serialized DNS-SD resolution, callback identity and session startup retry; Linux GPUI discovery and initial-start recovery.
- **Exact-head CI:** PR #87 head `a0b2fd0d2e72a9a6a05abf71434022a1b43235da` passed format, Rust and Android plus the CI Gate in run `39238620079`. Earlier recovery PRs #84/#85 closed as superseded by that integrated source.
- Earlier PRs #79–83 integrated pairing/transport/capabilities, notifications, bounded audit, signed terminal revocation, Android QR/LAN recovery and saved trusted-device inventory. PR #86 introduced change-aware/manual CI.
- These are **software verifications**, not a claim of normal-product physical Linux–arm64 Android acceptance.

## Active next vertical slice

- **Branch:** `feat/phase2-notification-consent-durability`, based on `be8afff`. Implementation code head prior to this handoff document is `44646cc42e7817c5569f8175d959ce3ab10abec1`; use the PR head for the exact current source.
- **Work completed in source:** atomic confirmed Android owner/content preference commits; owner disable and reapproval reset content sharing atomically; synchronized consent reads/writes and blocked capture until commit success; in-process fail-closed consent on storage read/write failures; native error notice; focused JVM tests for opt-out, reapproval, storage errors and commit races.
- **Verification:** new branch has **not yet passed Android unit CI** or physical notification mirroring tests at this source checkpoint. This does not add a wire protocol, identity store, trust mechanism, privileged service, or approved architecture change. A failed durability operation may require owner reapproval; no cross-restart guarantee under underlying storage failure is claimed.

## Release blockers and exact next task

1. Finish one consolidated Android check on the notification-consent branch; correct confirmed failures together and merge after required checks.
2. Continue Phase 2 practical file-transfer publication/recovery reliability, OS permission/lifecycle UX, audit failure recovery and cross-device integrity/regression polish. Prioritize code before repeated CI runs.
3. **Physical release gate:** normal Linux + arm64 Android QR/LAN pairing and reconnect, fresh trust, permission default-deny/revision, clipboard, file transfer/resume/cancel/integrity, notification OS withdrawal/redaction, audit corruption/write failure, and signed terminal revocation on disposable credentials (revocation last). No recorded successful post-fix physical acceptance. Record only redacted results and exact versions.

Follow [AGENTS.md](../../AGENTS.md), [WORKFLOW.md](WORKFLOW.md), [CI.md](CI.md). Keep uploaded reference archives unless a completed feature no longer needs them. Phase 3 remains deferred.
