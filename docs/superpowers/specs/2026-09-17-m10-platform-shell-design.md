# M10 First Platform Vertical Slice Design

**Status:** Approved  
**Approved:** 2026-09-17  
**Milestone:** M10 — First Platform Vertical Slice  
**Base:** `064526f688aed0a92dd5b0ae4815f54d5b97d6f6`

## 1. Purpose

M10 moves Cross-Lab from the verified core/network foundation into the first real Linux desktop + Android platform slice without weakening the architecture proven in M1–M9.

The first slice proves one useful end-to-end behavior: a Linux desktop and Android device establish a real authenticated Cross-Lab session over the existing Quinn local/LAN transport and expose coherent device, trust, connection, and session status through native Cross-Lab user interfaces.

M10 also establishes the smallest cross-platform design-system contract required by those two applications. It does not build the full future component catalog or promote the M9 Iroh experiment into production.

## 2. Governing Architecture

- Linux/Windows/macOS desktop UI uses Rust + GPUI + GPUI Kit.
- Android uses Kotlin for UI/platform integration over one deliberately narrow shared-Rust façade.
- iOS/iPadOS remains Swift over the same conceptual narrow Rust façade when Apple mobile work begins.
- UI code does not own network protocols, cryptography, persistence internals, or privileged operations.
- Platform-specific behavior remains behind narrow adapters.
- Quinn remains the local/LAN transport for this slice.
- Iroh remains selected for remote/NAT/relay architecture but is not promoted until ADR-0009 production gates are satisfied.

## 3. First Vertical Slice

The first M10 slice is **authenticated local device connection + device status**.

A Linux desktop and Android device must be able to:

1. start their normal unprivileged Cross-Lab runtime;
2. use owner-authorized device credentials and trust state from the existing security model;
3. connect over Quinn on a local network;
4. perform the existing channel-bound Cross-Lab authentication and protocol/capability negotiation;
5. expose authenticated peer and connection/session state to UI-facing layers;
6. show coherent device status in both native UIs;
7. disconnect cleanly and reconnect through a fresh authenticated session;
8. reject reconnect after trust revocation according to existing core semantics.

Pairing/bootstrap UI is not part of this first slice. Real-device development may provision already owner-authorized credentials through a clearly development/test-only path. No fake cryptography or release-enabled trust bypass is permitted.

## 4. Cross-Platform Visual System

Cross-Lab owns one product visual language across desktop and mobile. Native frameworks remain responsible for platform behavior, lifecycle, accessibility, input, safe areas, and OS integration; Cross-Lab controls product visual identity.

The baseline direction is dense and Zed-like:

- sharp geometry;
- compact information density;
- near-flat surfaces and thin separators;
- restrained semantic status colors;
- clear hierarchy without decorative card inflation;
- strong keyboard/focus behavior on desktop;
- appropriate touch targets and accessibility on mobile;
- visible device, trust, security, transport, and connection state.

### 4.1 Theme identities

Cross-Lab defines two baseline named themes:

- `Darkmatter` — default dark theme;
- `Ayu Light` — default light theme.

`System` is a selector, not a third palette. It resolves to Darkmatter for OS dark appearance and Ayu Light for OS light appearance. Explicit selection of a named theme overrides system appearance until changed by the user.

The exact Darkmatter palette must come from the owner-provided authoritative theme/CSS source. That source is not present in the currently verified project context, so M10 must not invent values from screenshots or approximations. Architecture, schema, renderer adapters, runtime work, and Ayu Light may proceed; Darkmatter visual acceptance remains open until the source is supplied.

Ayu Light may use maintained GPUI Kit theme material as an implementation reference where license/compatibility review permits, while Cross-Lab's canonical theme data remains Cross-Lab-owned and renderer-neutral.

### 4.2 Canonical theme contract

CSS/React/Tailwind supplied by the owner is design input, not runtime architecture. Values are translated into a small versioned Cross-Lab theme document independent of GPUI, Compose, SwiftUI, CSS, or a browser runtime.

The initial semantic contract covers:

```text
identity / appearance
semantic OKLCH colors
font families / sizes / weights / line heights
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

Radius `0` remains the baseline Cross-Lab default but is a theme token rather than a hard-coded feature constant. Themes may intentionally change radius, typography, density, spacing, and related presentation without feature code branching on theme names.

Theme data is presentation state only. It must never alter trust, risk, policy, capability availability, network classification, authorization, logical-session semantics, or privilege behavior.

### 4.3 Renderer adapters

The same canonical theme document is mapped separately into native renderer types:

```text
Cross-Lab theme document
          |
   +------+-------+
   |              |
Desktop        Android
GPUI/Kit       Kotlin/Compose
   |
future Windows/macOS share desktop renderer

