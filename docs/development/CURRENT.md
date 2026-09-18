# Current Development State

This file is the durable resume guide for active Cross-Lab development. Git/code/tests are factual state; this file records the intended handoff.

## Current Phase

**M10 first real Linux + Android platform slice is in implementation.**

M1–M9 are complete. M9 selected Quinn for local/LAN and Iroh for remote/NAT/relay through accepted ADR-0009. M10 design and ADR-0012 were owner-approved on 2026-09-17.

## Canonical Baseline

- Canonical `main` before Task 5: `7ba8bef4d9f22b904b493a6b085c36aa3cd7db00` (`feat(quic): productize authenticated endpoint bootstrap`).
- M10 Task 4 PR: #33 (`feat(quic): productize authenticated endpoint bootstrap`).
- Post-merge Task 4 `main` CI: GitHub Actions `35257937886` — passed dependency audit, `cargo fmt --check`, workspace check, clippy with warnings denied, and full workspace tests.
- Active implementation plan: `docs/superpowers/plans/2026-09-17-m10-first-platform-slice.md`.
- Approved design: `docs/superpowers/specs/2026-09-17-m10-platform-shell-design.md`.
- Master Architecture revision 2.3 and accepted ADR-0012 remain the M10 design baseline.

## Completed M10 Runtime / Quinn Checkpoints

Tasks 3 and 4 are integrated and verified:

- `crates/runtime` owns reusable platform-neutral session/dispatcher/transport coordination instead of duplicating it in `apps/sim`;
- runtime snapshots expose typed presentation-safe summaries while hiding inactive `SessionId`, credentials, keys, channel-binding bytes, and payload material;
- transport loss, peer revocation, owner-root replacement, and Device Signing authority replacement fail closed;
- capability-event subscriptions remain bounded;
- `apps/sim` consumes the shared runtime;
- Quinn client/server endpoint wrappers require explicit TLS certificate/trust material;
- authenticated bootstrap derives ADR-0008 `quic-tls-exporter-v1` only after the full QUIC/TLS handshake;
- no accept-all verifier, implicit development certificate fallback, TLS-certificate-to-`DeviceId` authority mapping, or 0-RTT authority exists;
- `AuthenticatedQuicSession` is only produced after Cross-Lab authentication is active;
- negative coverage includes wrong exporter binding, replay, untrusted/revoked peers, malformed/oversized bootstrap input, timeout/cancellation, and fresh reconnect binding/`SessionId` rotation.

## Task 5 Verification Checkpoint

**M10 Task 5 — application runtime actor for lifecycle-safe consumers — is implementation-complete and awaiting final documentation-head CI plus merge.**

- Task 5 branch: `m10-task5-runtime-actor`.
- Task 5 PR: #34 (`feat(runtime): add lifecycle-safe runtime actor`).
- Verified implementation head: `00974547e4a43e44c3bdbe534f62c163a79da7c1`.
- Exact-head implementation CI: GitHub Actions `35295857847` — passed dependency audit, `cargo fmt --check`, workspace check, clippy with warnings denied, and full workspace tests.
- The final documentation-only checkpoint must also pass exact-head CI before merge; use the actual PR head from GitHub rather than embedding a self-referential commit SHA here.

Task 5 provides:

- one actor-owned mutable runtime/session lifecycle with no process-wide singleton or global mutable state;
- bounded Tokio `mpsc` commands and latest-value `watch` status snapshots so slow consumers cannot grow unbounded queues;
- explicit duplicate-start rejection;
- deterministic explicit stop plus fail-closed drop/cancellation semantics;
- network-loss handling that clears active session authority and closes transport state;
- reconnect that requires a distinct freshly authenticated session;
- owned transport support for actor/task lifetime while preserving `RuntimeNode` fail-closed cleanup;
- tests for single-start ownership, duplicate start, stop, network loss, fresh reconnect, bounded command delivery, slow subscribers, and dropped-handle shutdown.

## First M10 Slice

