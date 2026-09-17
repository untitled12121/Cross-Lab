# M10 First Platform Vertical Slice Design

**Status:** Proposed for owner review  
**Date:** 2026-09-17  
**Milestone:** M10 — First Platform Vertical Slice  
**Base:** `064526f688aed0a92dd5b0ae4815f54d5b97d6f6`

## 1. Purpose

M10 moves Cross-Lab from the verified core/network foundation into the first real Linux desktop + Android platform slice without weakening the architecture proven in M1–M9.

The first slice proves one useful end-to-end behavior: a Linux desktop and an Android device establish a real authenticated Cross-Lab session over the existing Quinn local/LAN transport and expose coherent device, trust, connection, and session status in native Cross-Lab user interfaces.

This milestone also establishes the smallest cross-platform design-system contract needed by those two applications. It does not attempt to build the full future component catalog or promote the M9 Iroh experiment into production.

## 2. Governing Architecture

M10 preserves the accepted platform model:

- Linux/Windows/macOS desktop UI uses Rust + GPUI + GPUI Kit.
- Android uses Kotlin for UI/platform integration over one deliberately narrow shared-Rust façade.
- iOS/iPadOS remains Swift + the same conceptual shared-Rust façade boundary when Apple mobile work begins.
- UI code does not own network protocols, cryptography, persistence internals, or privileged operations.
- Platform-specific APIs remain behind narrow platform adapters.
- Quinn remains the proven local/LAN transport for this first real-device slice.
- Iroh remains selected for future remote/NAT/relay production use only after the ADR-0009 production-promotion obligations are satisfied.

## 3. First Vertical Slice

The first M10 slice is **authenticated local device connection + device status**.

A Linux desktop and Android device must be able to:

1. start their normal unprivileged Cross-Lab runtime;
2. use owner-authorized device credentials and trust state from the existing security model;
3. connect over Quinn on a local network;
4. perform the existing channel-bound Cross-Lab authentication and protocol/capability negotiation;
5. expose the authenticated peer and connection/session state to their UI-facing layers;
6. show coherent device status in both UIs;
7. disconnect cleanly and reconnect through a fresh authenticated session;
8. reject reconnect after trust revocation according to the existing core semantics.

The initial slice does not add pairing/bootstrap UI. Integration and real-device development may provision already owner-authorized credentials through a clearly test/development-only path. No fake cryptography or release-enabled trust bypass is permitted.

## 4. Cross-Platform Visual System

Cross-Lab owns one product visual language across desktop and mobile. Android must not become visually defined by stock Material defaults, and iOS must not become visually defined by stock Cupertino defaults. Native frameworks remain responsible for platform behavior, accessibility, lifecycle, input, safe areas, and OS integration; Cross-Lab controls the visual identity.

The default visual direction is dense and Zed-like:

- sharp geometry;
- compact information density;
- near-flat surfaces and thin separators;
- restrained semantic status colors;
- clear hierarchy without decorative card inflation;
- strong keyboard/focus behavior on desktop;
- appropriate touch targets and accessibility on mobile;
- visible device, trust, security, transport, and connection state.

### 4.1 Theme identities

Cross-Lab defines two baseline theme identities:

- `Darkmatter` — default dark theme;
- `Ayu Light` — default light theme.

`System` is a selector, not a third palette. It resolves to `Darkmatter` when the OS requests dark appearance and `Ayu Light` when the OS requests light appearance. Explicit selection of either named theme overrides the OS appearance until changed by the user.

The exact Darkmatter palette must come from the owner-provided theme/CSS source. The current repository/project context does not contain a reliably identifiable copy of that source, so M10 must not invent Darkmatter color values from screenshots or approximation. The design-system structure may be implemented before those values are re-supplied, but visual acceptance of Darkmatter requires the authoritative source values.

Ayu Light may use GPUI Kit's maintained Ayu theme as an implementation reference where license and compatibility checks permit, but Cross-Lab's canonical cross-platform theme data remains Cross-Lab-owned and renderer-neutral.

### 4.2 Canonical theme contract

CSS/React theme code supplied by the owner is design input, not the runtime architecture. Values are translated into a small versioned Cross-Lab theme document that is independent of GPUI, Compose, SwiftUI, CSS, or any one rendering library.

