# Current Development State

**Phase 2 Linux + Android MVP active; Cross-Lab 0.1 not released.** Git implementation and tests are authoritative. The owner will separately perform normal Linux and arm64 Android physical acceptance; no device evidence has yet been supplied.

## Integrated software checkpoints

- **`main` merge `eb6932c72f59800e1ba04133c2ea2f11eb67992e` (PR #101):** Android verified-session status, native recovery UX and accessible action descriptions. Seven JVM regression tests, Android and CI Gate success in run `39316816232`.
- PR #99 merge `33f33610`: Android observer fault isolation and fail-closed native owner permission controls; Android and gate success run `39315312331`.
- PR #100 merge `59e2b37b`: OS notification callbacks, owner/OS consent, content redaction and lifecycle failure recovery; Android and gate success run `39314114314`.
- PR #98 merge `a8eed54a`: Android network/foreground/reconnect/teardown fault recovery; Android and gate success run `39301328776`.
- PR #97 merge `2948be3c`: Android exact-session clipboard and file picker actions, Android and gate success run `39299050701`.
- PR #96 merge `87a6eaea`: Linux GPUI exact-session native clipboard and file picker guards, Rust and gate success run `39298337209`.
- Prior Phase 2 milestones (#87–95): product QR/LAN pairing and trust, native owner permissions, notifications, resumable single-file transfer, protected audit, signed terminal revocation. CI alone does not validate real hardware.

## Active milestone — Linux GPUI verified-session and safe recovery UX

- **Branch:** `feat/phase2-linux-verified-session-status-ux`. Source checkpoint before this handoff: `3edc11e08baf60612be5b4dce4ad589851dd3abd`; use live PR head after this integrated commit.
- **Verified presentation:** Desktop `VerifiedSessionDisplay` distinguishes authenticated live peer authority from discovered or merely Online advertised state, terminal revocation, reconnecting, paused, failed and absent peers. Badges never label unverified presence as authenticated, and capability-less offline messaging avoids implying a live session.
- **Actions:** `PresenceControlAction` keeps Disconnect for an authenticated session, Reconnect for recoverable paused/failed/unverified states, and hides reconnect for revoked peers. The native session/capability boundary is unchanged.
- **Focused Rust tests:** false Online presence and incomplete authentication, owner-revoked trust and terminal controls, native recovery labels and actions.
- **Verification:** Initial source `3edc11e0` failed Source format only (Rust job was skipped). Applied the exact rustfmt changes in this batch and reconciled with PR #101's merged main. **Exact-head format, Rust and CI Gate pending**; physical verification remains separate.
- No changes to approved identity, crypto, wire, trust, or privileged-service architecture.

## Next actions

1. Verify exact-head source format, Rust and CI Gate; fix any confirmed failures as one batch and merge on green.
2. Continue Phase 2 native status, keyboard/accessibility and operational error recovery, including permission, transfer, notification and protected audit lifecycle.
3. **Owner's manual release gate remains open:** normal real-device Linux/arm64 Android QR/LAN pairing, fresh trust on restart, replay denial, peer default-deny and clipboard both directions, transfer resume/cancel/checksum/storage failure, OS notification permission withdrawal/redaction, protected audit faults, and signed terminal revocation **last** with disposable identities. Record redacted outcomes and exact tested commit/OS. Do not mark Phase 2 or 0.1 released without evidence.

See [AGENTS.md](../../AGENTS.md), [WORKFLOW.md](WORKFLOW.md), [CI.md](CI.md), accepted ADRs, and [Phase 2 completion gates](../plans/phase-2/MVP-completion-gates.md). Phase 3 is deferred.
