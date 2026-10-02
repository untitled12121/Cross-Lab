# Phase 2 Linux + Android File Transfer — Physical Evidence

The software path for `files.transfer` v2.0 was merged through PR #76. This is a **manual, real-device verification record**, not evidence that the feature works on physical hardware. All results below remain **pending** until someone performs and records them.

For the older M10 development-provisioning lifecycle test, use [M10-platform-evidence.md](M10-platform-evidence.md). **Do not use that synthetic provisioning mode to validate the file-transfer product path.** The Android application selects `MobileRuntimePort` instead of `ProductPresencePort` when development provisioning is loaded, so product file-transfer controls are unavailable in that mode.

## Preparation

- Record the exact tested Cross-Lab commit, desktop build, Android build, Linux distro/desktop session, Android OS/API level and coarse device model. Use **normal product pairing**, not synthetic credentials.
- Use a Linux workstation and an **arm64** physical Android device on a local network, with camera and local-network access. Follow [the product build guide](../development/BUILD.md): `cargo crosslab setup`, `cargo crosslab doctor`, `cargo crosslab build desktop`, `cargo crosslab build android`, `cargo crosslab install android`, and `cargo crosslab run desktop`. Install and launch the ordinary product build, **without** `--development` or a synthetic development-provisioning override.
- Use the product **Add Device** / QR flow to pair and establish an authenticated trusted session. Both sides must show connected/active and negotiate `files.transfer` v2.0.
- Check and explicitly configure the destination owner's exact `files.transfer / receive` rule for the peer. **Default deny is intentional**; choosing a save destination alone does not authorize a denied transfer.
- Prepare disposable test files: empty, small text, a binary file larger than 3 MiB, and two different files with an identical basename. Record hashes externally only if safe. Never use private documents.

## Real-device matrix

Record a pass/fail/blocked outcome and a redacted evidence ID for every exercised scenario. Do not copy secret payloads or full paths.

| Scenario | Required observation | Result | Evidence ID / notes |
| --- | --- | --- | --- |
| QR pairing and capability | Trusted authenticated session established; v2 capability visible only when the complete product path is available. | pending | pending |
| Explicit default deny | With `receive` denied, an offer cannot gain stream authority merely by opening a picker. No file published. | pending | pending |
| Linux → Android, small file | Owner approves inbound offer, Android selects **Choose destination** with SAF CreateDocument, verified bytes saved, terminal result Completed. | pending | pending |
| Android → Linux, small file | Android **Send file** uses SAF OpenDocument, desktop owner selects local save location, exact bytes saved, Completed shown. | pending | pending |
| Empty and larger binary | Both directions correctly handle a 0-byte file and a binary larger than 3 MiB using bounded progress; resulting bytes/digest match. | pending | pending |
| Owner declines before Ready | No destination file data or active stream authority; retry requires a fresh explicit offer. | pending | pending |
| Cancel during stream | No completed destination publication; no stale active state after cancellation. Retained partial can be reused only for the exact original file identity. | pending | pending |
| Network loss and reconnect | Active transfer stops on session loss; new authenticated session established, no old stream/OperationId reused, progress resumes from the last durable 1 MiB checkpoint. | pending | pending |
| Android app background / restart | Partial bytes survive in app-private no-backup storage, incomplete trailing bytes are not reported as durable, owner is offered **Resume saved destination** or **Reauthorize document** as appropriate. | pending | pending |
| Stale destination picker | Cancel the pending offer, then allow an older Save As result to return. It must not authorize a newer request or change the destination. | pending | pending |
| Source changed on retry | Modify source content after interruption; same TransferId identity must fail rather than send changed bytes. New selection starts a fresh transfer. | pending | pending |
| Destination collision | Existing owner file is not silently overwritten; check document-provider behavior separately on Android, because SAF provider publication/visibility semantics vary. | pending | pending |
| Cancellation near final publication | Prepublication cancel prevents Completed; once publication begins, UI must not offer a misleading cancellable operation. Record provider behavior without assuming atomic SAF publication. | pending | pending |
| Trust / policy revocation | Revocation while pending/active removes old authority; subsequent reconnect and transfers are denied until the owner explicitly reauthorizes. | pending | pending |
| Permissions and privacy | No broad Android storage permission; URI/local paths are not sent to peer or printed in ordinary logs; optional persisted URI access requires an explicit owner choice. | pending | pending |
| Stability | Repeated transfer/cancel/retry and app foreground cycles do not create duplicate active streams, runaway retries, or persistent busy UI. | pending | pending |

A physical timing window or race may be difficult to reproduce manually. Mark it **blocked or unverified**, retain the corresponding automated regression evidence separately, and never substitute a unit test for physical observation.

## Evidence rules and acceptance

For every pass/fail, record tested commit, result, evidence ID and date. Acceptable artifacts include redacted product-state screenshots, outcome notes and non-sensitive hashes of disposable fixtures. Do not record Android serials, LAN IPs, MACs, Wi-Fi SSIDs, credentials, pairing QR payloads, session IDs, root keys, signing keys, tokens, full file paths or file content.

The Android destination is an owner-selected SAF document. Its provider's creation, truncation and publication guarantees are not necessarily those of a native Linux filesystem. Do not claim atomic or non-clobbering behavior across providers without testing it.

Only after required physical scenarios pass should the corresponding real-device gate be marked complete. Keep the existing M10 lifecycle/camera/LAN evidence in `M10-platform-evidence.md` separate; Phase 3 remains out of scope.
