# Current Development State

**Phase 2 Linux + Android MVP is active; Cross-Lab 0.1 is not released.** Git source and automated tests determine software status. The owner will perform real Linux + arm64 Android acceptance **after the remaining software hardening and release checks** and return redacted pass/fail evidence. No real-device product acceptance has been received. Phase 3 remains deferred.

## Integrated Phase 2 product functionality

The normal (non-development-provisioned) Linux/Android path contains QR/LAN pairing and durable trust, verified authenticated QUIC sessions and reconnect, per-peer capability permissions (default deny), bidirectional text clipboard subject to platform permissions, owner-controlled resumable single-file transfers v2, Android-to-Linux foreground notification mirroring v3 with separate OS/owner/content gates, protected metadata-only audit, and terminal signed revocation. These are **merged software implementations**, not physical acceptance claims. Owner control, current authenticated session, cancellation, policy, and least-privilege boundaries remain required throughout.

## Recent integrated software and CI

| PR | Integrated outcome | Exact source reviewed / GitHub CI |
| --- | --- | --- |
| [#107](https://github.com/untitled12121/Cross-Lab/pull/107) | Linux transfer disconnect failure state and stale-callback protection | Source `015eee05e609005f6bbdd673e9e113a26e036739`, format/Rust/CI Gate success in `39356181216` |
| [#108](https://github.com/untitled12121/Cross-Lab/pull/108) | Bounded Android transfer listener/I/O/control/cleanup pools; overload and cancellation tests | `988a5babec4436853e1e0179ee80842710d47f63`, Android/CI Gate success `39357626447`; main merge `cd5e049f` |
| [#109](https://github.com/untitled12121/Cross-Lab/pull/109) | Session-bound receive callbacks, prepublication cancellation and stale-session sender protection | `0aba14d3e75b3fd525adcea24a42bf615729c797`, Android/CI Gate success `39389553636`; merge `8091f372` |
| [#110](https://github.com/untitled12121/Cross-Lab/pull/110) | Clipboard request generations, native exception recovery and controller shutdown | `ddb638877cdd2081b947c82fed675caef08a721f`, Android/CI Gate success `39390070171`; merge `c615a803` |
| [#111](https://github.com/untitled12121/Cross-Lab/pull/111) | Bounded presence/clipboard workers, bounded Android UI clipboard handoffs and overload tests | `f2032ed521b5ebe81fc581481a481aba1da481bc`, Android/CI Gate success `39390779284`; merge `4395f374` |
| [#112](https://github.com/untitled12121/Cross-Lab/pull/112) | Bounded product pairing work, independent cancellation, stale-generation checks | `66f756dd0783bbfde76691a1e3f695a458040065`, Android/CI Gate success `39391553964`; merge `96f0109f` |

Earlier Phase 2 PRs #87–106 implemented and hardened product pairing, inventory, permissions, clipboard, file transfer, notifications, audit, native UI, and lifecycle recovery; their CI is documented in prior Git history.

## In-flight software milestone

- [PR #113](https://github.com/untitled12121/Cross-Lab/pull/113): Android receiver cleanup admission during overload: reserve a bounded cleanup lane, retain failed cleanup requests, refuse new transfers until cleanup can be safely enqueued and recover availability. Branch `fix/phase2-android-transfer-cleanup-admission`, last edited head `05ec5ffe73c5e11f08d0cce4ab2c87ad8c88bcc9`; exact-head Android CI **pending** at this checkpoint. Do not treat this as merged until exact-head verification and merge succeed.
- No security/wire/identity/policy/privilege contract was changed in this hardening batch. Do not expand into adaptive Phase 3 networking.

## Exact next work / software release gate

1. Verify and integrate PR #113 after its Android JVM/build and CI Gate pass; address any confirmed failures with regression coverage.
2. Review and resolve remaining Phase 2-only blocker reports and error/lifecycle/resource regressions across pairing, authenticated reconnect, permissions, clipboard, resumable transfer, notifications, audit and revocation. Do not imply that software verification proves Android foreground/background or Linux desktop behavior on actual hardware.
3. Run an explicit **full** GitHub Actions CI profile on the final integrated `main` source (as specified in [CI.md](CI.md)): Rust formatting, workspace tests, static security/dependency checks, Linux desktop build and normal Android arm64 APK build. Prior PR path-aware Android checks do **not** replace the final full release gate. Record exact final commit and job conclusions.
4. After the software gates are complete, hand the reviewed **normal product builds** to the owner for the physical acceptance matrix in [Phase 2 product acceptance](../research/Phase2-product-acceptance.md) and the [file-transfer matrix](../research/Phase2-file-transfer-device-evidence.md). Fix any hardware-discovered failures before claiming Phase 2 complete.

No local shell build, manual full release CI, or physical Linux/Android acceptance is claimed by this handoff.

## Owner-managed physical release gate — open

Test normal, **not development-provisioned**, Linux + arm64 Android builds on the same reachable LAN: QR creation/scan and expiration/forgery/replay; reciprocal trust and restart persistence; authenticated reconnect; exact-peer default-deny permissions; clipboard both directions; resumable file transfer (empty, >3 MiB, cancel/retry/checksum, lost connection, storage/permission loss); OS notification access opt-in/withdrawal and redaction; protected audit corruption/rollback/recovery and owner clear/export; accessibility/resource behavior. Perform **owner-signed terminal revocation last**, using disposable identities only. Record only redacted pass/fail with tested commit and coarse OS/build; do not record QR payloads, network addresses, identifiers, private file paths/content, tokens or keys.

**Cross-Lab 0.1 and Phase 2 remain incomplete until the release CI and actual physical acceptance gates pass.** See [AGENTS.md](../../AGENTS.md), [WORKFLOW.md](WORKFLOW.md), [CI.md](CI.md), [Phase 2 gates](../plans/phase-2/MVP-completion-gates.md) and approved ADR-0020–0023.