future iOS maps the same contract into Swift UI code
```

The design contract is shared; UI implementation code is not. Layout may adapt to input method, safe areas, window size, navigation conventions, and accessibility while preserving Cross-Lab tokens, hierarchy, states, and identity.

Theme data is local application presentation configuration. It does not travel through the Cross-Lab peer protocol or mobile core FFI merely to synchronize appearance.

## 5. UI Framework Decisions

### 5.1 Desktop

The desktop application is a Rust GPUI application using GPUI Kit for compatible primitives/integration while keeping Cross-Lab-owned components and styling under `components/ui/`.

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

Only components required by the first slice are introduced. `page.rs` composes; feature state/controller logic stays under `features/`; networking and security logic stay outside GPUI components.

### 5.2 Android

Android remains Kotlin-native. M10 selects Jetpack Compose for the first Android product shell because its state-driven model maps cleanly to the semantic theme contract while lifecycle, networking callbacks, permissions, secure storage, and later deep integrations remain in native Kotlin/Android APIs.

Compose or Material libraries are renderer/toolkit dependencies, not the Cross-Lab visual authority. Android maps Cross-Lab tokens locally and must not become visually defined by stock Material defaults.

### 5.3 Future iOS

M10 does not scaffold iOS. The theme contract and mobile façade remain suitable for a future Swift implementation, but no speculative Swift code is created.

## 6. React/CSS Reference Workflow

Owner-provided React components, CSS, Tailwind classes, screenshots, and similar references are treated as visual specifications. Cross-Lab reproduces their visible hierarchy, layout, spacing, typography, component states, and interaction intent as closely as practical in native GPUI/Kotlin/Swift components.

No React runtime, DOM, browser shell, or Tailwind-driven application architecture is introduced solely to reproduce a reference.

Reusable patterns are promoted into Cross-Lab design tokens or `components/ui/` only when they are genuinely reusable.

## 7. Runtime Boundary

The desktop UI remains separate from the normal unprivileged Cross-Lab runtime boundary. UI state observes and commands an application-facing runtime; UI code does not receive ambient privileged authority.

Because Linux desktop and Android are immediate consumers, M10 may introduce one small platform-neutral runtime/coordinator crate when implementation shows it prevents duplication of the existing simulator coordination logic. It must remain free of GPUI, Kotlin/Android, Swift, and privileged APIs.

No UI component may directly create Quinn connections, sign credentials, evaluate trust policy, or own long-running networking tasks.

## 8. Android Runtime and Mobile FFI

M10 introduces one deliberately narrow mobile façade rather than exporting `crosslab-core`, `crosslab-identity`, `crosslab-policy`, `crosslab-protocol`, or transport crates independently.

The façade owns only stable mobile-facing DTOs, commands, state/event delivery, error mapping, threading rules, and lifecycle entry points consumed by this slice. Internal domain types remain private unless a specific mobile contract requires a representation.

The conceptual surface is limited to:

```text
runtime lifecycle
local device summary
peer/device summaries
connection/session status
connect/disconnect or start/stop intent
state/event subscription
bounded error/status information
```

UniFFI is the preferred initial binding mechanism and its exact maintained compatible version is verified immediately before implementation.

## 9. Android Lifecycle Ownership

A UI composable must not own the Rust networking/session runtime. The runtime is application/service scoped behind a Kotlin lifecycle owner that can explicitly start, stop, suspend/resume, and react to network changes.

M10 tests at least:

- foreground/background transitions;
- explicit shutdown;
- network loss/restoration;
- reconnect without stale authority;
- repeated lifecycle events without duplicate runtimes or unbounded tasks.

Production foreground-service/background operation is added only when an actual product behavior requires it and the corresponding user-visible/OS requirements are implemented.

## 10. Identity, Key Storage, and Transport TLS

Existing Cross-Lab identity/trust semantics remain authoritative. Mobile UI code never receives raw long-term private keys.

When persistent Android production identity material is introduced, storage must be mediated through an Android adapter using Keystore/StrongBox where appropriate. Development provisioning must be isolated from release behavior and must never become an insecure fallback.

ADR-0008 defines the exact Quinn TLS-exporter channel binding but explicitly did not decide production certificate provisioning. M10 therefore must not add accept-all certificate verification or silently equate a TLS certificate with Cross-Lab identity. Real-device development may use explicitly provisioned test/development TLS material while production certificate lifecycle remains a separately reviewed requirement when a release consumer needs it.

## 11. Networking and Session Semantics

The first real-device slice uses the existing Quinn local/LAN transport and existing Cross-Lab logical session/authentication model.

UI displays domain state but does not redefine it. Connectivity, trust, risk, lifecycle, and recovery remain orthogonal state axes.

Reconnect creates a fresh authenticated session according to existing M7/M8 behavior. Trust revocation terminates or prevents invalid sessions according to current core rules.

Iroh is not promoted in this slice. ADR-0009's Android lifecycle, real-device transition, secure-key-storage, dependency-review, and transport-key obligations remain production gates for a later consuming slice.

## 12. Repository Shape

M10 adds only directories/crates with immediate consumers. Expected implementation boundaries are:

```text
apps/desktop/
apps/android/
crates/runtime/          # shared app coordination only if both consumers use it
crates/mobile-ffi/       # one narrow Android/iOS-facing Rust façade
design/
  themes/
  schema/
