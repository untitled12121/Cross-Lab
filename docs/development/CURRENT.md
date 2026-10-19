# Current Development State

**Phase:** Phase 2 Linux + Android MVP, not released. Git source and tests are authoritative; real-device release acceptance is outstanding.

## Integrated software

- **`main` merge `a8eed54a9e746855eb1219eb35dcf020d9d1f0c7` (PR #98):** Android foreground/background, network transitions, reconnect, trust reload and shutdown now recover from native faults by clearing authenticated status, isolating teardown errors, and rejecting late callbacks. Android and CI Gate passed for source `cb1181f0` in GitHub Actions run `39301328776`.
- PR #97 merge `2948be3c`: Android exact peer/session clipboard and file-picker handoffs; Android and gate passed run `39299050701`.
- PR #96 merge `87a6eaea`: Linux native exact-session callbacks and OS clipboard/file selection guards; Rust and gate passed run `39298337209`.
- PRs #87–95 integrated Linux/Android capability permission gates, QR/LAN pairing and discovery, notifications with owner/OS consent, durable trust/policy, file resume/integrity/retention and protected owner audit recovery. These are software checkpoints, **not physical-device acceptance**.

## Active milestone — Android observer and owner-control failure isolation

- **Branch:** `feat/phase2-android-observer-fault-isolation` based on merged PR #98. Consolidated implementation source checkpoint: `4887b59f40f5a619122455e85f88681d41dfbc25` (this document follows that code commit; use the PR head after pushing).
- **Observer isolation:** Android runtime, clipboard, transfer, pairing, native presence and file transfer publication use bounded, independent listener delivery. A broken UI subscriber cannot interrupt other observers or a security-relevant state transition. Pairing cancellation continues even when closing discovery fails.
- **Owner-control recovery:** thrown native policy writes, peer-targeted permission changes and policy snapshot reads fail closed, discard apparent authenticated authority, and return failure to presentation. Failed teardown makes admission terminal. Session-binding provider faults return no native ticket.
- **Publication safety:** native presence commits session/security snapshot before notifying platform/UI observers; local notification-consent errors deny delivery, and a failed transfer-session adapter is detached rather than retaining stale authority.
- **Tests:** JVM regressions cover observer failure across feature controllers; additional fake-port cases cover policy/snapshot failures, terminal failed stop and session-ticket errors. **This batch has not yet passed exact-head CI or physical-device tests.** No new protocol, owner-trust rule, identity format, privilege boundary or ADR is introduced.

## Exact next work

1. Run one Android JVM/compile checkpoint for the consolidated branch, inspect the CI Gate, fix confirmed failures together, and merge after verification.
2. Continue Phase 2 Linux/Android product reliability, native status/accessibility, explicit permission and recovery UX, and practical notification, transfer and audit lifecycle resilience in coherent vertical slices.
3. **Release gate:** collect redacted real Linux + arm64 Android evidence for production QR/LAN pairing, restart/reconnect, default-deny, clipboard both ways, resumable file transfer and checksum/cancel/storage faults, notification OS consent withdrawal/content redaction, protected audit recovery, and owner-signed terminal revocation **last** on disposable identities. No 0.1 release until actual physical acceptance.

See [AGENTS.md](../../AGENTS.md), [WORKFLOW.md](WORKFLOW.md), [CI.md](CI.md), and [Phase 2 completion gates](../plans/phase-2/MVP-completion-gates.md). Phase 3 remains deferred.
