# Phase 2 — Linux + Android product acceptance evidence

**Gate:** open · **Plan:** Master Architecture v2.11 and accepted ADR-0021/22/23 · **Build type:** normal product (never development provisioning).

This is an evidence worksheet, **not a certification**. All scenarios are pending until performed on actual owner Linux and arm64 Android devices on a reachable LAN. Host CI passes prove that the code compiles and its simulated/automated cases pass, not that networking, OS permission prompts, physical reconnection or user files work.

## Prepare

On the Linux host, use the exact reviewed Phase 2 feature branch, record its short commit hash, and follow [the build guide](../development/BUILD.md). From the repository root:

```bash
git status --short
git branch --show-current
git log -1 --oneline
cargo crosslab setup
cargo crosslab doctor
cargo crosslab build desktop
cargo crosslab build android
cargo crosslab install android
cargo crosslab run desktop
```

Run **normal** builds; never pass `--dev`, `--development`, or synthetic-provisioning Gradle overrides. Connect both devices to the same LAN without AP/client isolation. Use disposable non-private files and non-private test notifications. Keep the Linux desktop and Android foreground while verifying the first connection. Record a coarse Linux distribution/session, Android OS/API level and tested source commit (not raw device serials).

The final signed revocation is **permanent for that device identity**. Exercise it **last**, only with disposable test identity material; revocation is not undone by clearing audit history.

## Evidence record

| Field | Recorded result |
| --- | --- |
| Linux and Android tested commit | pending |
| Date / reviewer | pending |
| Linux distro and session (coarse) | pending |
| Android OS API level / arm64 confirmed | pending |
| LAN reachability without client isolation | pending |
| Overall release gate | **pending** |

Each scenario must have a redacted observation/evidence ID and **passed / failed / blocked**, not inferred from a green CI run.

| Scenario | Required owner observation | Actual | Redacted evidence ID |
| --- | --- | --- | --- |
| Linux **Devices → Add Device** | Normal-mode QR appears without QUIC bind failure | pending | pending |
| Android camera and QR | Real scanner recognizes invitation; expired/replayed QR cannot enroll | pending | pending |
| One-time pairing | Secure transcript completes; reciprocal durable trust shown on both apps | pending | pending |
| Restart and discover | Restart both apps, persist offline inventory and reconnect with new active authenticated session | pending | pending |
| Loss and recovery | Disable/re-enable Wi-Fi and foreground/background Android; no stale authority/retry storm | pending | pending |
| Default deny | Clipboard/file/notifications do not operate merely because capability is advertised | pending | pending |
| Explicit permission | Exact connected peer Allow works; Ask/Deny remain closed; permission survives normal restart | pending | pending |
| Clipboard both directions | Disposable non-private text only, Android foreground/OS restrictions respected | pending | pending |
| File both directions | Small/empty and binary above 3 MiB with equal final checksum | pending | pending |
| File resume and cancel | Interrupt/reconnect, bounded progress, integrity and owner destination preserved; no stale stream ID | pending | pending |
| Notification Access | Disabled by default; owner separately grants OS access, mirroring, peer permission and optional content | pending | pending |
| Notification event/redaction | Posted/removed delivered to Linux only after approval; protected/private/content-off redacted | pending | pending |
| Notification revocation | Deny, OS access withdrawal, owner opt-out and background close pending source and erase transient reader state | pending | pending |
| Notification timeout | Unanswered/denied subscription cannot leave Linux permanently awaiting approval or disclose private data | pending | pending |
| Audit lifecycle | Owner history shows only typed pairing/session/permission/transfer/subscription outcomes without contents or IDs | pending | pending |
| Audit clear/export | CSV has only hour/action/outcome/revision; clear and restart preserve empty history; signed trust unchanged | pending | pending |
| Audit store failure | With a **disposable test identity**, protected store unavailable/corrupt/rollback fails closed and shows safe status | pending | pending |
| **Terminal revocation (last)** | Confirm full-ID peer, persist signed tombstone; offline/restart reconnect and new enrollment under same identity denied | pending | pending |
| Accessibility and resources | Keyboard/focus, OS permission guidance, app foreground cycles and idle resource use are acceptable | pending | pending |

For detailed file-transfer edge cases, use [the transfer evidence matrix](Phase2-file-transfer-device-evidence.md). For synthetic-only M10 lifecycle tests see [M10 evidence](M10-platform-evidence.md); those **cannot** substitute for this real product gate.

## Privacy rules

Never attach raw QR code/text, private keys, credentials, device IDs or serials, IP/MAC/SSID, real file names/paths/contents, clipboard contents, notification title/body/app label, database/keyring payloads or unredacted logs. Use screenshots with identifiers concealed, observed status labels, coarse OS/API details and checksums of **disposable random fixture data**. If blocked, record a harmless state/error label and what action preceded it. Do not overwrite production identity/keyring data to simulate corruption.

The Phase 2 gate can be closed only when the owner has supplied physical evidence, software CI passes at the **actual reviewed commit**, and remaining failures are resolved. Leave PR #79 unmerged until then.
