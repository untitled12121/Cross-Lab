# M9 Foundation Security Hardening Plan

**Status:** Complete — verified before M9 Task 4  
**Scope:** Correctness, security, resource ownership, privacy, CI/supply-chain, and repository hygiene findings discovered during the Tasks 1–3 review.

## Goal

Close the identified foundation weaknesses with durable fixes and regression coverage before any Task 4 implementation. This plan does not change approved architecture; it enforces the existing Master Architecture, session/transport, policy/authorization, audit/privacy, and M9 design invariants.

## Constraints

- Work only on the existing `m9-remote-networking` branch; do not create scratch branches or issues.
- Follow RED -> verified failure -> minimal GREEN for behavioral fixes.
- Keep network/trust policy inputs locally authoritative and fail closed on stale/revoked trust.
- Do not expose transport-library types through domain APIs.
- Release terminal operation/task state so bounded capacities remain reusable.
- Do not log endpoint descriptions or other sensitive transport metadata through ordinary `Debug` output.
- Pin mutable CI inputs where practical and add a RustSec vulnerability gate.
- Task 4 remains blocked until this hardening pass and the full verification gate are green.

## Completed Tasks

1. **Authorization provenance**
   - `SimNode` network classification is explicit at construction.
   - Inbound control validates current local peer trust against the authenticated owner/device and session trust revision before policy dispatch.
   - Pending/revoked trust fails closed.
   - Regressions cover `Remote + LocalOnly`, revocation before explicit teardown, and wrong trust-record provenance.

2. **Bounded operation-state reclamation**
   - Terminal `StreamAdmission` operation records are reclaimed after they can no longer authorize work.
   - Expiry during admission and `cancel_all` release registration state.
   - Capacity-reuse regressions cover completion, expiry, and cancellation.
   - Completed `SingleStream` authority is retired; a later reuse attempt returns `OperationNotFound`.

3. **Candidate task ownership**
   - Task admission and task registration are serialized by the runtime registry lock.
   - Spawn attempts after shutdown admission closes return false and drop the future rather than detaching it.
   - Shutdown joins all owned task handles.
   - Deterministic regression coverage verifies post-close behavior.

4. **Transport metadata privacy**
   - `ConnectionMetadata` has a redacted `Debug` implementation for local/remote endpoint descriptions.
   - Regression coverage proves endpoint strings do not appear in ordinary debug output.

5. **Supply-chain / CI hardening**
   - Linux CI is pinned to `ubuntu-24.04`.
   - `actions/checkout` is pinned to `3d3c42e5aac5ba805825da76410c181273ba90b1`.
   - Rust remains pinned to `1.98.1`.
   - `cargo-audit 0.22.2` is a required CI step.
   - Fuzzing uses dated `nightly-2026-09-12`, pinned `cargo-fuzz 0.13.2`, and a committed `fuzz/Cargo.lock` verified with `--locked`.

6. **Verification and handoff**
   - Exact code head `26b9ecd8abf8f7386e440bc4862a6e259d1654fe` passed Rust CI run `34724799407`: lockfile, audit, rustfmt, workspace check, Clippy `-D warnings`, and full workspace tests.
   - Fuzz Smoke run `34724799451` passed lockfile verification, formatting, and all five bounded fuzz targets.
   - Focused reconciliation run `34724713810` passed the previously stale single-stream simulator scenario.
   - The detailed assessment is `security/M9-FOUNDATION-HARDENING-ASSESSMENT.md`.

## Dependency Audit Decision

`cargo audit` reported no vulnerability failure. It emitted one allowed maintenance warning for `paste 1.0.15` (`RUSTSEC-2024-0436`, unmaintained). Dependency tracing showed it is transitive through the isolated Iroh candidate networking stack rather than a direct Cross-Lab dependency. The warning is not suppressed; it must be re-evaluated with Iroh candidate updates and before ADR-0009 promotes a remote-networking architecture.

## External Repository Setting

`main` branch protection is a repository-admin setting, not a code change. The connected GitHub integration currently exposes branch-protection reads but no write action. `main` is currently unprotected. Branch protection should disable force-push/deletion and require the Rust CI check when that administration capability is available.

## Gate Result

The technical hardening gate is complete and green. The separate manual-review hold already recorded for M9 Tasks 1–3 remains in force; Task 4 must not begin until that review is explicitly approved.
