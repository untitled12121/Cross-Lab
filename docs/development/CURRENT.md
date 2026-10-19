# Current Development State

**Active phase:** Phase 2 — Linux + Android MVP. **Not released.** Git code/tests are authoritative; this is a concise resume snapshot, not a historical log.

## Integrated baseline

- `main` application-source baseline: `7097d5653f0feeeac2fe57d12fd0a91c5b686105` (PR #83 merge). This workflow-documentation commit follows it without changing application code.
- PR #79: pairing/SDK recovery, presence, clipboard, resumable file transfer, notifications, private audit and terminal signed revocation integrated with Rust/Android/fuzz CI. ADR-0021/22/23 accepted.
- PR #80 and #81: Android LAN/lifecycle recovery and QR teardown/accessibility integrated.
- PR #82 and #83: saved trusted-peer inventory on Android and Linux merged; offline saved trust never implies authenticated online presence. Exact PR #83 CI `39222362996` passed Source format, Rust and Android.
- Last recorded real-device attempt predates the pairing bootstrap fixes: normal Linux Add Device showed a QUIC bind failure and Android camera launched. **No successful post-fix physical Linux↔Android acceptance is recorded.**

## Open implementation work

- [PR #84](https://github.com/untitled12121/Cross-Lab/pull/84): Android trusted-session DNS-SD serialized resolution, stale callback cleanup; source head `a846114324d7ae75f93bcb11b217dc05d8668caa`. Its latest CI run `39224169331` completed successfully. **Not yet merged at this checkpoint.**
- [PR #85](https://github.com/untitled12121/Cross-Lab/pull/85): Linux trusted-session startup/reconnect recovery controls; head `dc7a52840229ae0998387e5a0c4c962c6ffda606`. Latest CI run `39224027475` completed successfully. **Not yet merged at this checkpoint.**
- Remaining Phase 2: finish practical native owner capability/permission controls; product pairing/reconnect/recovery integration; clipboard/transfer/notification/audit lifecycle quality and accessibility; collect owner Linux + arm64 Android acceptance evidence. No Phase 3 features.

## Exact next task

1. Verify the open PR heads/mergeability remain unchanged, then integrate #84 and #85 after confirming their exact checks and conflicts. Avoid unrelated CI reruns.
2. Work in one substantial Phase 2 feature batch, prioritizing owner-facing per-device capability permission controls and cross-platform recovery, with targeted trust/policy tests during implementation.
3. Run consolidated verification when the batch is ready; record results, exact Git commit, blockers and follow-on work here. Hardware acceptance and irreversible revocation remain a release gate, with revocation last and disposable identities only.

## Workflow and references

- [AGENTS.md](../../AGENTS.md) and [WORKFLOW.md](WORKFLOW.md) define completion-first engineering; don't create micro-PRs or repeatedly poll CI.
- [Phase 2 gates](../plans/phase-2/MVP-completion-gates.md) and [acceptance worksheet](../research/Phase2-product-acceptance.md) define outstanding evidence.
- Uploaded research repositories are optional references, not source of truth. [Reference archive audit](../research/REFERENCE-ARCHIVES.md) records suggested Project Source cleanup; **none have been removed by this documentation change**.
