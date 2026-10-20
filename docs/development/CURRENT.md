# Current Development State

**Phase 2 Linux + Android MVP: software implementation, hardening, and full integrated CI verification are complete at the reviewed checkpoint below.** This is a **software-ready** status, not a Cross-Lab 0.1 release tag or a claim about observed behavior on actual devices. The owner will conduct separate device testing **after** software completion. Phase 3 is deferred under the approved architecture.

## Exact Git checkpoint

- **Verified product source:** `main` commit **`e10fa6253e6087cd073077944e5b19f80f962ff2`** (merged [PR #119](https://github.com/untitled12121/Cross-Lab/pull/119)). Later documentation-only merges can move `main` without changing this product source; inspect the live branch before new code work.
- **Full integrated verification:** GitHub Actions **push** [CI run 39452949225](https://github.com/untitled12121/Cross-Lab/actions/runs/39452949225), `head_sha=e10fa6253e6087cd073077944e5b19f80f962ff2`, **completed / success**. The workflow and `ci_scope` selected `full=true`; all four verification jobs and the strict CI Gate succeeded.
- At this checkpoint, repository search found no open product PRs or issues. This is not proof that software is defect-free; new confirmed failures should become focused Phase 2 fixes, not automatic Phase 3 scope.

## Implemented Phase 2 product software

The normal (non-development-provisioned) Linux + Android path integrates:

- Owner-controlled QR/LAN pairing, durable reciprocal trust, trusted/offline device inventory, authenticated Quinn/QUIC sessions and reconnect.
- Explicit default-deny per-peer capability permissions and exact session/operation checks; Android and Linux owner controls.
- Bidirectional text clipboard, resumable single-file transfer v2 with checksums, owner destination selection, cancellation, recovery and bounded worker/stream handling.
- Android-to-Linux foreground notification mirroring v3 with independent OS, owner, content and peer-policy gates; metadata-only protected owner audit/history, explicit owner export/clear, and terminal signed device revocation.
- Fail-closed native listener/session lifecycle recovery, bounded Android pairing/presence/clipboard/transfer work, and Linux/Android stale transfer-stream rejection with regression tests.

See merged PRs [#108–#114](https://github.com/untitled12121/Cross-Lab/pull/114), [#115](https://github.com/untitled12121/Cross-Lab/pull/115), [#116](https://github.com/untitled12121/Cross-Lab/pull/116), [#117](https://github.com/untitled12121/Cross-Lab/pull/117), and [#119](https://github.com/untitled12121/Cross-Lab/pull/119). Accepted Master Architecture v2.11 and ADR-0020–0023 remain unchanged.

## Software checks actually verified

**Exact integrated `main`, run 39452949225 — all success:**

- Scope-classifier and strict CI Gate regressions; Rust formatting; `cargo audit` dependency/security check.
- Rust lockfile/check and workspace type checks, FFI unsafe-boundary enforcement, generated Kotlin UniFFI bindings, development tool help and theme/SDK script validation.
- Linux GPUI **normal and development builds**, workspace Clippy with warnings denied, **full Rust workspace tests**.
- Android SDK bootstrap tests and Gradle **debug unit tests**, **normal arm64 APK assembly** and **development-wiring assembly**.
- Final `CI Gate`: verified every selected job **succeeded**, rather than accepting missing/skipped required jobs.

[PR #119](https://github.com/untitled12121/Cross-Lab/pull/119) also passed the same full matrix on its reviewed head in [run 39451594239](https://github.com/untitled12121/Cross-Lab/actions/runs/39451594239); the `main` push run above is the canonical evidence. The independent protocol fuzz workflow was **not** rerun as part of this CI profile; there were no new protocol-parser changes in the final release-CI fix. CI does not imply physical device behavior was exercised.

## Next owner and developer actions

- **Software work is complete at this reviewed checkpoint.** Do not reopen already integrated worker, transfer, pairing, permission, audit or UI milestones without a confirmed regression.
- The owner separately tests the normal builds when ready, using [product acceptance](../research/Phase2-product-acceptance.md) and [file-transfer evidence](../research/Phase2-file-transfer-device-evidence.md). This follows software completion and is **not a remaining software development task**.
- If owner testing uncovers defects, reproduce and fix those in coherent Phase 2 regression-tested batches. Cross-Lab 0.1 is **not released/tagged** until the owner approves that distinct acceptance/release decision. Phase 3 stays deferred until the approved phase exit.

For future development use [AGENTS.md](../../AGENTS.md), [WORKFLOW.md](WORKFLOW.md), [CI.md](CI.md), the [Phase 2 milestone](../plans/phase-2/MVP-completion-gates.md), and accepted architecture/ADRs.