The first slice remains **authenticated local device connection + device status** between Linux desktop and Android:

- Linux desktop: Rust + GPUI + GPUI Kit with feature-first `page.rs` / `layout.rs` / `_components` / `components/ui` / `features` organization.
- Android: Kotlin + Jetpack Compose over one narrow shared-Rust mobile façade.
- Shared coordination: `crosslab-runtime`.
- Mobile FFI: one narrow UniFFI façade; internal crates are not exported independently.
- Transport: Quinn local/LAN using existing Cross-Lab channel-bound authentication/session semantics.
- Scope: authenticated connection, safe device/trust/connectivity/session status, clean disconnect/fresh reconnect, revocation, and bounded lifecycle ownership.
- Pairing/bootstrap UI, clipboard, file transfer, BLE/Wi-Fi Direct, iOS, Windows, and production Iroh promotion remain outside this first slice.

## Security / Lifecycle Constraints

- UI code does not own networking, cryptography, persistence internals, or privileged operations.
- Status snapshots/FFI DTOs never expose private keys, credentials, authentication secrets, channel-binding bytes, or sensitive payloads.
- ADR-0008 leaves production Quinn certificate provisioning/pinning lifecycle undecided; Task 4 productizes explicit transport-local provisioning without silently resolving that product decision.
- Persistent Android production identity material still requires a reviewed secure-storage adapter such as Android Keystore/StrongBox.
- Do not promote Iroh in M10; preserve ADR-0009 and `Libp2p trigger: no`.
- Do not invent the missing canonical Darkmatter palette.

## Deferred / External Risks

- Durable rollback-resistant persistence for `OwnerAuthorityState` and accepted authority epochs.
- Real Android/iOS lifecycle/background-networking/secure-keystore evidence beyond the current slice.
- Windows/macOS platform networking and firewall evidence.
- Production Quinn certificate issuance/pinning lifecycle.
- Production persistence/rotation/privacy policy for stable Iroh transport keys.
- `paste 1.0.15` / `RUSTSEC-2024-0436` in the isolated Iroh dependency graph.
- `main` branch protection and other repository-administration controls.

## Exact Next Task

Finish Task 5 integration, then execute **M10 Task 6 — Linux desktop shell and local design adapter**:

1. require exact-head CI for the actual final Task 5 PR head, mark PR #34 ready, merge with the verified expected head, and verify post-merge `main` CI;
2. create a fresh Task 6 branch from that verified `main`;
3. re-verify GPUI Kit release/license/source before editing Cargo; reviewed baseline is `gpui-kit 0.6.1` (Apache-2.0) with its pinned GPUI pre-release family;
4. add parser/mapper tests first for the canonical theme contract, invalid/missing-field fixtures, injected `System` appearance resolution, and explicit `ThemeUnavailable` for missing Darkmatter;
5. add device presentation tests that map `crosslab-runtime` snapshots into safe UI state without secret/session-binding bytes;
6. implement the smallest Linux GPUI + GPUI Kit shell using the approved feature-first desktop structure, with networking/session ownership remaining outside UI components;
7. keep radius 0, canonical semantic theme values, keyboard/focus accessibility, compact spacing, and restrained status colors; do not invent Darkmatter values or a speculative component library;
8. run desktop unit/build gates plus full workspace fmt/check/clippy/tests, checkpoint this file, and merge only the verified head.

## Resume Procedure

1. verify canonical `main`, active feature branch, PR state, exact-head CI, and recent commits before editing;
2. read Master Architecture revision 2.3, ADR-0008, ADR-0009, ADR-0012, `SESSION-TRANSPORT.md`, and the active M10 implementation plan;
3. preserve Cross-Lab identity independence from transport identity and keep all authority fail-closed;
4. do not invent Darkmatter palette values while its authoritative source is absent;
5. keep development/test credential and TLS provisioning explicit and non-release;
6. verify, commit/push, and checkpoint this file after each meaningful M10 milestone.
