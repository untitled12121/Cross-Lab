# Current Development State

This file is the durable resume guide for active Cross-Lab development. Git/code/tests are factual state; this file records the intended handoff.

## Current Phase

**M10 first real Linux + Android platform slice is in implementation.**

M1-M9 are complete. M10 Tasks 3-7 are integrated on `main`. Task 8, the Android Kotlin/Compose application scaffold, is implementation-complete on PR #37 and has a fully green implementation head.

## Canonical Baseline

- Canonical `main`: `5226206d87f98c081c0e7e39e8d410ace9126fef` (Task 7 / PR #36 merged).
- Task 7 PR: #36, merged as `5226206d87f98c081c0e7e39e8d410ace9126fef`.
- Active Task 8 branch: `m10-task8-android-shell`.
- Task 8 PR: #37 (`feat(android): add Kotlin Compose application scaffold`).
- Verified Task 8 implementation head: `313df8e9edacbc1ef8e968f37ce11d3ec55cbb1a`.
- Exact-head Task 8 implementation CI: GitHub Actions `35332114096` - Android SDK/toolchain setup, Gradle wrapper verification, Android unit tests, debug assembly, Rust lockfile verification, dependency audit, theme validation, formatting, workspace check, handwritten-unsafe boundary check, UniFFI Kotlin generation, Linux desktop build, Clippy with warnings denied, and full workspace tests all passed.
- Active implementation plan: `docs/superpowers/plans/2026-09-17-m10-first-platform-slice.md`.
- Approved design: `docs/superpowers/specs/2026-09-17-m10-platform-shell-design.md`.
- Master Architecture revision 2.3 and accepted ADR-0012 remain the M10 design baseline.

## Completed M10 Runtime / Quinn / Desktop / Mobile FFI Checkpoints

Tasks 3-7 are integrated and verified:

- `crates/runtime` owns reusable platform-neutral session/dispatcher/transport coordination and presentation-safe runtime snapshots;
- transport loss, peer revocation, owner-root replacement, and Device Signing authority replacement fail closed;
- capability-event subscriptions and actor commands are bounded;
- Quinn endpoint wrappers require explicit TLS certificate/trust material and derive ADR-0008 channel binding only after the full QUIC/TLS handshake;
- Cross-Lab identity/trust remain independent from TLS/Quinn identity;
- authenticated sessions and reconnects require fresh Cross-Lab session authority;
- the Linux desktop shell uses Rust + GPUI + GPUI Kit with the approved feature-first structure;
- the desktop maps the renderer-neutral theme contract locally, keeps Darkmatter unavailable while its canonical palette is absent, and exposes only presentation-safe runtime state;
- `crates/mobile-ffi` exposes one narrow UniFFI boundary using `uniffi 0.32.1`;
- the FFI exports only owned lifecycle/trust/connectivity/session/protocol/network/security/metered/capability presentation data and redacted typed lifecycle errors;
- the Rust-side `RuntimeStatus` bridge remains internal and intentionally ready for Task 9 runtime-actor wiring;
- deterministic Kotlin binding generation is enforced from embedded UniFFI metadata;
- handwritten unsafe remains prohibited in `crates/mobile-ffi/src`.

## Task 8 Android Application Checkpoint

**Implementation is complete and the implementation head is fully green.**

Task 8 provides:

- a feature-first Android application under `apps/android` using Kotlin + Jetpack Compose;
- application-scoped lifecycle/network ownership outside composables;
- an explicit disconnected runtime port rather than fabricated connectivity before Task 9;
- local mapping of the canonical Cross-Lab theme contract with sharp radius-0 UI primitives;
- presentation-safe device/runtime state and status components;
- tests for theme parsing/selection, runtime lifecycle/network handling, and device presentation mapping;
- the official Gradle 9.5 wrapper;
- a deterministic Android toolchain: AGP `9.3.1`, Kotlin/Compose compiler `2.4.20`, Compose BOM `2026.09.00`, JDK 17, NDK `28.2.13676358`, `compileSdk 37` + `compileSdkMinor 0`, `targetSdk 36`, and `minSdk 23`;
- CI installation of the published minor-versioned Android SDK package `platforms/android-37.0`;
- deterministic Rust `arm64-v8a` compilation and JNI packaging for `crosslab-mobile-ffi`;
- deterministic UniFFI Kotlin generation before Android compilation;
- serialized native build/binding generation to avoid concurrent rustup/Cargo toolchain races;
- explicit NDK C/C++/archive toolchain environment for native dependency build scripts;
- green `:app:testDebugUnitTest` with 12 tests and green `:app:assembleDebug`.

The final documentation-only Task 8 head created by this checkpoint must also pass exact-head CI before PR #37 is marked ready and merged.

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

## Exact Next Task

Finish Task 8 integration, then execute **M10 Task 9 - real desktop/mobile runtime + Quinn development wiring**:

1. require exact-head CI for this final Task 8 documentation head, mark PR #37 ready, merge with the verified expected head, and confirm the resulting `main` head;
2. create a fresh Task 9 branch from the verified Task 8 `main`;
3. re-read the Master Architecture, active M10 plan/design, ADR-0008, ADR-0009, ADR-0012, `SESSION-TRANSPORT.md`, current runtime/Quinn/mobile-FFI code, and relevant uploaded Quinn/UniFFI reference material before editing;
4. wire the desktop and Android/mobile facade to the real `crosslab-runtime` actor and existing Quinn transport using explicit development/test provisioning only;
5. keep Cross-Lab identity/session authority independent from transport identity and require fresh authenticated session authority after reconnect;
6. preserve application-scoped Android lifecycle/network ownership and keep all network/crypto/business logic outside Compose UI;
7. expose only presentation-safe runtime state through the existing narrow mobile facade; do not widen FFI for convenience;
8. add integration/security tests for connection, disconnect, reconnect, revocation/fail-closed behavior, lifecycle transitions, and bounded event delivery;
9. verify Android unit tests/debug assembly, desktop build, Rust format/check/Clippy/tests, generated bindings, and exact-head CI before checkpointing and merging Task 9.

## Resume Procedure

1. verify canonical `main`, active feature branch, PR state, exact-head CI, and recent commits before editing;
2. read Master Architecture revision 2.3, ADR-0008, ADR-0009, ADR-0012, `SESSION-TRANSPORT.md`, and the active M10 implementation plan;
3. preserve Cross-Lab identity independence from transport identity and keep all authority fail-closed;
4. keep Darkmatter unavailable until the authoritative palette is supplied;
5. keep development/test credential and TLS provisioning explicit and non-release;
6. verify, commit/push, and checkpoint this file after each meaningful M10 milestone.
