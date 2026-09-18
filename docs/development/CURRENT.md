# Current Development State

This file is the durable resume guide for active Cross-Lab development. Git/code/tests are factual state; this file records the intended handoff.

## Current Phase

**M10 first real Linux + Android platform slice is in implementation.**

M1-M9 are complete. M10 Tasks 3-6 are integrated on `main`. Task 7, the narrow UniFFI mobile facade, is implementation-complete on PR #36 and is at the final documentation/integration gate.

## Canonical Baseline

- Canonical `main`: `5d13322f556d6e5c2a054f47e6981bfc609a0911` (Task 6 / PR #35 merged).
- Task 6 PR: #35, merged as `5d13322f556d6e5c2a054f47e6981bfc609a0911`.
- Active Task 7 branch: `m10-task7-mobile-ffi`.
- Task 7 PR: #36 (`feat(mobile): add narrow UniFFI facade`).
- Verified Task 7 implementation head: `03a8beb166950ecba252e984696b8f8bdf9a1bbe`.
- Exact-head Task 7 implementation CI: GitHub Actions `35314900930` - lockfile verification, dependency audit, theme validation, formatting, workspace check, handwritten-unsafe boundary check, Kotlin binding generation, Linux desktop build, Clippy with warnings denied, and full workspace tests all passed.
- Active implementation plan: `docs/superpowers/plans/2026-09-17-m10-first-platform-slice.md`.
- Approved design: `docs/superpowers/specs/2026-09-17-m10-platform-shell-design.md`.
- Master Architecture revision 2.3 and accepted ADR-0012 remain the M10 design baseline.

## Completed M10 Runtime / Quinn / Desktop Checkpoints

Tasks 3-6 are integrated and verified:

- `crates/runtime` owns reusable platform-neutral session/dispatcher/transport coordination and presentation-safe runtime snapshots;
- transport loss, peer revocation, owner-root replacement, and Device Signing authority replacement fail closed;
- capability-event subscriptions and actor commands are bounded;
- Quinn endpoint wrappers require explicit TLS certificate/trust material and derive ADR-0008 channel binding only after the full QUIC/TLS handshake;
- Cross-Lab identity/trust remain independent from TLS/Quinn identity;
- authenticated sessions and reconnects require fresh Cross-Lab session authority;
- the Linux desktop shell uses Rust + GPUI + GPUI Kit with the approved feature-first structure;
- the desktop maps the renderer-neutral theme contract locally, keeps Darkmatter unavailable while its canonical palette is absent, and exposes only presentation-safe runtime state.

## Task 7 Mobile FFI Checkpoint

**Implementation is complete and the implementation head is fully green.**

Task 7 provides:

- a deliberately small `crates/mobile-ffi` Rust boundary using `uniffi 0.32.1`;
- `lib` + `cdylib` output for Rust tests/integration and mobile foreign-language loading;
- UniFFI-safe owned records/enums for lifecycle, trust, connectivity, logical-session, protocol, network, transport-security, metered, and capability-count presentation state;
- typed lifecycle errors with fixed redacted messages;
- explicit start/stop semantics and duplicate-start/not-started handling;
- bounded latest-event delivery with deterministic oldest-entry eviction;
- active-session-only `SessionId` projection so stale inactive session identifiers are not exported;
- a Rust-side `RuntimeStatus` bridge that is intentionally not exported through UniFFI and can be wired to the runtime actor in Task 9;
- no credentials, private keys, authentication secrets, channel-binding bytes, raw Quinn objects, payload material, theme data, or internal domain references in the UniFFI surface;
- deterministic Kotlin binding generation from the built library's embedded UniFFI metadata;
- CI enforcement that handwritten unsafe blocks/functions are absent from `crates/mobile-ffi/src`, while permitting the generated FFI scaffolding required by UniFFI;
- tests for start/stop, duplicate lifecycle calls, owned snapshot conversion/redaction, connection/session conversion, stale-session suppression, bounded delivery, and sensitive-error redaction.

The uploaded UniFFI research snapshot is `0.32.0`; the published dependency used by Cross-Lab is `0.32.1`. Its actual published feature surface does not contain the research snapshot's `macro-scaffolding` feature, so Task 7 uses the maintained published API instead of copying snapshot assumptions.

The final documentation-only PR head created by this checkpoint must also pass exact-head CI before PR #36 is merged.

## First M10 Slice

The first slice remains **authenticated local device connection + device status** between Linux desktop and Android:

- Linux desktop: Rust + GPUI + GPUI Kit.
- Android: Kotlin + Jetpack Compose over one narrow shared-Rust mobile facade.
- Shared coordination: `crosslab-runtime`.
- Mobile FFI: `crosslab-mobile-ffi`; internal domain crates are not exported independently.
- Transport: Quinn local/LAN using existing Cross-Lab channel-bound authentication/session semantics.
- Scope: authenticated connection, safe device/trust/connectivity/session status, clean disconnect/fresh reconnect, revocation, and bounded lifecycle ownership.
- Pairing/bootstrap UI, clipboard, file transfer, BLE/Wi-Fi Direct, iOS, Windows, and production Iroh promotion remain outside this first slice.

## Security / Lifecycle Constraints

- UI code does not own networking, cryptography, persistence internals, or privileged operations.
- FFI DTOs never expose private keys, credentials, authentication secrets, channel-binding bytes, sensitive payloads, or raw transport objects.
- The mobile facade remains narrow; Rust-domain integration hooks are not foreign exports.
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

## Task 8 Preflight Recheck

Official dependency documentation was rechecked on 2026-09-18 before Android scaffolding:

- AGP `9.4.0` requires/defaults Gradle `9.6.0`, JDK `17`, and NDK `28.2.13676358`, and supports API 37.
- Current stable Compose setup has moved beyond the plan's original assumptions: Compose 1.12 requires `compileSdk 37`; the official setup documentation now lists stable BOM `2026.09.00`.
- Google Play currently requires new apps/updates to target API 36 or higher, so `targetSdk 36` remains valid while `compileSdk` must be 37 for current Compose.
- Kotlin `2.4.20` is current, but JetBrains documents full AGP compatibility only through AGP `9.3.1`. AGP 9.4 uses built-in Kotlin and carries KGP `2.2.10` by default.
- Do not combine AGP 9.4 and external KGP 2.4.20 merely because both are individually current. Resolve the Android toolchain as one verified compatible set before committing Gradle files.
- No owner-defined Android minimum OS version is recorded in the architecture. The Task 8 branch must choose `minSdk` from actual Compose/native/Keystore requirements and record the rationale rather than guessing.

## Exact Next Task

Finish Task 7 integration, then execute **M10 Task 8 - Android Kotlin/Compose application scaffold**:

1. require exact-head CI for this final Task 7 documentation head, mark PR #36 ready, merge with the verified expected head, and confirm the resulting `main` head;
2. create a fresh Task 8 branch from the verified Task 7 `main`;
3. resolve the rechecked Android toolchain compatibility before editing Gradle: prefer a single officially supported AGP/Kotlin/Compose combination over individually newer incompatible versions;
4. set `compileSdk 37`; keep `targetSdk 36` unless a newer Play requirement or consumed API requires otherwise;
5. choose and document `minSdk` only after checking actual Compose, native Rust/NDK, lifecycle/network, and future Keystore requirements;
6. add tests first for theme parsing/mapping, theme selection, runtime lifecycle/network commands, and device presentation state;
7. scaffold the feature-first Kotlin/Compose application with application/lifecycle ownership outside composables;
8. add deterministic Rust Android ABI + UniFFI Kotlin generation/packaging for only the M10-required device/emulator ABIs;
9. verify clean-checkout binding generation, Android unit tests, debug assembly, Rust gates, and CI before checkpointing and merging.

## Resume Procedure

1. verify canonical `main`, active feature branch, PR state, exact-head CI, and recent commits before editing;
2. read Master Architecture revision 2.3, ADR-0008, ADR-0009, ADR-0012, `SESSION-TRANSPORT.md`, and the active M10 implementation plan;
3. preserve Cross-Lab identity independence from transport identity and keep all authority fail-closed;
4. keep Darkmatter unavailable until the authoritative palette is supplied;
5. keep development/test credential and TLS provisioning explicit and non-release;
6. verify, commit/push, and checkpoint this file after each meaningful M10 milestone.