The initial contract covers:

```text
identity / appearance
semantic colors in OKLCH
font families
font sizes
font weights
line heights
radius scale
spacing scale
density
border widths
control heights
list-row heights
icon size / stroke
shadow / elevation
motion duration / easing
```

Radius `0` remains the baseline Cross-Lab default, but radius is a theme token rather than a hard-coded application constant. Themes may therefore change radius, typography, density, spacing, and related presentation without feature code branching on theme names.

The schema is semantic rather than component-specific. Components consume roles such as surface, foreground, muted, border, accent, destructive, spacing scale, typography scale, and radius scale instead of embedding raw values.

Theme data is presentation state only. It must never alter trust, policy, capability availability, network classification, authorization, or security behavior.

### 4.3 Renderer adapters

The same theme document is mapped by platform adapters:

```text
Cross-Lab theme document
          |
   +------+-------+
   |              |
Desktop        Android
GPUI/Kit       Kotlin UI
   |
future macOS/Windows share desktop renderer

future iOS maps the same contract into Swift UI code
```

The design contract is shared; UI implementation code is not. Platform UIs may adapt navigation and layout to available space while preserving the same tokens, hierarchy, component states, and product identity.

The theme contract does not travel through the Cross-Lab network protocol or the mobile core FFI. Each application owns local presentation configuration.

## 5. UI Framework Decisions

### 5.1 Desktop

The desktop application is a Rust GPUI application using GPUI Kit for compatible primitives and integration while keeping Cross-Lab's own components and styling under `components/ui/`.

The first desktop shell follows the existing feature-first layout:

```text
apps/desktop/src/
├── pages/
│   └── devices/
│       ├── page.rs
│       ├── layout.rs
│       └── _components/
├── components/
│   └── ui/
└── features/
    ├── devices/
    └── appearance/
```

Only components required by the first slice should be introduced. Likely initial primitives are navigation item, panel/section, list row, status badge, button/icon button, and loading/empty/error states. A speculative full component library is out of scope.

### 5.2 Android

Android remains Kotlin-native. For M10, Jetpack Compose is the recommended UI toolkit because its declarative model maps cleanly to semantic theme tokens and state-driven UI while leaving lifecycle, services, permissions, secure storage, and other Android integrations in native Kotlin APIs.

The Android renderer must not directly expose Cross-Lab internal Rust crates. It consumes only the M10 mobile façade and local presentation/theme state.

If the implementation review finds a concrete Compose platform or dependency blocker, the design must return for review rather than silently replacing the mobile UI architecture.

### 5.3 Future iOS

M10 does not scaffold the iOS application. The theme contract and mobile façade must remain suitable for a future Swift implementation, but no speculative Swift code is created in this milestone.

## 6. React/CSS Reference Workflow

When the owner provides React components, CSS, Tailwind classes, screenshots, or similar UI references, they are treated as visual specifications.

Cross-Lab should reproduce the visible hierarchy, layout, spacing, typography, component states, and interaction intent as closely as practical in native GPUI/Kotlin/Swift components. The web framework itself is not imported: no React runtime, DOM, browser shell, or Tailwind-driven application architecture is introduced solely to clone the reference.

Reusable patterns discovered while translating a reference should be promoted into Cross-Lab `components/ui/` or the shared theme contract only when they are genuinely reusable.

## 7. Desktop Runtime Boundary

The desktop UI remains separate from the normal per-user Cross-Lab agent/runtime boundary. UI state observes and commands the application-facing runtime; it does not receive ambient privileged authority.

For M10, implementation should prefer the smallest boundary that preserves this separation. The milestone does not require inventing a privileged service or general local RPC framework unless a concrete operation needs it.

No GPUI component may directly create Quinn connections, perform credential signing, evaluate trust policy, or own long-running network tasks.

## 8. Android Runtime and Mobile FFI

M10 introduces one deliberately narrow mobile façade rather than exporting `crosslab-core`, `crosslab-identity`, `crosslab-policy`, `crosslab-protocol`, or transport crates independently.

The façade is responsible for stable mobile-facing DTOs, commands, state/event delivery, error mapping, threading rules, and lifecycle entry points. Internal domain types remain private unless a specific mobile contract requires a representation.

The first façade should expose only what the device-status slice consumes, conceptually:

```text
runtime lifecycle
local device summary
peer/device summaries
connection/session status
connect/disconnect or start/stop intent required by the selected runtime ownership model
state/event subscription
bounded error/status information
```

Exact function and DTO names are chosen from the existing core APIs during the implementation plan rather than invented as a second domain model.

UniFFI remains the preferred initial binding mechanism. Before implementation, the exact maintained compatible UniFFI version is verified rather than guessed.

## 9. Android Lifecycle Ownership

A UI composable must not own the Rust networking/session runtime. The runtime is application/service scoped behind a Kotlin lifecycle owner that can explicitly start, stop, suspend, resume, and react to network changes.

M10 must define and test at least:

- app foreground/background transition behavior;
- explicit runtime shutdown;
- network loss and restoration;
- reconnect without stale session authority;
- repeated lifecycle events without duplicate runtimes or unbounded tasks.

Production background operation requiring Android foreground-service behavior is enabled only when the product behavior actually requires it and the corresponding user-visible/OS requirements are implemented.

## 10. Identity and Key Storage

The existing Cross-Lab identity and trust semantics remain authoritative. Mobile UI code never receives raw long-term private keys.

When M10 introduces persistent Android production identity material, storage must be mediated through an Android platform adapter using Android Keystore/StrongBox where appropriate. Temporary development provisioning must be isolated from release behavior and must never become an implicit fallback when secure storage fails.

The first slice must not add a second mobile-specific identity model.

## 11. Networking and Session Semantics

The first real-device slice uses the existing Quinn local/LAN transport and the existing Cross-Lab logical session/authentication model.

The UI displays domain state but does not redefine it. In particular, connectivity, trust, risk, lifecycle, and recovery state remain orthogonal concepts. A single visual summary may compose them, but application state must not collapse them into one convenience enum that loses security meaning.

Reconnect creates a fresh authenticated session according to the existing M7/M8 behavior. Trust revocation must terminate or prevent invalid sessions according to the current core rules.

Iroh is not promoted in this slice. ADR-0009's Android lifecycle, real-device transition, secure-key-storage, dependency-review, and transport-key obligations remain production gates for a later consuming slice.

## 12. Repository Shape

M10 adds only directories with an immediate consumer. The expected first implementation may introduce:

```text
apps/desktop/
apps/android/
crates/mobile-ffi/        # only if the approved implementation plan confirms this boundary
design/
  themes/
  schema/
```

The exact Rust crate name for the mobile façade is chosen during implementation planning after checking current workspace naming and dependency direction. Empty future `apps/ios`, platform, service, plugin, or design-system trees are not created merely to match the long-term architecture diagram.

The neutral theme schema belongs under `design/`, not inside a desktop-only GPUI module, because Android is an immediate M10 consumer.

## 13. Research and Reuse Findings

M10 follows the repository's research policy: reuse/wrap maintained libraries where they fit; study architectural patterns where direct reuse would leak unsuitable APIs.

Relevant findings reviewed for this design:

- **GPUI Kit:** current upstream exposes semantic theme tokens for colors, radii, spacing, typography, and shadows, plus a theme registry and built-in Ayu themes. Cross-Lab should reuse/wrap compatible GPUI primitives but keep the canonical cross-platform design contract independent of GPUI Kit-specific types.
- **UniFFI:** current upstream supports proc-macro-defined Rust APIs and Kotlin/Swift binding generation. It is appropriate for the narrow mobile façade selected by the Master Architecture; internal crates should not each become independent FFI surfaces.
- **COSMIC Connect Core:** retained as a study reference for Rust + mobile FFI cross-device boundaries. No source is adapted in M10 without exact source/license inspection.

No research repository defines Cross-Lab identity, trust, policy, session, or product UI semantics.

## 14. Alternatives Considered

### Flutter for mobile

Not selected. Flutter would share presentation code but Cross-Lab still requires substantial Kotlin/Swift platform integration for lifecycle, secure storage, networking, Bluetooth/Wi-Fi APIs, media capture, biometrics, and later deep OS capabilities. It would add a Dart/native/Rust boundary without eliminating the native platform layer.

### Rust-only mobile UI

