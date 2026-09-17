# Current Development State

This file is the durable resume guide for active Cross-Lab development. Git/code/tests are factual state; this file records the intended handoff.

## Current Phase

**Phase 1 foundation complete; M10 first real Linux + Android platform slice is approved and entering implementation.**

## Current Milestone

**M10 — First Platform Vertical Slice, architecture/design integration checkpoint.**

M1–M9 are complete. M9 selected Quinn for local/LAN and Iroh for remote/NAT/relay through accepted ADR-0009. M10 design and ADR-0012 were owner-approved on 2026-09-17; production code starts only after this design checkpoint is merged and verified on `main`.

## Canonical Baseline

- Verified pre-M10 canonical `main`: `064526f688aed0a92dd5b0ae4815f54d5b97d6f6` (`docs: checkpoint M9 completion and M10 handoff`).
- Exact `main` CI for that checkpoint: GitHub Actions `35180292718` — passed.
- M9 Task 10 PR: #29 (`docs(networking): accept ADR-0009 and complete M9`).
- M9 Task 10 merge commit: `918a2988eb0d79dc57797e1e9d35dc46e9a97b58`.
- Accepted remote-networking decision: `docs/adr/ADR-0009-remote-networking.md`.
- M9 evidence report: `docs/research/M9-networking-evidence.md`.

## Approved M10 Design

- Active design branch: `m10-platform-shell-design`.
- Branch base: verified `main` `064526f688aed0a92dd5b0ae4815f54d5b97d6f6`.
- Approved design: `docs/superpowers/specs/2026-09-17-m10-platform-shell-design.md`.
- Accepted design-system decision: `docs/adr/ADR-0012-cross-platform-design-system.md`.
- Detailed implementation plan: `docs/superpowers/plans/2026-09-17-m10-first-platform-slice.md`.
- Master Architecture revision 2.3 integrates ADR-0012.
- No M10 production code is part of this design branch.

## First M10 Slice

The first slice is **authenticated local device connection + device status** between Linux desktop and Android:

- Linux desktop: Rust + GPUI + GPUI Kit using the feature-first page/layout/_components/components-ui/features organization.
- Android: Kotlin-native application with Jetpack Compose for the M10 shell over one narrow shared-Rust mobile façade.
- Shared application coordination: a small platform-neutral Rust runtime boundary is introduced only because desktop and mobile are immediate consumers.
- Mobile FFI: one narrow UniFFI façade; internal crates are not exported independently.
- Transport: existing Quinn local/LAN baseline and existing Cross-Lab channel-bound authentication/session semantics.
- Scope: authenticated connection, safe device/trust/connectivity/session status, clean disconnect/fresh reconnect, revocation behavior, and bounded lifecycle ownership.
- Pairing/bootstrap UI, clipboard, file transfer, BLE/Wi-Fi Direct, iOS, Windows, and production Iroh promotion are outside this first slice.

## Cross-Platform Design Contract

ADR-0012 establishes one Cross-Lab-owned renderer-neutral semantic theme contract used by native renderers.

- `Darkmatter` is the baseline dark theme.
- `Ayu Light` is the baseline light theme.
- `System` resolves OS dark/light appearance to one of those named themes.
- Canonical theme data is semantic and renderer-neutral: OKLCH color roles, typography, radius, spacing, density, borders, control/list metrics, icon metrics, elevation/shadow, and motion values.
- Radius `0` remains the sharp baseline default but is a token, not a hard-coded renderer invariant.
- GPUI/GPUI Kit, Android Kotlin/Compose, and future iOS Swift map the same contract locally; presentation data does not travel through the Cross-Lab peer protocol or mobile core FFI.
- React/CSS/Tailwind/screenshots supplied by the owner are visual references translated into native Cross-Lab UI, not a web runtime dependency.
- The authoritative Darkmatter source is still absent from the verified project context. Do not invent or infer its palette. M10 may progress on schema/runtime/UI structure and Ayu Light, but Darkmatter visual completion stays open until the source is supplied.

## Verified Dependency/Tooling References for the Plan

Verified on 2026-09-17 and re-check before each first dependency commit:

- GPUI Kit `0.6.1` (Apache-2.0).
- UniFFI `0.32.1` (MPL-2.0 dependency; depend on it, do not copy its source).
- Android Gradle Plugin `9.4.0`, Gradle `9.6.0`, JDK `17`.
- Kotlin `2.4.20`.
- Compose BOM `2026.08.00`.
- Android compile/target API `36` for the first Android shell; minimum supported API remains to be evidence-based before first release build.

## Security / Lifecycle Constraints

- UI code does not own networking, cryptography, persistence internals, or privileged operations.
- Android UI composables do not own the Rust runtime; a Kotlin application/service-scoped lifecycle owner sends typed lifecycle/network commands.
- Status snapshots/FFI DTOs must never expose private keys, credentials, authentication secrets, channel-binding bytes, or sensitive payloads.
- Development/test provisioning for already owner-authorized credentials and Quinn TLS material must be explicit and unavailable as an implicit release fallback.
- ADR-0008 left production Quinn certificate provisioning undecided. M10 must not add accept-all certificate verification or silently define certificate identity as Cross-Lab identity.
- Persistent Android production identity material requires an Android Keystore/StrongBox-backed adapter or another explicitly reviewed secure-storage path.
- Do not promote Iroh in the first M10 slice. ADR-0009's Android lifecycle, real-device transitions, secure key storage, dependency audit, and transport-key privacy/rotation gates remain open for future production Iroh use.

## Deferred / External Risks

- Durable rollback-resistant persistence for `OwnerAuthorityState` and accepted authority epochs.
- Real Android/iOS lifecycle, background-networking, entitlement, firewall, and secure-keystore evidence beyond the M10 slice.
- Windows/macOS platform networking and firewall evidence.
- Production Quinn certificate provisioning/pinning lifecycle when a release consumer requires it.
- Production persistence/rotation/privacy policy for stable Iroh transport keys.
- `paste 1.0.15` / `RUSTSEC-2024-0436` in the isolated Iroh dependency graph.
- Explicit authorization/subscription design for future authority-sensitive system-event families.
- `main` branch protection and other repository-administration controls.

## Exact Next Task

Integrate the approved design checkpoint before production code:

1. open a PR from `m10-platform-shell-design` to `main`;
2. require exact-head CI and merge only the verified head;
3. verify post-merge `main` CI;
4. create a fresh implementation branch from that verified `main`;
5. execute `docs/superpowers/plans/2026-09-17-m10-first-platform-slice.md` beginning with the canonical theme-contract foundation and shared runtime extraction, using tests first where meaningful;
6. update this file at each durable implementation checkpoint.

## Resume Procedure

1. verify canonical `main`, active M10 branch, recent commits, and CI before editing;
2. read Master Architecture revision 2.3, ADR-0012, the approved M10 design, and the implementation plan;
3. preserve ADR-0009 and `Libp2p trigger: no`; do not auto-promote Iroh;
4. do not invent Darkmatter palette values while its authoritative source is absent;
5. keep dev/test credential and TLS provisioning explicit and non-release;
6. verify, commit/push, and checkpoint `CURRENT.md` after each meaningful M10 milestone.
