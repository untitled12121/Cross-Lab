# Current Development State

**Phase 2 Linux + Android MVP software implementation and the documented security-hardening batch are integrated.** The earlier complete product baseline passed full CI; the subsequently merged security fixes each passed their required scoped exact-head CI. **The combined latest code has not yet received a fresh all-profile full CI run.** This is a **software-ready for owner testing** state, not independent security certification, a Cross-Lab 0.1 release tag, or a claim about actual-device behavior. The owner's separate device-testing stage follows software completion; Phase 3 remains deferred.

## Exact Git checkpoint

- **Latest merged product security source:** `main` commit **`87685d204122718e06b98527de6c3753235f55ff`** (merged [PR #122](https://github.com/untitled12121/Cross-Lab/pull/122)). PR #122 reviewed head `8b4ea839016c1cd9586d4ecfe180d390ee608650` passed source format, Rust workspace check, Clippy, focused Linux desktop tests and strict CI Gate in [run 39480992580](https://github.com/untitled12121/Cross-Lab/actions/runs/39480992580). Preceding merged [PR #121](https://github.com/untitled12121/Cross-Lab/pull/121) code head `f14214edc85f990fb9ec09ae5bf206738adcdb57` passed Rust, Android debug unit tests, format and CI Gate in [run 39476668443](https://github.com/untitled12121/Cross-Lab/actions/runs/39476668443). Both runs were scoped; new full-profile integration CI is not claimed. Inspect live `main` for newer changes.
- **Full integrated verification:** GitHub Actions **push** [CI run 39452949225](https://github.com/untitled12121/Cross-Lab/actions/runs/39452949225), `head_sha=e10fa6253e6087cd073077944e5b19f80f962ff2`, **completed / success**. The workflow and `ci_scope` selected `full=true`; all four verification jobs and the strict CI Gate succeeded.
- **Security review checkpoint:** both [PR #121](https://github.com/untitled12121/Cross-Lab/pull/121) and [PR #122](https://github.com/untitled12121/Cross-Lab/pull/122) are merged after exact-head selected CI passed. No Phase 3 scope is authorized.

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

## Security assessment and targeted fixes (20 October 2026)

Read [the source-grounded Phase 2 security assessment](../security/PHASE2-SECURITY-ASSESSMENT-2026-10-20.md) before treating this source as independently audited. The previous full-profile RustSec audit saw **1,064 Cargo.lock package entries, no reported vulnerability errors, nine unmaintained-crate warnings and one yanked-crate warning**; it did **not** evaluate Android Maven/Gradle dependencies.

- **Merged + scoped-CI verified:** PR #121 enforces Linux/Android committed identity currentness, refuses owner rebootstrap after protected key state survives missing files, bounds local identity/policy/signer/transfer metadata reads, and has Linux/Android adversarial regression tests. Merged SHA `b4aea96924ccc22f89a1d74f3e7c60d7cf2672f8`.
- **Merged + scoped-CI verified:** PR #122 adds a fail-fast OS file lock around the Linux policy compare-and-swap commit. Its competing-writer regression verifies conflicting writers are refused, and its reviewed merge is `87685d204122718e06b98527de6c3753235f55ff`.
- **Residual security review work:** triage yanked and unmaintained transitive dependencies, add Android dependency advisory/provenance scanning, and deepen OS-backed fault-injection and adversarial QUIC/fuzz coverage when safely reproducible. These are not claims of known remote exploitation.
- **CI distinction:** the previous full integrated product run 39452949225 remains valid evidence for **that exact earlier source**; do not claim the changed security source has rerun the entire full profile until another actual full run succeeds.

## Next owner and developer actions

- **Security batch complete at this checkpoint:** all three confirmed local storage issues are corrected and their reviewed exact-head CI jobs passed. Preserve the risk-based assessment's limitations; do not claim a line-by-line external security audit or all-profile CI on the newest combined source. Do not reopen unrelated completed milestones without a reproduced bug.
- The owner separately tests the normal builds when ready, using [product acceptance](../research/Phase2-product-acceptance.md) and [file-transfer evidence](../research/Phase2-file-transfer-device-evidence.md). This follows software completion and is **not a remaining software development task**.
- If owner testing uncovers defects, reproduce and fix those in coherent Phase 2 regression-tested batches. Cross-Lab 0.1 is **not released/tagged** until the owner approves that distinct acceptance/release decision. Phase 3 stays deferred until the approved phase exit.

For future development use [AGENTS.md](../../AGENTS.md), [WORKFLOW.md](WORKFLOW.md), [CI.md](CI.md), the [Phase 2 milestone](../plans/phase-2/MVP-completion-gates.md), and accepted architecture/ADRs.
