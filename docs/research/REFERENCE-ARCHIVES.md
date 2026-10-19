# Uploaded Reference Archives — Retention Audit

**2026-10-19 — Corrected retention policy.** The owner wants to free Project Source slots **only for repositories whose intended Cross-Lab functionality is completed and whose reference code is no longer needed for remaining engineering or acceptance**. A repository being relevant to a later phase is **not** a reason to remove it. Previous advice recommending immediate removal of 17 sources was incorrect and is withdrawn.

## Removal decision

**Confirmed safe to remove today: none (0/24 archives).** The current roadmap reports Phase 2 Linux + Android MVP as active, with unresolved hardware acceptance and reliability/UX work. No completed-feature reference has been verified to be entirely unnecessary for outstanding work. A milestone marked “implemented” does not by itself establish that its upstream reference is disposable.

Treat every uploaded archive as retained by default until a feature-specific code/reuse audit shows (1) its intended behavior is complete, (2) relevant tests and required platform acceptance are complete, (3) no ongoing fix or planned future feature still needs the reference, and (4) the upstream identity/license is understood. Verify these facts before suggesting a deletion; do not delete Project Sources automatically.

## Keep — active Phase 2 and reusable foundations (7)

| Archive | Why keep it |
| --- | --- |
| `gpui-kit.zip` | GPUI Kit is part of the active desktop UI architecture and ongoing UX improvements |
| `uniffi-rs.zip` | Rust ↔ Kotlin bindings are used during Android integration and may be needed for debugging |
| `quinn.zip` | Quinn transport milestone was implemented, but normal product pairing/reconnection still needs refinement and physical evidence |
| `cosmic-ext-connect-core.zip` | Reference for Linux device integration, discovery and lifecycle behavior |
| `kdeconnect-kde.zip` | Reference for cross-device permissions, notifications, clipboard and other Phase 2 capabilities |
| `syncthing.zip` | Streaming, resumable transfer and integrity/recovery ideas while Phase 2 file transfer is being accepted |
| `FlyingCarpet.zip` | File-transfer reference: initial implementation exists, but Phase 2 integrity/resume/cancel acceptance remains unfinished |

Some code in these areas is already integrated. **That does not establish that the references have no remaining value.**

## Keep — features not yet completed or later phases (16)

| Archives | Planned or still-evaluable area |
| --- | --- |
| `btleplug.zip`, `nusb.zip` | BLE and USB/device transport |
| `iroh.zip`, `rust-libp2p.zip` | Future P2P, Internet and route evaluation |
| `cpal.zip` | Realtime audio |
| `rustdesk.zip`, `scrcpy.zip`, `FreeRDP.zip`, `xcap.zip`, `enigo.zip` | Remote screen, display capture and input control |
| `automerge.zip` | Distributed state and synchronization |
| `tough.zip` | Secure updates/recovery |
| `osquery.zip` | Host inventory/management |
| `pcbu-desktop.zip` | Cross-device authentication/unlock |
| `fungi.zip` | Later distributed/device application ideas |
| `reconya.zip` | Discovery/inventory research; not an approved replacement for the Phase 2 DNS-SD model. Retain pending a deliberate relevance/license decision, not because active scanning is approved |

Retaining a reference does **not** authorize using its architecture or code. Approved Cross-Lab ADRs, least-privilege privacy boundaries, upstream licenses, maintenance and platform fit still govern reuse.

## Keep — unidentified (1)

`another.zip` does not identify its contents by filename. Inspect its archive metadata and license before any removal decision.

## Permanent architectural reference (not one of the 24 ZIPs)

Keep `CrossLab-Master-Architecture-and-Development-Plan.md`, with accepted repository revisions and ADRs governing implementation changes.

## When space is needed

First audit which **completed** feature references are genuinely redundant. Only then propose precise deletions, with the relevant completed milestone and evidence that the reference is no longer needed. If no such source is confirmed, leave all Project Sources in place and ask the owner before replacing any. Git documentation cannot remove ChatGPT Project Sources; the owner controls their membership.
