# Phase 2 Linux + Android MVP — Completion Gates

**Status:** Active, incomplete.  
**Software checkpoint:** PR [#79](https://github.com/untitled12121/Cross-Lab/pull/79) merged into `main` as `b6458c5` on 2026-10-19. Reviewed source `8fd25ef7` passed the required Rust/Android/fuzz steps; see [CURRENT.md](../../development/CURRENT.md) for the exact run's final-status qualification. **Phase 2 release gate remains open for real-device evidence and final UX/reliability work.**

## Delivery order

1. **Product pairing real-device gate:** normal (non-development) Linux and arm64 Android builds; real QR generated/scanned; bounded DNS-SD route; one-time QUIC + reciprocal trust; restart and reconnect; reject expired/forged/replayed codes. Log only redacted outcomes. PR #79 fixes Tokio binding and SDK bootstrap; real-device evidence pending.
2. **Device inventory + routing stabilization:** show validated offline trusted peers without interpreting network sightings as identity. Prefer resolved IPv4 address while the product pairing listener is IPv4-only. CI + physical verification pending.
3. **Owner revocation/removal:** ADR-0022 accepted; owner-confirmed signed durable terminal trust, persist-before-apply, runtime invalidation and retained tombstones are in software. Offline/restart reconnect denial on both physical platforms still needs evidence; owner-only presentation forgetting is not equivalent to revocation.
4. **Notifications:** ADR-0021 accepted; Android listener and OS/owner/content consent, exact peer Allow/Deny policy, authenticated v3.0 subscribe request/response and bounded posted/removed events, ephemeral opaque IDs and Linux plain-text reader **integrated and software-verified at reviewed PR #79 source**. Require real deny, OS withdrawal, content-redaction, revocation, lifecycle, event replay, permission persistence and no-payload-log evidence. Android→Linux foreground-only first; no inferred background guarantee.
5. **Audit/history:** ADR-0023 accepted; protected Android/Linux owner read/export/clear and metadata-only model integrated. Pairing, trust, authenticated session, permission revision, transfer terminal and subscription lifecycle producers, bounded writer/status counters, and callback failure handling have software coverage. **Physical acceptance remains unverified**, including protected storage faults and lifecycle/recovery. Test corruption/rollback/write failure, retention, privacy, burst/backpressure, foreground/teardown and clear-ordering.
6. **Regression and polish:** Linux/Android capability controls, clipboard, resumable single-file transfer including retry/resume/denial, app lifecycle and battery/idle resources, native accessibility/keyboard states, clear UI errors and recovery.

## Release evidence

- One consolidated static/security/integration/build CI run per coherent batch, with smaller targeted local tests during implementation. Fix failures together rather than repeatedly triggering full CI.
- Two actual devices on a reachable LAN. Test pairing, trust persist/restart, session reconnect, policy default-deny, clipboard in both directions subject to Android platform limits, file transfer/resume/integrity/cancel, notification consent/revocation, audit read/delete, and lost-device revocation reconnect denial.
- Record exact commit, coarse OS/build, pass/fail evidence without IPs, raw device IDs, private keys, QR bootstrap payloads or personal file/notification contents.
- No Phase 2 complete / Cross-Lab 0.1 release claim until implementation, approvals, CI and physical evidence gates pass. Phase 3 remains out of scope.
