# Current Development State

This file is the durable resume guide for active Cross-Lab development. Git/code/tests are factual state; this file records the intended handoff.

## Current Phase

**M10 Linux + Android implementation is complete in code; physical Linux/Android camera/LAN evidence remains pending. Phase 2 Linux + Android MVP is the active implementation phase. Phase 3 adaptive networking remains explicitly out of scope until Phase 2 is complete.**

M1-M9 are complete. M10 now includes the normal QR → bounded DNS-SD → provisional Quinn → authenticated pairing → reciprocal durable trust product path on Linux + Android, with full CI verification. Owner-hardware evidence remains a separate gate and must not be inferred from CI.

## Canonical Baseline

- Current `main` after PR #57: `7e5985f0f8d8ce4546396da8aa4288fb809b508b`.
- PR #49 — Linux Add Device QR invitation UI + Android CameraX/ML Kit scanner: merged as `445bbf32178dff94f339fc1ae80447967f5da215`; exact-head CI `35533414673` green on `7d3176ffd4387d3e29982ae5fe641249d6d33445`.
- PR #50 — shared product pairing coordinator + durable reciprocal trust persistence: merged as `3af825e6f78bef4512168f587f452c1b0267b6a7`; exact-head CI `35567548228` and Fuzz Smoke `35567548208` green on `ec59f99f7c09898aa2533d8b40bd4e980fa2b022`.
- PR #51 — ADR-0016 LAN discovery profile + versioned product-pairing wire + provisional Quinn pairing channel: merged as `cf2add350ffa60056d74ac57b0a187a0297d8777`; exact-head CI `35593397861` and Fuzz Smoke `35593397844` green on `2702cc0c926cf044c65aef0376f573aaedae8c6f`.
- PR #52 — bounded DNS-SD discovery adapters: merged as `2881a8cb042207bf55a1ede9f2dc61f30ccd4eb9`; exact-head CI `35598750651` green on `2ef9ab362fb9fa59d7693662ec8de2f65e5031af`.
- PR #53 — product pairing network/platform lifecycle: merged as `9453b0e7f3e67578c79615acb1034c849a82603d`; implementation head `cc099fd7bf3335f51f54d4e09f2d002ab136e4e0` and documentation head `c5570add764ac1e223b08bdb63737abd3b182a25` both passed the full Rust + Android gate (`35641211049` / `35643137494`).
- Master Architecture revision 2.9 governs the accepted ADR-0018 text clipboard profile and ADR-0019 owner-policy-store boundary in addition to the established pairing/trusted-session architecture.
- Physical-evidence continuation branch: `m10-task10-real-device-evidence`.
- M10 evidence protocol: `docs/research/M10-platform-evidence.md`.
- Product pairing implementation plan: `docs/superpowers/plans/2026-09-20-product-add-device-pairing.md`.
- PR #55 — trusted-session platform presence lifecycle: merged as `6abb3984a36be276aa439e9e1dabb42ffaa4a8b3`; exact-head `91a96966a55c52b6b72c7e5f9b8dc686f8cf0211` passed Rust + Android CI `35859170053`.
- PR #56 — per-device permission/capability-control foundation: merged as `d707b330b5d1008e8020267faa3755ad84920591`; exact head `6565efcdaa7ffc533bbeba6d1b16b5ab8c2d7196` passed full CI `35942887247` and Fuzz Smoke `35942887292`.
- PR #57 — clipboard event-driven control preparation: merged as `7e5985f0f8d8ce4546396da8aa4288fb809b508b`; exact head `f8190dfcf70dc70c94c5f15aae86d907ce4b7a15` passed full Rust + Android CI `35972369409`.
- Active Phase 2 PR: #58 on `phase2-clipboard-implementation`.
- ADR-0018 (text clipboard capability profile v1) and ADR-0019 (platform owner-policy store boundary) were owner-accepted on 2026-09-25. The active milestone is the shared owner-policy store core, followed by Linux/Android protected-currentness adapters before writable permission UI.

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

## Phase 2 Linux + Android MVP — Active

Trusted-session foundation is implemented on PR #54:

- ordinary session proofs use the production SigningProvider boundary;
- authenticated Quinn can resolve the presented peer against bounded durable trusted-peer records;
- ProductIdentityState projects persisted peer evidence into verified TrustRecord values;
- ADR-0017 defines privacy-conscious normal-session DNS-SD with ephemeral random instances and bounded candidate/retry behavior;
- production trusted-session Quinn endpoints use TLS 1.3, no 0-RTT, non-authoritative ephemeral TLS identity, ADR-0008 channel binding, and fresh SessionId per connection;
- provider-backed endpoint/reconnect tests verify fresh session authority and unknown-peer rejection;
- PR #54 merged as `69759d8887a98392519dd72151404399adb9c5dc`; implementation head `feb0fe427045f1f067d2b4b42af4cf4618308b96` passed CI `35685303240`, and documentation head `7178e9b8d4a29a16527b3ecff96018d4a7388d80` passed CI `35686509887`.

