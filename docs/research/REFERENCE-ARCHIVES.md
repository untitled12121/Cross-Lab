# Uploaded Reference Archives — Phase 2 Retention Guide

**2026-10-19 · Advisory only.** These 24 ZIP archives were uploaded as Project Sources for research. Git code/tests and approved ADRs govern Cross-Lab; deleting a Project Source does not delete code already in Cross-Lab. This classification is based on archive names, the current roadmap and upstream project descriptions where identifiable—not a byte-by-byte audit of every archived revision. Reinspect license and actual archive version before copying any source.

## Keep while completing Phase 2 (6)

| Project source | Current value |
| --- | --- |
| `gpui-kit.zip` | Native desktop UI/components and design-system integration |
| `uniffi-rs.zip` | Shared Rust ↔ Kotlin binding/debugging patterns |
| `quinn.zip` | QUIC transport, pairing, session and connection recovery troubleshooting |
| `cosmic-ext-connect-core.zip` | Linux peer integration/lifecycle reference |
| `kdeconnect-kde.zip` | Linux/Android capability and permission UX behavior reference |
| `syncthing.zip` | Transfer recovery, safe streaming and integrity patterns |

The last three are optional once their Phase 2 behavior is stable. They are examples, not license-free code libraries.

## Good candidates to remove from ChatGPT Project Sources now (17)

| Archive(s) | Why not needed as always-loaded Phase 2 references |
| --- | --- |
| `btleplug.zip`, `nusb.zip` | Phase 3 BLE/USB transports |
| `iroh.zip`, `rust-libp2p.zip` | Deferred remote/P2P transport evaluation (ADR-0009 remains authoritative) |
| `cpal.zip` | Later realtime audio |
| `rustdesk.zip`, `scrcpy.zip`, `FreeRDP.zip`, `xcap.zip`, `enigo.zip` | Later screen capture, remote desktop and input control |
| `automerge.zip` | Later distributed state/synchronization |
| `tough.zip` | Later secure updater/recovery |
| `osquery.zip` | Later system inventory/advanced host management |
| `pcbu-desktop.zip` | Later cross-device authentication/unlock; upstream uses GPL-3.0 |
| `fungi.zip` | Later device app platform/distributed services |
| `reconya.zip` | LAN scanning/inventory is not Cross-Lab's approved passive, privacy-preserving Phase 2 DNS-SD model; public upstream is CC BY-NC 4.0 |
| `FlyingCarpet.zip` | File transfer reference largely superseded by existing Cross-Lab single-file resume/Quinn code; retain Syncthing for targeted recovery reference |

## Inspect before removal (1)

`another.zip` has no unambiguous project identity from its name. Determine its upstream project and license before deciding; do not assume what code it contains.

**Keep permanently:** `CrossLab-Master-Architecture-and-Development-Plan.md` (governing uploaded architecture). Approved repository revisions/ADRs are authoritative for implementation. Archived references can be cloned again on demand when their phase becomes active.

**Cleanup responsibility:** Project Source membership is controlled in ChatGPT Project settings, not by Git. This document records recommendations only; it does not remove any Project uploads.
