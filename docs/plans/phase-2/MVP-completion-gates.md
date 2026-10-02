# Phase 2 Linux + Android MVP — Completion Gates

**Status:** Active, incomplete.  
**Current code checkpoint:** PR #79 pairing/SDK recovery (exact-head Rust + Android CI passed run `36986048412`). Later work may be staged on that feature branch; see `docs/development/CURRENT.md`.

## Delivery order

1. **Product pairing real-device gate:** normal (non-development) Linux and arm64 Android builds; real QR generated/scanned; bounded DNS-SD route; one-time QUIC + reciprocal trust; restart and reconnect; reject expired/forged/replayed codes. Log only redacted outcomes. PR #79 fixes Tokio binding and SDK bootstrap; real-device evidence pending.
2. **Device inventory + routing stabilization:** show validated offline trusted peers without interpreting network sightings as identity. Prefer resolved IPv4 address while the product pairing listener is IPv4-only. CI + physical verification pending.
3. **Owner revocation/removal:** approve ADR-0022 first. Implement signed durable terminal trust, persist-before-apply, runtime invalidation, precise owner confirmation and offline/restart reconnect denial on both platforms.
4. **Notifications:** ADR-0021 accepted; Android listener and OS/owner/content consent, exact peer Allow/Deny policy, authenticated v3.0 subscribe request/response and bounded posted/removed events, ephemeral opaque IDs and Linux plain-text reader **implemented in PR #79 source, not yet CI/hardware-verified**. Require real deny, OS withdrawal, content-redaction, revocation, lifecycle, event replay, permission persistence and no-payload-log evidence. Android→Linux foreground-only first; no inferred background guarantee.
5. **Audit/history:** ADR-0023 accepted; protected Android/Linux owner read/export/clear and metadata-only model implemented. Pairing, trust, authenticated session, permission revision, transfer terminal and subscription lifecycle producers and bounded writer/status counters are now source-staged in PR #79. **New integration remains unverified** until exact-head Rust/Android CI and physical test evidence pass. Test corruption/rollback/write failure, retention, privacy, burst/backpressure, foreground/teardown and clear-ordering.
6. **Regression and polish:** Linux/Android capability controls, clipboard, resumable single-file transfer including retry/resume/denial, app lifecycle and battery/idle resources, native accessibility/keyboard states, clear UI errors and recovery.

## Release evidence

- One consolidated static/security/integration/build CI run per coherent batch, with smaller targeted local tests during implementation. Fix failures together rather than repeatedly triggering full CI.
- Two actual devices on a reachable LAN. Test pairing, trust persist/restart, session reconnect, policy default-deny, clipboard in both directions subject to Android platform limits, file transfer/resume/integrity/cancel, notification consent/revocation, audit read/delete, and lost-device revocation reconnect denial.
- Record exact commit, coarse OS/build, pass/fail evidence without IPs, raw device IDs, private keys, QR bootstrap payloads or personal file/notification contents.
- No Phase 2 complete / Cross-Lab 0.1 release claim until implementation, approvals, CI and physical evidence gates pass. Phase 3 remains out of scope.
