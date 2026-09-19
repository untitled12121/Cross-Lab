# Current Development State

This file is the durable resume guide for active Cross-Lab development. Git/code/tests are factual state; this file records the intended handoff.

## Current Phase

**M10 first real Linux + Android platform slice is in implementation.**

M1-M9 are complete. M10 Tasks 3-8 are integrated on `main`. Task 9, shared runtime + Quinn development wiring for Linux desktop and Android, is implementation-complete on PR #38 with a fully green implementation head.

## Canonical Baseline

- Canonical `main`: `66ccb2d71ba9133068c64dc9d9c40ea5ab992230` (Task 8 / PR #37 merged).
- Task 8 PR: #37, merged as `66ccb2d71ba9133068c64dc9d9c40ea5ab992230`.
- Active Task 9 branch: `m10-task9-runtime-quinn-wiring`.
- Task 9 PR: #38 (`feat(m10): wire runtime to Quinn platform consumers`), still draft until the final documentation head is verified.
- Verified Task 9 implementation head: `60ffcaba2df2612f2e183df11abdf27b03e36c0f`.
- Exact-head Task 9 implementation CI: GitHub Actions `35435001835` - Android unit tests/debug assembly plus the full Rust gate all passed, including lockfile verification, dependency audit, theme validation, `cargo fmt --check`, workspace/all-target/all-feature check, mobile handwritten-unsafe enforcement, UniFFI Kotlin generation, Linux desktop build, Clippy with warnings denied, and full workspace/all-feature tests.
- Active implementation plan: `docs/superpowers/plans/2026-09-17-m10-first-platform-slice.md`.
- Approved design: `docs/superpowers/specs/2026-09-17-m10-platform-shell-design.md`.
- Master Architecture revision 2.3 plus ADR-0008, ADR-0009, and ADR-0012 remain the M10 architecture baseline.

## Completed M10 Runtime / Quinn / Desktop / Mobile FFI Checkpoints

Tasks 3-8 are integrated and verified:

- `crates/runtime` owns reusable platform-neutral session/dispatcher/transport coordination and presentation-safe runtime snapshots;
- transport loss, peer revocation, owner-root replacement, and Device Signing authority replacement fail closed;
- capability-event subscriptions and actor commands are bounded;
- Quinn endpoint wrappers require explicit TLS certificate/trust material and derive ADR-0008 channel binding only after the full QUIC/TLS handshake;
- Cross-Lab identity/trust remain independent from TLS/Quinn identity;
- authenticated sessions and reconnects require fresh Cross-Lab session authority;
- the Linux desktop shell uses Rust + GPUI + GPUI Kit with the approved feature-first structure;
- the desktop maps the renderer-neutral theme contract locally and keeps Darkmatter unavailable while its canonical palette is absent;
- `crates/mobile-ffi` exposes one narrow UniFFI boundary using `uniffi 0.32.1`;
- the FFI exports only owned lifecycle/trust/connectivity/session/protocol/network/security/metered/capability presentation data and redacted typed lifecycle errors;
- deterministic Kotlin binding generation is enforced from embedded UniFFI metadata;
- handwritten unsafe remains prohibited in `crates/mobile-ffi/src`;
- the Android Kotlin/Compose shell owns lifecycle/network state outside composables, builds the shared Rust `arm64-v8a` library deterministically, and consumes generated UniFFI Kotlin sources.

## Task 9 Runtime + Quinn Wiring Checkpoint

**Implementation is complete and the implementation head is fully green.**

Task 9 now provides:

- a product-API two-peer integration harness using real owner/device credentials, explicit TLS trust, public Quinn authenticated-session APIs, `RuntimeNode`, and `RuntimeActor`;
- proof that authenticated peers agree on peer IDs/session ID and reach trusted/active state only after existing Cross-Lab proofs complete;
- fail-closed disconnect behavior, fresh `SessionId` on reconnect, peer-revocation handling, reconnect rejection after revocation, and presentation-output redaction checks;
- actor-level peer-revocation commands so platform consumers do not mutate runtime nodes directly;
- an event-driven Quinn transport-close signal used by platform runtime bridges without polling;
- a non-default `development-provisioning` feature under `crosslab-transport-quic` for explicit M10 development configuration only;
- explicit client/server development provisioning that reconstructs Cross-Lab owner/device/trust authority in Rust and keeps TLS identity separate from Cross-Lab identity;
- Unix development provisioning files are rejected when group/world permissions are present;
- production builds do not enable development provisioning by default;
- the mobile facade owns bounded runtime events and exposes blocking event wait rather than periodic polling;
- mobile network-loss/network-restoration commands fail closed and clear stale session-facing state;
- the Android `MobileRuntimePort` consumes real generated UniFFI runtime snapshots instead of the former disconnected placeholder;
- Android development provisioning is opt-in through the build property `crosslabDevelopmentProvisioning`, is restricted to debuggable application builds, and passes only an app-private provisioning-file path into Rust;
- Android background/foreground lifecycle stops and reconfigures the development connector rather than retaining a stale authenticated session;
- the Linux desktop devices feature owns a development Quinn server/runtime controller outside GPUI rendering code;
- the desktop server remains listening across disconnects and uses a fresh authenticated Quinn/Cross-Lab session for reconnect;
- GPUI observes presentation-safe runtime status through a watch channel and redraws the page with `cx.notify()`; UI code does not own networking, crypto, credentials, or transport objects;
- an end-to-end mobile-facade integration test verifies authenticated Quinn state, peer/session identity mapping, disconnect, and fresh-session reconnect.

The final documentation-only Task 9 head created by this checkpoint must also pass exact-head CI before PR #38 is marked ready and merged.

## First M10 Slice

The first slice is **authenticated local device connection + device status** between Linux desktop and Android:

- Linux desktop: Rust + GPUI + GPUI Kit.
- Android: Kotlin + Jetpack Compose over one narrow shared-Rust mobile facade.
- Shared coordination: `crosslab-runtime`.
- Mobile FFI: `crosslab-mobile-ffi`; internal domain crates are not exported independently.
- Transport: Quinn local/LAN using existing Cross-Lab channel-bound authentication/session semantics.
- Development endpoint selection/provisioning is explicit; mDNS/BLE discovery remains outside Task 9.
- Scope: authenticated connection, safe device/trust/connectivity/session status, clean disconnect/fresh reconnect, revocation, and bounded lifecycle ownership.
- Pairing/bootstrap UI, clipboard, file transfer, BLE/Wi-Fi Direct, iOS, Windows, and production Iroh promotion remain outside this first slice.

## Security / Lifecycle Constraints

- UI code does not own networking, cryptography, persistence internals, or privileged operations.
- FFI DTOs never expose private keys, credentials, authentication secrets, channel-binding bytes, sensitive payloads, or raw transport objects.
- The mobile facade remains narrow; Rust-domain integration hooks are not foreign exports.
- Development provisioning is an explicit non-default feature and must remain unavailable in release configuration by default.
- Development provisioning files contain sensitive material and must never be committed or logged.
- Production Quinn certificate provisioning/pinning remains a separate reviewed decision.
- Persistent Android production identity material still requires a reviewed Keystore/StrongBox adapter.
- Do not promote Iroh in M10; preserve ADR-0009 and `Libp2p trigger: no`.
- Do not invent the missing canonical Darkmatter palette.
- Reconnect always requires fresh Cross-Lab authentication/session authority.

## Deferred / External Risks

- Durable rollback-resistant persistence for `OwnerAuthorityState` and accepted authority epochs.
- Real Android lifecycle/background-networking/secure-keystore evidence is still required by M10 Task 10; CI does not substitute for device evidence.
- Windows/macOS platform networking and firewall evidence.
- Production Quinn certificate issuance/pinning lifecycle.
- Production persistence/rotation/privacy policy for stable Iroh transport keys.
- `paste 1.0.15` / `RUSTSEC-2024-0436` in the isolated Iroh dependency graph.
- `main` branch protection and other repository-administration controls.

## Exact Next Task

Finish Task 9 integration, then execute **M10 Task 10 - CI gates and real-device evidence**:

1. require exact-head CI for this final Task 9 documentation head, mark PR #38 ready, merge with the verified expected head, and confirm the resulting `main` head;
2. create a fresh Task 10 branch from the verified Task 9 `main`;
3. preserve the full Rust gate exactly and keep Android host-side unit/build/binding checks reproducible;
4. create `docs/research/M10-platform-evidence.md` with a clear evidence template that contains no secrets;
5. collect/record real Linux + Android device/OS/build identifiers and evidence for foreground -> background -> foreground, local network loss -> restoration, disconnect -> fresh authenticated reconnect, revocation -> reconnect denial, and clean explicit shutdown;
6. record bounded idle/reconnect behavior and verify that lifecycle churn does not create duplicate runtimes;
7. record Ayu Light theme-selection behavior; keep Darkmatter/System-dark explicitly incomplete unless an authoritative Darkmatter palette is supplied;
8. do not claim M10 complete from CI alone: the real-device lifecycle criterion in the active plan is mandatory;
9. checkpoint `CURRENT.md` with exact Task 10 evidence identifiers and verification status before Task 11 final integration.

## Resume Procedure

1. verify canonical `main`, active feature branch, PR state, exact-head CI, and recent commits before editing;
2. read Master Architecture revision 2.3, ADR-0008, ADR-0009, ADR-0012, `SESSION-TRANSPORT.md`, and the active M10 implementation plan;
3. preserve Cross-Lab identity independence from transport identity and keep all authority fail-closed;
4. keep Darkmatter unavailable until the authoritative palette is supplied;
5. keep development/test credential and TLS provisioning explicit, non-release, private, and out of logs/source control;
6. verify, commit/push, and checkpoint this file after each meaningful M10 milestone.
