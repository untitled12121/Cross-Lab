# Current Development State

This file is the durable resume guide for active Cross-Lab development. Git/code/tests are factual state; this file records the intended handoff.

## Current Phase

**M10 Linux + Android implementation is complete in code; physical Linux/Android camera/LAN evidence remains pending. Phase 2 Linux + Android MVP is the active implementation phase. Phase 3 adaptive networking remains explicitly out of scope until Phase 2 is complete.**

M1-M9 are complete. M10 now includes the normal QR → bounded DNS-SD → provisional Quinn → authenticated pairing → reciprocal durable trust product path on Linux + Android, with full CI verification. Owner-hardware evidence remains a separate gate and must not be inferred from CI.

## Canonical Baseline

- Current `main` before PR #53 merge: `711b7c0fadfa53be5a17b5b8fd8ed10c808b14a8`.
- PR #49 — Linux Add Device QR invitation UI + Android CameraX/ML Kit scanner: merged as `445bbf32178dff94f339fc1ae80447967f5da215`; exact-head CI `35533414673` green on `7d3176ffd4387d3e29982ae5fe641249d6d33445`.
- PR #50 — shared product pairing coordinator + durable reciprocal trust persistence: merged as `3af825e6f78bef4512168f587f452c1b0267b6a7`; exact-head CI `35567548228` and Fuzz Smoke `35567548208` green on `ec59f99f7c09898aa2533d8b40bd4e980fa2b022`.
- PR #51 — ADR-0016 LAN discovery profile + versioned product-pairing wire + provisional Quinn pairing channel: merged as `cf2add350ffa60056d74ac57b0a187a0297d8777`; exact-head CI `35593397861` and Fuzz Smoke `35593397844` green on `2702cc0c926cf044c65aef0376f573aaedae8c6f`.
- PR #52 — bounded DNS-SD discovery adapters: merged as `2881a8cb042207bf55a1ede9f2dc61f30ccd4eb9`; exact-head CI `35598750651` green on `2ef9ab362fb9fa59d7693662ec8de2f65e5031af`.
- PR #53 — product pairing network/platform lifecycle: exact implementation head `cc099fd7bf3335f51f54d4e09f2d002ab136e4e0`; CI `35641211049` green after the final lifecycle/security review. Documentation checkpoint follows before merge.
- Master Architecture revision 2.7 and ADR-0016 govern local product pairing.
- Physical-evidence continuation branch: `m10-task10-real-device-evidence`.
- M10 evidence protocol: `docs/research/M10-platform-evidence.md`.
- Product pairing implementation plan: `docs/superpowers/plans/2026-09-20-product-add-device-pairing.md`.

## Implemented M10 Product Path

- Linux/Android production signing-provider and identity-store boundaries.
- Versioned identity-store envelope/currentness validation.
- Product owner/local-device identity snapshots and schema-v2 bounded trusted-peer persistence.
- Five-minute single-use `crosslab:pair:v1:` invitation generation.
- Native GPUI QR presentation with cancel/regenerate and terminal lifecycle states.
- Android CameraX scanner with bundled/offline ML Kit QR recognition and explicit camera permission handling.
- Opaque/redacted Rust mobile bootstrap state; secret-bearing QR material is not retained in Compose state.
- Shared ADR-0003 product pairing coordinator.
- Reciprocal peer credential/trust establishment and atomic Linux/Android persistence.
- Replay, forged-proof, cancellation, persistence failure, restart, stale-writer, rollback/currentness, message-order, and final-ack regression coverage.
- ADR-0016 bounded DNS-SD discovery profile.
- Linux Avahi advertisement and Android bounded `NsdManager` resolution with exact PairingId/TXT filtering.
- Versioned product-pairing wire messages for authority/credential bundles, reciprocal trust, persistence/final acknowledgements, and cancellation.
- Provisional Quinn product-pairing channel with ephemeral invitation TLS material, TLS 1.3 only, no 0-RTT, one connection/one bidirectional stream, bounded frames/timeouts, and graceful final acknowledgement delivery.
- Real product lifecycle wiring: Linux invitation/listener/advertisement → Android scan/discovery/client → ADR-0003 verification → local persistence barriers → reciprocal trust → final completion acknowledgement.
- Linux GPUI and Android Compose pairing states for waiting/finding, connecting, verifying, saving trust, finalizing, paired, expiry, cancellation, and failure.
- Linux Avahi service-type normalization regression coverage so the full architecture service name is mapped correctly to Avahi's type/domain API.

## M10 Pending Evidence Only

- Real Linux + Android QR camera scan.
- Real LAN DNS-SD resolution across owner hardware/network.
- Real provisional Quinn pairing exchange between physical devices.
- Resource/lifecycle evidence required by `docs/research/M10-platform-evidence.md`.

These are owner-hardware evidence tasks, not missing implementation tasks.

## Phase 2 Linux + Android MVP — Active Next

Implement in small verified vertical slices:

- automatic trusted-device connection and reconnect;
- device status/presence;
- per-device permissions and capability controls;
- clipboard;
- resumable file transfer;
- notifications;
- privacy-conscious audit/history;
- revocation/device removal;
- complete professional Linux GPUI and Android Compose control-center UX for those features.

Phase 3 adaptive networking does not begin until Phase 2 is complete.

## Security Invariants

- QR possession is temporary bootstrap authentication, not durable trust.
- ADR-0003 transcript confirmation, owner-authorized credential validation, and device proof-of-possession remain mandatory.
- Discovery addresses, DNS-SD metadata, TLS certificates, and QUIC connection identity are routing/channel metadata only.
- Successful pairing is product-visible only after required credential/trust state is committed through ADR-0015.
- Currentness/persistence failure fails closed.
- Secrets, private keys, signer material, credentials, and sensitive payload contents are never logged.

## Exact Next Task

1. merge PR #53 after the documentation checkpoint exact-head gate;
2. start Phase 2 with automatic trusted-device connection/reconnect and device presence/status as one coherent vertical slice;
3. expose that state cleanly in Linux GPUI and Android Compose;
4. then continue through permissions, clipboard, resumable file transfer, notifications, audit/history, and revocation/device removal;
5. keep M10 physical evidence separately pending until owner hardware is available;
6. stop before Phase 3.

## Resume Procedure

1. verify `main`, active feature/evidence branches, open PRs, exact-head CI, and recent commits;
2. read Master Architecture revision 2.7, this file, active plans, and relevant ADRs/specifications;
3. inspect existing code and relevant uploaded research before adding adapters/dependencies;
4. reuse existing pairing/session/policy/currentness boundaries rather than duplicating security semantics;
5. work in small verifiable milestones, run full relevant gates, commit/push, and update this file;
6. keep physical-device evidence separate from CI claims.