PR #55 implements the trusted-session platform lifecycle:

- Linux Avahi and Android NsdManager advertise/browse the ADR-0017 normal-session profile using ephemeral routing-only instances, exact TXT validation, self filtering, and the shared bounded candidate limit;
- the narrow Rust `crosslab-agent` presence coordinator loads durable `ProductIdentityState`, consumes the platform `SigningProvider`, and owns automatic trusted-session connect/accept/reconnect lifecycle above Quinn;
- deterministic dial-role handling and bounded 1/2/4/8/15-second reconnect backoff avoid duplicate connection races and unbounded retry churn;
- network loss, discovery failure, explicit Disconnect/Reconnect, and discovery-instance rotation are wired on Linux and Android;
- authenticated presence is surfaced in Linux GPUI and Android Compose as discovering, connecting, online, reconnecting, paused, or failed without treating discovery/TLS metadata as identity authority;
- physical Linux/Android LAN evidence remains separately pending; PR #55 merged after exact-head Rust + Android CI `35859170053` passed.

PR #56 implements the per-device permission/capability-control foundation:

- `PolicyState` exposes exact typed rule lookup/effect update/removal while preserving constraints/obligations, monotonic revisioning, idempotent no-ops, and default deny when no exact rule exists;
- runtime policy replacement rejects stale revisions and clears request/subscription state tied to the previous policy revision;
- `TrustedPresenceAgent` owns the current in-memory policy, propagates changes into the active runtime, and carries the latest policy into fresh authenticated reconnects;
- Linux GPUI and Android Compose receive presentation-safe negotiated capability IDs plus exact per-peer permission posture, showing unruled capability operations as default deny;
- the slice deliberately does not invent wildcard authority, new product operation identifiers, or a persistent policy-store format; concrete capability features register exact operations and persistence is reviewed when the first product editor requires it.

PR #57 clipboard preparation implements:

- Quinn signals inbound control readiness only after a frame enters its existing bounded queue;
- `RuntimeActor` drains inbound control without polling and publishes bounded non-status `NodeEvent` values;
- the trusted presence coordinator consumes the runtime event stream so an unobserved event queue cannot stall a session;
- actor send wrappers expose the existing capability advertisement/request/response paths without defining new wire semantics;
- ADR-0018 now fixes the first product clipboard profile: explicit text-only `clipboard.read/get` and `clipboard.write/set`, no automatic background clipboard event in v1;
- ADR-0019 now fixes a separate rollback/currentness-aware owner-policy store with load-before-session and persist-before-apply ordering.

Owner approval on 2026-09-25 unblocked implementation. PR #58 now contains the shared `crosslab-policy-store` deterministic bounded snapshot/currentness/CAS core and exact `PolicyState` snapshot restoration, plus Linux policy-state/Secret Service currentness and Android AtomicFile/Keystore currentness adapters. Both product presence paths load validated durable policy before constructing the trusted-session agent. Exact-head full CI is pending for the platform-persistence milestone.

Continue in small verified vertical slices:

- per-device permissions/capability-control foundation — implemented on PR #56;
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

1. require exact-head Rust + Android CI and Fuzz Smoke to pass for PR #58 owner-policy persistence;
2. wire owner permission edits through persist-before-apply storage commits and active-policy replacement, preserving exact revision CAS and fail-closed recovery;
3. then implement the shared text clipboard capability runtime, Linux GPUI adapter/UI, and Android ClipboardManager adapter/UI under accepted ADR-0018;
4. keep policy loaded before every trusted-session runtime and never fall back to revision 0 after committed-state corruption;
5. preserve exact default deny, bounded request state, reconnect/policy invalidation, and no plaintext clipboard logging/history;
6. then continue through resumable file transfer, notifications, audit/history, and revocation/device removal;
7. keep M10 physical evidence separately pending until owner hardware is available;
8. stop before Phase 3.

## Resume Procedure

1. verify `main`, active feature/evidence branches, open PRs, exact-head CI, and recent commits;
2. read Master Architecture revision 2.8, this file, active plans, and relevant ADRs/specifications;
3. inspect existing code and relevant uploaded research before adding adapters/dependencies;
4. reuse existing pairing/session/policy/currentness boundaries rather than duplicating security semantics;
5. work in small verifiable milestones, run full relevant gates, commit/push, and update this file;
6. keep physical-device evidence separate from CI claims.
