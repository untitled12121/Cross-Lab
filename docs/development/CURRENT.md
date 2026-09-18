# Current Development State

This file is the durable resume guide for active Cross-Lab development. Git/code/tests are factual state; this file records the intended handoff.

## Current Phase

**M10 first real Linux + Android platform slice is in implementation.**

M1-M9 are complete. M10 Tasks 3-5 are integrated on `main`. Task 6, the Linux GPUI desktop shell and local design adapter, is implementation-complete on PR #35 and is at the final documentation/integration gate.

## Canonical Baseline

- Canonical `main` before Task 6: `5b092c96fb80934215d1949db532a434dbf839ad` (`feat(runtime): add lifecycle-safe runtime actor`).
- Task 5 PR: #34, merged as `5b092c96fb80934215d1949db532a434dbf839ad`.
- Active Task 6 branch: `m10-task6-linux-desktop-shell`.
- Task 6 PR: #35 (`feat(desktop): add Linux GPUI shell and design adapter`).
- Verified Task 6 implementation head: `142ddfe7f8f0809e047d936a2b6f5bacb9ad56b2`.
- Exact-head Task 6 implementation CI: GitHub Actions `35306661979` - dependency audit, theme JSON validation, formatting, workspace check, Linux desktop build, Clippy with warnings denied, and full workspace tests all passed.
- Active implementation plan: `docs/superpowers/plans/2026-09-17-m10-first-platform-slice.md`.
- Approved design: `docs/superpowers/specs/2026-09-17-m10-platform-shell-design.md`.
- Master Architecture revision 2.3 and accepted ADR-0012 remain the M10 design baseline.

## Completed M10 Runtime / Quinn Checkpoints

Tasks 3-5 are integrated and verified:

- `crates/runtime` owns reusable platform-neutral session/dispatcher/transport coordination and presentation-safe runtime snapshots;
- transport loss, peer revocation, owner-root replacement, and Device Signing authority replacement fail closed;
- capability-event subscriptions and actor commands are bounded;
- Quinn client/server endpoint wrappers require explicit TLS certificate/trust material;
- ADR-0008 `quic-tls-exporter-v1` binding is derived only after the full QUIC/TLS handshake;
- Cross-Lab identity/trust remain independent from TLS/Quinn identity;
- `AuthenticatedQuicSession` is created only after Cross-Lab authentication becomes active;
- the runtime actor owns one mutable runtime/session lifecycle with explicit stop, deterministic shutdown, network-loss cleanup, and fresh authenticated reconnect.

## Task 6 Linux Desktop Checkpoint

**Implementation is complete and the implementation head is fully green.**

Task 6 provides:

- a Rust + GPUI + GPUI Kit `0.6.1` Linux desktop application shell;
- the approved feature-first desktop structure with `pages/devices/page.rs`, `layout.rs`, page-local `_components`, reusable `components/ui`, and feature logic under `features/`;
- typed parsing and validation for the renderer-neutral theme-v1 contract;
- injected `System` appearance resolution and explicit `ThemeUnavailable(Darkmatter)` while the authoritative Darkmatter palette is absent;
- canonical Ayu Light presentation mapped locally into GPUI/GPUI Kit without making renderer types part of the shared theme contract;
- GPUI-local OKLCH conversion, typography/radius/spacing/control metrics, and radius-0 baseline styling;
- presentation-safe mapping from `crosslab-runtime` status snapshots into device UI state without credentials, private keys, channel-binding bytes, or payload material;
- a disconnected empty state rather than fabricated connected-device data;
- keyboard-focusable navigation, accessible labels, compact layout, thin separators, and restrained semantic status treatment;
- Linux GPUI build dependencies and an explicit desktop build gate in CI.

The first compile attempt exposed missing GPUI trait imports in `apps/desktop/src/main.rs`; the fix imports `AppContext` and `Styled` explicitly. Exact-head CI `35306661979` confirms the corrected shell compiles and the complete repository gate passes.

