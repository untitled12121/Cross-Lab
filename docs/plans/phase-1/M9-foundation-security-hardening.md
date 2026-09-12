# M9 Foundation Security Hardening Plan

**Status:** In progress — owner-requested gate before M9 Task 4  
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

## Tasks

1. **Authorization provenance**
   - Make `SimNode` network classification explicit at construction instead of hard-coding `Local`.
   - Require current local peer trust when accepting inbound control traffic.
   - Validate the supplied trust record belongs to the authenticated peer.
   - Treat local revocation as fatal for ordinary inbound session traffic before policy/capability dispatch.
   - Add regression tests for `Remote + LocalOnly`, revoked trust before explicit teardown, and wrong trust-record provenance.

2. **Bounded operation-state reclamation**
   - Reclaim terminal `StreamAdmission` operation records after consumed/expired/revoked/cancelled authority can no longer authorize work.
   - Ensure `cancel_all` releases registered operation state.
   - Add capacity-reuse regression tests.

3. **Candidate task ownership**
   - Replace debug-only post-close spawn protection with a runtime-enforced admission result.
   - Ensure a task offered after shutdown admission closes is dropped rather than detached.
   - Add a deterministic unit regression test.

4. **Transport metadata privacy**
   - Replace derived `Debug` for `ConnectionMetadata` with redacted endpoint formatting while retaining non-sensitive metered state.
   - Add a regression test proving endpoint strings never appear in ordinary debug output.

5. **Supply-chain / CI hardening**
   - Pin the Linux runner major image, `actions/checkout` commit, cargo-audit version, cargo-fuzz version, and fuzz nightly date.
   - Add `cargo audit` as a required CI step and resolve any vulnerability it reports rather than suppressing findings without justification.
   - Evaluate the isolated fuzz workspace lockfile policy and record the durable choice.

6. **Verification and handoff**
   - Run focused tests, formatting, workspace check, Clippy with `-D warnings`, full workspace tests, dependency audit, and fuzz smoke.
   - Update `CURRENT.md` and a concise security assessment with fixed/deferred/external-setting findings.
   - Keep Task 4 blocked until the exact hardening head is green.

## External repository setting

`main` branch protection is a repository-admin setting, not a code change. The connected GitHub integration currently exposes branch-protection reads but no write action. Code/CI will be hardened here; branch protection must be enabled through GitHub administration when that write capability is available, with force-push/deletion disabled and the Rust CI check required.