Not selected. It would move Android/iOS framework bindings, lifecycle bridging, and accessibility/platform behavior into custom Rust integration while the desktop already has a separate GPUI renderer. The project remains Rust-dominant by keeping reusable domain/network/security logic in Rust and native platform shells thin.

### Platform-default Material/Cupertino product styling

Not selected. Cross-Lab requires a recognizable product language across platforms. Native behavior and accessibility remain platform-correct while product appearance follows Cross-Lab tokens.

### GPUI Kit theme format as the universal schema

Not selected. GPUI Kit is a desktop dependency. Making its internal schema the mobile product contract would couple Android/iOS presentation to a desktop rendering library. Cross-Lab therefore owns a renderer-neutral schema and maps it into GPUI Kit.

### Clipboard-first capability slice

Deferred until the platform/session shell is proven. Starting with clipboard would mix a new platform capability with first-time desktop UI, Android UI, FFI, lifecycle, networking, and authentication boundaries. Device status proves those foundations with less capability-specific surface.

## 15. Testing and Verification

M10 verification is layered:

- Rust unit/integration tests for any new façade/runtime adapters;
- schema/theme validation tests, including missing/invalid token failure behavior;
- desktop component/state tests where GPUI supports meaningful behavior tests;
- Android JVM/unit tests for theme mapping and lifecycle/controller logic;
- Android build/binding-generation verification;
- Linux + Android integration test over Quinn using real Cross-Lab authentication;
- disconnect/reconnect and revocation regression tests;
- bounded event/state delivery and clean shutdown tests;
- manual real-device evidence for foreground/background and network-change behavior before M10 is declared complete.

CI must continue to run repository formatting, locked dependency verification, audit, check, clippy with warnings denied, and workspace tests. Mobile-specific build checks are added when the Android project exists.

## 16. Success Criteria

M10's first implementation slice is complete when all of the following are true:

1. a Linux GPUI desktop shell builds and renders the Cross-Lab device/status surface;
2. an Android Kotlin application builds and consumes the narrow Rust façade;
3. both applications use the same versioned Cross-Lab semantic theme contract;
4. `System`, `Darkmatter`, and `Ayu Light` selection works, with System resolving by OS appearance;
5. at least representative UI primitives prove that non-color tokens such as radius, typography, spacing/density, and control metrics are actually consumed rather than merely stored;
6. a Linux and Android device establish a real authenticated Quinn local/LAN Cross-Lab session using existing security semantics;
7. both UIs surface peer identity/trust/connectivity/session state without exposing secrets;
8. disconnect/reconnect creates fresh authenticated session state;
9. revocation prevents invalid reconnect according to existing policy/session behavior;
10. Android runtime lifecycle and network-change ownership are bounded and verified on a real device;
11. no production Iroh adapter, broad mobile FFI, privileged UI authority, or web UI runtime is introduced;
12. repository and mobile verification gates pass and `CURRENT.md` records the exact next task/evidence.

Darkmatter visual acceptance additionally requires the authoritative owner-provided Darkmatter theme source to be present; values must not be inferred.

## 17. Non-Goals

M10's first slice does not include clipboard sync, file transfer UI, notification mirroring, pairing QR UX, BLE discovery, Wi-Fi Direct, remote/NAT networking promotion, background always-on behavior without a demonstrated need, iOS implementation, Windows implementation, privileged helpers, plugin runtime, or a complete design-system component catalog.

These remain later vertical slices built on the platform/session foundation.

## 18. Architecture Decision Requirement

The shared renderer-neutral theme contract changes the current baseline from a desktop-only fixed `radius 0` design statement to a cross-platform contract where sharp/radius-0 is the default and geometry/typography/density are tokenized. Because this affects a cross-platform API contract, it requires an ADR under the Master Architecture policy.

ADR-0012 is therefore proposed alongside this design. The ADR must be reviewed and accepted before the Master Architecture is changed or production implementation depends on the new cross-platform theme contract.

## 19. Implementation Gate

After owner review of this design and ADR-0012:

1. accept/revise ADR-0012 and reconcile the Master Architecture;
2. write the detailed M10 implementation plan from the actual repository APIs and verified dependency versions;
3. implement in small vertical milestones with tests first where meaningful;
4. update `CURRENT.md` at each durable checkpoint;
5. open/merge reviewed PRs only after exact-head verification and verify `main` after integration.
