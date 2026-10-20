# Current Development State

**Phase 2 Linux + Android MVP is active; Cross-Lab 0.1 is not released.** Git and tests are authoritative. Real Linux + arm64 Android owner acceptance has not been supplied. Phase 3 remains deferred until the Phase 2 release gate closes.

## Branch, source and integrated scope

- **Latest reviewed and merged product source:** `main` at **`e732f2649b96557a46c478d2a5ed7d7af9920476`** (PR #117) before this documentation-only handoff. Inspect the live `main` SHA when resuming; documentation merges advance HEAD without changing product code.
- **Active product feature branches/PRs:** no open PRs or GitHub issues were found at this checkpoint. Do not treat this as proof that software has no remaining bugs.
- The normal (non-development-provisioned) Linux + Android product software contains QR/LAN pairing and durable trust, QUIC authenticated sessions/reconnect, offline trusted-device inventory, owner per-peer default-deny permissions, bidirectional clipboard, resumable single-file transfer v2, Android→Linux foreground notification mirroring v3 with independent OS/owner/content gates, protected metadata-only owner audit and signed terminal device revocation. **Merged software is not physical acceptance.**
- No accepted transport, owner-trust, pairing, permission, privilege or ADR-0020–0023 compatibility contract was intentionally changed in the hardening batches.

## Recent verified software milestones

| PR | Integrated change | Exact-head GitHub verification |
| --- | --- | --- |
| [#108–#114](https://github.com/untitled12121/Cross-Lab/pull/114) | Bounded Android transfer, presence, clipboard and pairing execution; late-callback, cancellation, snapshot, owner audit and cleanup-admission recovery; development handoff | Source/Android CI and path-aware gate passed on the individual code PRs; see their PR history and preceding checkpoints |
| [#115](https://github.com/untitled12121/Cross-Lab/pull/115) | Receiver cleanup is not considered finished merely because a close task was admitted; new transfers wait for actual release | Android tests + CI Gate succeeded in [run 39437949498](https://github.com/untitled12121/Cross-Lab/actions/runs/39437949498). Merge `93ec665f20f078ed9365c47f4ed5673f203306e0` |
| [#116](https://github.com/untitled12121/Cross-Lab/pull/116) | Fail-closed Android presence and clipboard native listener faults, UI-safe native callback recovery, startup snapshot and file-listener admission checks | Android tests + CI Gate succeeded in [run 39439917518](https://github.com/untitled12121/Cross-Lab/actions/runs/39439917518). Merge `ba266c392123be9fee03b502100457b17e6666b7` |
| [#117](https://github.com/untitled12121/Cross-Lab/pull/117) | Linux ignores stale file-stream events without failing active transfer; Android ignores duplicate stream Open, rejects missing/invalid durable offset; regression tests both platforms | Exact head `c026deb82af3b55c5d34b5da0608c9f57ee4a3a2` passed Source format, Rust, Android and CI Gate in [run 39446329014](https://github.com/untitled12121/Cross-Lab/actions/runs/39446329014). Merge `e732f2649b96557a46c478d2a5ed7d7af9920476` |

**PR #117 verification scope actually performed:** Rust workspace check, FFI unsafe-boundary check, Clippy, focused desktop tests, Android SDK bootstrap and Android JVM tests. **Not performed in that run:** dependency audit (skipped by path selection), full Rust workspace tests, Linux normal/development desktop build steps, Android normal/development APK assembly. The source formatting and selected CI Gate passed. Do not call this a full release matrix.

## Exact next task — final software release gate

1. On the latest integrated `main` source, run the repository's **manual GitHub Actions → CI → Run workflow → `full`** profile, per [CI.md](CI.md). Record the **exact tested commit** and outcomes of formatting, dependency/security audit, Rust workspace tests, Linux GPUI desktop builds and normal Android arm64 APK build. The present GitHub connector can review runs but does **not** expose workflow dispatch; **the manual `full` run has not been performed by this session.**
2. If any consolidated gate fails, fix confirmed problems in one coherent regression-tested batch, rerun the necessary gates and update this handoff. Keep security/policy/privilege errors fail closed; do not suppress checks to obtain a green status.
3. After software release checks pass, use the normal non-development product builds on real Linux + arm64 Android devices and record owner observations under [product acceptance](../research/Phase2-product-acceptance.md) and [file-transfer physical evidence](../research/Phase2-file-transfer-device-evidence.md). Physical QR, LAN, permission prompts, lifecycle, SAF provider behavior, notification access and terminal revocation are **not validated by host CI**.
4. Address any real-device failures before closing the release gate. Only then claim Phase 2 complete or publish Cross-Lab 0.1. **Do not start Phase 3 until the approved Phase 2 criteria close.**

## Owner-managed physical acceptance — still open

Test real QR pairing/expiration/forgery/replay, reciprocal persisted trust, restart and authenticated reconnect, exact peer default-deny permissions, clipboard both directions, empty and larger-than-3-MiB binary files with digest verification, transfer cancel/resume/retry under network loss and SAF document-provider behavior, foreground notification opt-in/redaction/withdrawal, protected audit read/clear/export/fault recovery and accessible UI/idle resources. Perform owner-signed **terminal revocation last**, only with disposable test identities. Record redacted pass/fail evidence with tested commit/coarse platform versions; never store QR payloads, private identifiers, network addresses, keys, clipboard contents, notification contents or real file paths in the issue/CI history.

See [AGENTS.md](../../AGENTS.md), [WORKFLOW.md](WORKFLOW.md), [CI.md](CI.md), [Phase 2 gates](../plans/phase-2/MVP-completion-gates.md), Master Architecture and accepted ADR-0020–0023.
