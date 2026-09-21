# Current Development State

This file is the durable resume guide for active Cross-Lab development. Git/code/tests are factual state; this file records the intended handoff.

## Current Phase

**M10 Linux + Android platform work is integrated except for real-device evidence and completion of the normal product pairing lifecycle. Phase 2 Linux + Android MVP work follows immediately; Phase 3 adaptive networking is explicitly out of scope until Phase 2 is complete.**

M1-M9 are complete. M10 host/runtime/UI foundations are integrated on `main`; physical Linux + Android lifecycle/security/resource evidence remains pending and must not be inferred from CI.

## Canonical Baseline

- Current `main` after PR #51: `cf2add350ffa60056d74ac57b0a187a0297d8777`.
- PR #49 — Linux Add Device QR invitation UI + Android CameraX/ML Kit scanner: merged as `445bbf32178dff94f339fc1ae80447967f5da215`; exact-head CI `35533414673` green on `7d3176ffd4387d3e29982ae5fe641249d6d33445`.
- PR #50 — shared product pairing coordinator + durable reciprocal trust persistence: merged as `3af825e6f78bef4512168f587f452c1b0267b6a7`; exact-head CI `35567548228` and Fuzz Smoke `35567548208` green on `ec59f99f7c09898aa2533d8b40bd4e980fa2b022`.
- PR #51 — ADR-0016 LAN discovery profile + versioned product-pairing wire + provisional Quinn pairing channel: merged as `cf2add350ffa60056d74ac57b0a187a0297d8777`; exact-head CI `35593397861` and Fuzz Smoke `35593397844` green on `2702cc0c926cf044c65aef0376f573aaedae8c6f`.
- Master Architecture revision 2.7 and ADR-0016 govern local product pairing.
- Physical-evidence continuation branch: `m10-task10-real-device-evidence`.
- M10 evidence protocol: `docs/research/M10-platform-evidence.md`.
- Active product pairing plan: `docs/superpowers/plans/2026-09-20-product-add-device-pairing.md`.

## Implemented and Merged

- Phase 0 architecture/security specifications.
- M1-M9 foundation, including deterministic simulator, failure/revocation lifecycle, Quinn transport, and remote-networking ADR.
- Linux/Android production signing-provider and identity-store boundaries.
- Versioned identity-store envelope/currentness validation.
- Product owner/local-device identity snapshots and schema-v2 bounded trusted-peer persistence.
- Five-minute single-use `crosslab:pair:v1:` invitation generation.
- Native GPUI QR presentation with cancel/regenerate.
- Android CameraX scanner with bundled/offline ML Kit QR recognition and explicit camera permission handling.
- Opaque/redacted Rust mobile bootstrap state; secret-bearing QR material is not retained in Compose state.
- Shared ADR-0003 product pairing coordinator.
- Reciprocal peer credential/trust establishment and atomic Linux/Android persistence.
- Replay, forged-proof, persistence, restart, stale-writer, and rollback/currentness regression coverage.
- ADR-0016 bounded DNS-SD discovery profile.
- Versioned product-pairing wire messages for authority/credential bundles, reciprocal trust, persistence/final acknowledgements, and cancellation.
- Provisional Quinn product-pairing channel with ephemeral invitation TLS material, TLS 1.3 only, no 0-RTT, one connection/one bidirectional stream, bounded frames/timeouts, and graceful final acknowledgement delivery.

## Active / Pending M10 Product Pairing Work

- Linux bounded DNS-SD advertisement lifecycle for a live invitation.
- Android bounded `NsdManager` discovery after QR validation.
- Carry the existing shared coordinator over the provisional Quinn channel.
- Connect Linux Add Device and Android scanner UI lifecycles to real pairing state.
- Product-visible success only after local persistence acknowledgement.
- Cancellation, expiry, timeout, discovery failure, connection failure, malformed/out-of-sequence message, and reconnect/failure presentation states.
- Real Linux + Android camera/LAN evidence on owner hardware.

## Phase 2 Linux + Android MVP — Planned Next

After normal pairing is complete, implement the Phase 2 product capabilities in small reviewed/verified slices:

- automatic trusted-device connection and reconnect;
- device status/presence;
- per-device permissions and capability controls;
- clipboard;
- resumable file transfer;
- notifications;
- privacy-conscious audit/history;
- revocation/device removal;
- complete professional Linux GPUI and Android Compose control-center UX for those features.

Phase 3 adaptive networking does not begin until the Phase 2 MVP is complete.

## Security Invariants

- QR possession is temporary bootstrap authentication, not durable trust.
- ADR-0003 transcript confirmation, owner-authorized credential validation, and device proof-of-possession remain mandatory.
- Discovery addresses, DNS-SD metadata, TLS certificates, and QUIC connection identity are routing/channel metadata only.
- Successful pairing is product-visible only after required credential/trust state is committed through ADR-0015.
- Currentness/persistence failure fails closed.
- Secrets, private keys, signer material, credentials, and sensitive payload contents are never logged.

## Exact Next Task

1. implement Linux DNS-SD advertisement and Android bounded `NsdManager` discovery against ADR-0016;
2. wire the shared product pairing coordinator over the Quinn pairing channel;
3. connect both platform UI lifecycles and persistence acknowledgements;
4. complete automated failure/cancellation/timeout/reconnect tests;
5. checkpoint M10 implementation while keeping physical-device evidence explicitly pending;
6. continue directly into Phase 2 Linux + Android MVP slices, stopping before Phase 3.

## Resume Procedure

1. verify `main`, active feature/evidence branches, open PRs, exact-head CI, and recent commits;
2. read Master Architecture revision 2.7, this file, active plans, and relevant ADRs/specifications;
3. inspect existing code and relevant uploaded research before adding adapters/dependencies;
4. reuse existing pairing/session/policy/currentness boundaries rather than duplicating security semantics;
5. work in small verifiable milestones, run full relevant gates, commit/push, and update this file;
6. keep physical-device evidence separate from CI claims.