The final documentation-only PR head created by this checkpoint must also pass exact-head CI before PR #35 is merged.

## First M10 Slice

The first slice remains **authenticated local device connection + device status** between Linux desktop and Android:

- Linux desktop: Rust + GPUI + GPUI Kit.
- Android: Kotlin + Jetpack Compose over one narrow shared-Rust mobile facade.
- Shared coordination: `crosslab-runtime`.
- Mobile FFI: one narrow UniFFI facade; internal crates are not exported independently.
- Transport: Quinn local/LAN using existing Cross-Lab channel-bound authentication/session semantics.
- Scope: authenticated connection, safe device/trust/connectivity/session status, clean disconnect/fresh reconnect, revocation, and bounded lifecycle ownership.
- Pairing/bootstrap UI, clipboard, file transfer, BLE/Wi-Fi Direct, iOS, Windows, and production Iroh promotion remain outside this first slice.

## Security / Lifecycle Constraints

- UI code does not own networking, cryptography, persistence internals, or privileged operations.
- Status snapshots and future FFI DTOs never expose private keys, credentials, authentication secrets, channel-binding bytes, or sensitive payloads.
- Production Quinn certificate provisioning/pinning remains a separate reviewed decision; development/test provisioning must stay explicit.
- Persistent Android production identity material still requires a reviewed Keystore/StrongBox adapter.
- Do not promote Iroh in M10; preserve ADR-0009 and `Libp2p trigger: no`.
- Do not invent the missing canonical Darkmatter palette.
- Reconnect always requires fresh Cross-Lab authentication/session authority.

## Deferred / External Risks

- Durable rollback-resistant persistence for `OwnerAuthorityState` and accepted authority epochs.
- Real Android/iOS lifecycle/background-networking/secure-keystore evidence beyond the current slice.
- Windows/macOS platform networking and firewall evidence.
- Production Quinn certificate issuance/pinning lifecycle.
- Production persistence/rotation/privacy policy for stable Iroh transport keys.
- `paste 1.0.15` / `RUSTSEC-2024-0436` in the isolated Iroh dependency graph.
- `main` branch protection and other repository-administration controls.

## Exact Next Task

Finish Task 6 integration, then execute **M10 Task 7 - narrow UniFFI mobile facade**:

1. require exact-head CI for the final Task 6 documentation head, mark PR #35 ready, merge with the verified expected head, and confirm the resulting `main` head;
2. create a fresh Task 7 branch from the verified Task 6 `main`;
3. re-verify UniFFI before editing Cargo; reviewed baseline is `uniffi 0.32.1` under MPL-2.0;
4. add tests first for lifecycle start/stop, duplicate-start handling, snapshot conversion/redaction, connection/session status conversion, bounded delivery, and sensitive-error redaction;
5. implement one deliberately small `crates/mobile-ffi` facade with only M10-consumed UniFFI-safe records/enums and lifecycle/state delivery;
6. do not export internal domain crates/types, raw Quinn objects, credentials, private keys, channel-binding bytes, or theme data through FFI;
7. add deterministic binding-generation instructions/tooling only because Android is now a real consumer;
8. run focused mobile-ffi tests plus full workspace format/check/Clippy/tests, checkpoint this file, and merge only the verified head.

## Resume Procedure

1. verify canonical `main`, active feature branch, PR state, exact-head CI, and recent commits before editing;
2. read Master Architecture revision 2.3, ADR-0008, ADR-0009, ADR-0012, `SESSION-TRANSPORT.md`, and the active M10 implementation plan;
3. preserve Cross-Lab identity independence from transport identity and keep all authority fail-closed;
4. keep Darkmatter unavailable until the authoritative palette is supplied;
5. keep development/test credential and TLS provisioning explicit and non-release;
6. verify, commit/push, and checkpoint this file after each meaningful M10 milestone.
