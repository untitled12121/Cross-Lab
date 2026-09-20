# Current Development State

This file is the durable resume guide for active Cross-Lab development. Git/code/tests are factual state; this file records the intended handoff.

## Current Phase

**M10 first real Linux + Android platform slice remains in implementation.**

M1-M9 are complete. M10 Tasks 3-9 are integrated on `main`. Task 10 host-side CI/evidence infrastructure and the physical-evidence execution tooling are now integrated on `main` through PR #40. The PR #40 exact-head CI is fully green, but the mandatory physical Linux + Android lifecycle/security evidence is still pending. M10 must remain open until that evidence is collected and reviewed.

## Canonical Baseline

- Canonical Task 10 host-gate integration baseline: `main` merge `b31ecd3a8d6b9aaeb0ebae8944ab0c2b0d3b8db5` (PR #39).
- Task 9 PR: #38, merged as `f19aa4bb89d744f2ebf0f0fa2b204929fd8495a4`.
- Task 9 final exact-head CI: GitHub Actions `35435754375` on `f078f7b5e0c3a4b20c1aa3d38a3b29f592cc7152`, Android and Rust both fully green.
- Task 10 host-gate PR: #39, merged as `b31ecd3a8d6b9aaeb0ebae8944ab0c2b0d3b8db5`.
- Verified Task 10 host-gate implementation head: `fad35fd2f3535d148a64287fdde391d1613b720e`, CI `35438748708` fully green.
- Task 10 final documentation head: `969ca58a3fec716ab408a88508039b19fbe0fd7f`, exact-head CI `35459098042` fully green before merge.
- Physical-evidence enablement PR: #40, merged as `c183b3c2dba3eb14f5063024d6d9deab996fae10`.
- Build orchestration PR: #41, merged as `062773aaa8eda64e4aa5a8f3cae70a5893cf7dfd`; exact-head CI `35485671117` fully green on `ca3b502b42d7eb7620d8d06a691eb663c5369b42`.
- Build help PR: #42, merged as `a8659c24f459f760e3a8365e68397e077f909251`; exact-head CI `35489274255` fully green on `57eaedc297dc6b7d72d4e627707078d933b059ba`, including direct verification of `cargo crosslab --help`.
- Real-hardware DX fixes PR: #43, merged as `f8ec5e1e9ebfe32fa88af17abbecb9001e471059`; exact-head CI `35490894046` fully green on `dab3742143a781bfc9fcb7f033bbdcae0f8cc96b`. It adds user-local Android SDK bootstrap through `cargo crosslab setup`, broader SDK discovery, and display-aware desktop window sizing.
- Owner/device management PR: #44, merged as `97bd9c534870e43db4ac4535704012f5214184d7`; exact-head CI `35507648082` fully green on `7f5953c82faedede9a08111136890a627122ec88`. It adds desktop and Android Devices/Owner control-center surfaces, presentation-safe owner/local/peer identity summaries, and development-slice peer disconnect/reconnect/revocation controls through existing runtime/trust boundaries.
- Platform signing-provider PR: #45, merged as `dfca18aae64ff45e1861a54743691ebc90d6210a`; exact-head CI `35510597391` and Fuzz Smoke `35510597425` fully green on `d6c33d0cd48a8a0d6a23de47e5db7f1c6ebed96b`. ADR-0013 and Master Architecture revision 2.4 establish a fallible signing-provider boundary so production platform adapters need not export private key bytes.
- Product pairing bootstrap PR: #46, merged as `90812c4abd9e41faf844d9ccbca6ca1bacd8f5b2`; exact-head CI `35516012516` fully green on `f6ee5497adc5c208270f7a78925aca7bf7aee5af`. ADR-0014 and Master Architecture revision 2.5 define the secret-bearing `crosslab:pair:v1:` envelope while keeping discovery/transport hints non-authoritative.
- PR #40 exact-head implementation: `13bb1103f42e35a3beb0d3ad2b2c8cc7befe93d8`, CI `35479944418` fully green.
- Physical-evidence continuation branch: `m10-task10-real-device-evidence`; continue it from the current `main` checkpoint when a physical Linux + Android pair is available.
- Evidence protocol: `docs/research/M10-platform-evidence.md`.
- Active implementation plan: `docs/superpowers/plans/2026-09-17-m10-first-platform-slice.md`.
- Owner/device management usability plan: `docs/superpowers/plans/2026-09-20-m10-owner-device-management.md` (implemented through PR #44).
- Product pairing/signing foundation plan: `docs/superpowers/plans/2026-09-20-product-pairing-signing-foundation.md`; Tasks 1-2 are implemented through PRs #45-#46. Task 3 (platform identity-store design) is next.
- Approved design: `docs/superpowers/specs/2026-09-17-m10-platform-shell-design.md`.
- Master Architecture revision 2.5 plus ADR-0008, ADR-0009, ADR-0012, ADR-0013, and ADR-0014 are the current architecture baseline.

## Integrated M10 Platform Slice

Tasks 3-9 are integrated and verified:

- `crates/runtime` owns reusable platform-neutral session/dispatcher/transport coordination and presentation-safe runtime snapshots;
- transport loss, peer revocation, owner-root replacement, and Device Signing authority replacement fail closed;
- Quinn endpoint wrappers require explicit TLS certificate/trust material and derive ADR-0008 channel binding only after the full QUIC/TLS handshake;
- Cross-Lab identity/trust remain independent from TLS/Quinn identity;
- authenticated reconnects require fresh Cross-Lab session authority;
- Linux desktop uses Rust + GPUI + GPUI Kit and observes presentation-safe runtime state without putting networking/crypto in UI code;
- Android uses Kotlin + Compose over the single narrow `crosslab-mobile-ffi` UniFFI boundary;
- mobile runtime event delivery is bounded and event-driven rather than polling;
- explicit development provisioning is non-default, Rust-owned, unavailable by default in release configuration, and rejects group/world-readable files on Unix;
- Android development provisioning is opt-in for debuggable builds and passes only an app-private provisioning-file path into Rust;
- product-API integration tests cover authenticated connection, disconnect, fresh-session reconnect, revocation/reconnect denial, and presentation-state redaction;
- the first Linux/Android development slice can build with the real runtime + Quinn development wiring enabled.
- the unified Rust-native `cargo crosslab` build orchestrator is integrated, including host-aware desktop/Android builds, Android device install, development-provisioning builds, setup/doctor commands, and CI coverage of the same builder path.
- the self-documenting `cargo crosslab --help` entry point lists all current commands, flags, defaults, host behavior, and examples and is verified directly in CI.
- `cargo crosslab setup` can bootstrap the pinned Android command-line SDK into a user-owned data directory when no compatible SDK is installed; SDK license acceptance remains explicit and interactive.
- desktop startup clamps the initial window to the available primary display and enforces a minimum usable size, avoiding oversized windowed launches on scaled/smaller displays.
- Linux desktop and Android now expose a Devices + Owner control-center surface instead of only a passive status shell; Cross-Lab continues to use the owner trust domain rather than a mandatory cloud/email account.
- presentation-safe owner/local/peer identifiers flow through runtime/FFI without exposing keys, credentials, channel binding, or raw transport objects.
- the development hardware slice has explicit user-triggered disconnect/reconnect controls, while desktop development revocation uses the existing signed trust-transition path.

## Task 10 Host CI / Evidence Checkpoint

**Host CI infrastructure is complete and green; physical-device evidence is not yet complete.**

Task 10 currently provides:

- the original Rust gate unchanged:
  - `cargo metadata --locked --no-deps --format-version 1`;
  - `cargo audit`;
  - `cargo fmt --check`;
  - `cargo check --workspace --all-targets --all-features`;
  - `cargo clippy --workspace --all-targets --all-features -- -D warnings`;
  - `cargo test --workspace --all-features`;
- deterministic UniFFI Kotlin binding generation;
- default Linux desktop build plus Linux desktop build with `development-provisioning`;
- Android JVM unit tests;
- default Android debug assembly;
- Android development-wiring debug assembly using `-PcrosslabDevelopmentProvisioning=true`;
- a secret-safe real-device evidence protocol with explicit environment, lifecycle, reconnect, revocation, shutdown, duplicate-runtime, resource-behavior, and theme evidence rows;
- Android `INTERNET` permission required by the real Quinn connection;
- a development-only generator for synthetic throwaway Linux/Android identity + TLS provisioning that refuses relative/in-repository output and private-file overwrite;
- a development-only Linux `SIGUSR1` revocation trigger routed through the runtime actor and signed trust-transition path;
- development Quinn coverage proving peer revocation rejects a fresh reconnect.

Host verification on `fad35fd2f3535d148a64287fdde391d1613b720e`, run `35438748708`, is fully green. The merged physical-evidence enablement head `13bb1103f42e35a3beb0d3ad2b2c8cc7befe93d8`, run `35479944418`, is also fully green.

The following Task 10 criteria remain physically unverified and **must not be inferred from CI**:

- Android foreground -> background -> foreground;
- local network loss -> restoration;
- Linux/Android disconnect -> fresh authenticated reconnect;
- revocation -> reconnect denial on the physical pair;
- clean explicit shutdown;
- no duplicate runtime/session after lifecycle churn;
- bounded idle/reconnect behavior on real devices;
- Ayu Light rendering/selection on the physical pair.

Darkmatter/System-dark remains blocked until an authoritative Darkmatter palette is supplied.

## First M10 Slice

The slice remains **authenticated local device connection + device status** between Linux desktop and Android:

- Linux desktop: Rust + GPUI + GPUI Kit.
- Android: Kotlin + Jetpack Compose over one narrow shared-Rust mobile facade.
- Shared coordination: `crosslab-runtime`.
- Mobile FFI: `crosslab-mobile-ffi`; internal domain crates are not independently exported.
- Transport: Quinn local/LAN using existing Cross-Lab channel-bound authentication/session semantics.
- Development endpoint/provisioning is explicit; mDNS/BLE discovery remains outside this slice.
- Pairing/bootstrap UI, clipboard, file transfer, BLE/Wi-Fi Direct, iOS, Windows, and production Iroh promotion remain outside the first slice.

## Security / Lifecycle Constraints

- UI code does not own networking, cryptography, persistence internals, or privileged operations.
- FFI DTOs never expose private keys, credentials, authentication secrets, channel-binding bytes, sensitive payloads, or raw transport objects.
- Development provisioning remains an explicit non-default feature and must never be enabled silently in release builds.
- Development provisioning files contain sensitive material and must never be committed or logged.
- Production Quinn certificate provisioning/pinning remains a separate reviewed decision.
- Persistent Android production identity material still requires a reviewed Keystore/StrongBox adapter.
- Signing-sensitive shared domain APIs now accept the ADR-0013 `SigningProvider` boundary; software `SigningKey` remains for simulator/tests/development provisioning, while production platform adapters must provide protected provider implementations and fail closed on provider error.
- Do not promote Iroh in M10; preserve ADR-0009 and `Libp2p trigger: no`.
- Do not invent the missing canonical Darkmatter palette.
- Reconnect always requires fresh Cross-Lab authentication/session authority.

## Deferred / External Risks

- Durable rollback-resistant persistence for `OwnerAuthorityState` and accepted authority epochs.
- Physical Linux + Android lifecycle/background/networking evidence required by Task 10.
- Android production secure-keystore evidence.
- Windows/macOS platform networking and firewall evidence.
- Production Quinn certificate issuance/pinning lifecycle.
- Production persistence/rotation/privacy policy for stable Iroh transport keys.
- `paste 1.0.15` / `RUSTSEC-2024-0436` in the isolated Iroh dependency graph.
- `main` branch protection and other repository-administration controls.

## Exact Next Task

Two tracks remain distinct:

1. **M10 completion track:** collect the mandatory physical Linux + Android lifecycle/security/resource evidence on `m10-task10-real-device-evidence`; Task 11/M10 completion remains blocked until that evidence exists.
2. **Product pairing track:** Tasks 1-2 of `docs/superpowers/plans/2026-09-20-product-pairing-signing-foundation.md` are merged. Next implement Task 3: define the production platform identity-store boundary for authority metadata, trust state, and `SigningProvider` handles, then design Linux and Android adapters with explicit confidentiality, lifecycle, backup/restore, and rollback behavior.
3. Do not use development provisioning as production identity persistence and do not expose private key bytes through UI/FFI/protocol layers.
4. Darkmatter/System-dark remains blocked until an authoritative palette is supplied.

## Resume Procedure

1. verify canonical `main`, active evidence branch, PR state, exact-head CI, and recent commits before editing;
2. read Master Architecture revision 2.3, ADR-0008, ADR-0009, ADR-0012, `SESSION-TRANSPORT.md`, the active M10 plan, and `M10-platform-evidence.md`;
3. preserve Cross-Lab identity independence from transport identity and keep authority fail-closed;
4. keep Darkmatter unavailable until the authoritative palette is supplied;
5. keep development/test credential and TLS provisioning explicit, non-release, private, and out of logs/source control;
6. checkpoint `CURRENT.md` after every real-device evidence milestone.