```

No empty future iOS, privileged service, plugin, or capability trees are created merely to match the long-term architecture diagram.

The neutral theme schema belongs under `design/`, not in a desktop-only GPUI module.

## 13. Research and Reuse Findings

- **GPUI Kit:** reuse/wrap compatible GPUI primitives and theme integration, but keep canonical cross-platform semantics independent of GPUI-specific types.
- **UniFFI:** use for one narrow Kotlin/Swift-facing façade; do not export internal crates independently.
- **COSMIC Connect Core:** study Rust/mobile FFI boundary patterns only; no source is adapted without exact source/license inspection.

Research projects do not define Cross-Lab identity, trust, policy, session, or product UI semantics.

## 14. Alternatives Considered

**Flutter mobile UI:** not selected; it adds Dart/native/Rust boundaries while native Kotlin/Swift integrations remain necessary for lifecycle, secure storage, networking, Bluetooth/Wi-Fi, media, biometrics, and deep platform work.

**Rust-only mobile UI:** not selected; it would make Cross-Lab own more Android/iOS framework/lifecycle/accessibility bridging while desktop already uses a distinct GPUI renderer.

**Platform-default Material/Cupertino styling:** not selected; native behavior stays platform-correct, but Cross-Lab owns the product appearance.

**GPUI Kit theme schema as universal schema:** not selected; a desktop dependency must not define the mobile product contract.

**Clipboard-first slice:** deferred; first prove the platform/session/UI/FFI/lifecycle foundation without adding capability-specific complexity.

## 15. Testing and Verification

M10 uses layered verification:

- Rust unit/integration tests for shared runtime and mobile façade;
- theme/schema parsing and mapping tests;
- desktop state/component tests where meaningful;
- Android JVM tests for theme mapping and lifecycle/controller logic;
- Android binding-generation/build verification;
- Linux + Android Quinn integration using existing Cross-Lab authentication;
- disconnect/reconnect/revocation regression tests;
- bounded event/state delivery and clean shutdown tests;
- manual real-device foreground/background and network-change evidence before M10 completion.

CI preserves locked metadata, dependency audit, fmt, check, clippy with warnings denied, and workspace tests. Mobile-specific gates are added when the Android project exists.

## 16. Success Criteria

M10 completes only when:

1. a Linux GPUI shell builds and renders the device/status surface;
2. an Android Kotlin app builds and consumes the narrow Rust façade;
3. both use the same versioned semantic theme contract;
4. System/Darkmatter/Ayu Light selection behaves per ADR-0012 once authoritative Darkmatter data exists;
5. representative primitives consume non-color tokens such as radius, typography, spacing/density, and control metrics;
6. Linux and Android establish a real authenticated Quinn local/LAN Cross-Lab session;
7. both UIs surface peer identity/trust/connectivity/session state without secrets;
8. disconnect/reconnect creates fresh authenticated session state;
9. revocation prevents invalid reconnect;
10. Android lifecycle/network ownership is bounded and verified on a real device;
11. no production Iroh adapter, broad FFI, privileged UI authority, or web runtime is introduced;
12. repository/mobile verification gates pass and `CURRENT.md` records exact evidence/next task.

Darkmatter visual acceptance additionally requires the authoritative owner-provided source; values must not be inferred.

## 17. Non-Goals

The first M10 slice does not include clipboard sync, file transfer UI, notification mirroring, pairing QR UX, BLE discovery, Wi-Fi Direct, production remote/NAT networking promotion, speculative always-on background behavior, iOS, Windows, privileged helpers, plugin runtime, or a complete component catalog.

## 18. Approved Architecture Decision

ADR-0012 was owner-approved on 2026-09-17. Master Architecture revision 2.3 incorporates the renderer-neutral theme contract, tokenized radius baseline, Darkmatter/Ayu Light/System identities, and the first Android Compose shell direction.

## 19. Implementation Gate

The implementation path is defined by `docs/superpowers/plans/2026-09-17-m10-first-platform-slice.md`.

Before production code starts, this approved design/ADR/Master/plan checkpoint must be merged through exact-head CI and post-merge `main` verification. Implementation then proceeds from a fresh branch in small test-backed milestones with `CURRENT.md` checkpointed after meaningful progress.
