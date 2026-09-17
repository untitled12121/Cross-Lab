# ADR-0012: Cross-platform design system and theme contract

**Status:** Accepted  
**Date:** 2026-09-17

## Context

Cross-Lab now enters M10, where the first Linux GPUI desktop and Android Kotlin application become real consumers of the shared product architecture.

The current Master Architecture defines a desktop visual direction with semantic OKLCH tokens, strong light/dark themes, sharp geometry, and radius `0`, while mobile architecture separately defines native Kotlin and Swift application layers over a shared Rust core.

That is insufficient for M10 because visual decisions would otherwise drift into platform-local constants. The owner requires one recognizable Cross-Lab visual language across desktop and mobile, with theme switching that controls more than color: typography, radius, spacing, density, borders, control metrics, icons, shadows, and motion must be configurable through semantic presentation tokens.

GPUI Kit provides a useful desktop implementation reference: current upstream exposes semantic color/radius/spacing/typography/shadow tokens and a runtime theme registry. Those GPUI-specific types must not become the cross-platform product contract because Android and future iOS should not depend conceptually on a desktop rendering library.

## Decision

Cross-Lab will own a **versioned, renderer-neutral semantic theme contract** used by all product UIs.

1. The canonical theme document is Cross-Lab-owned presentation data under the repository `design/` boundary, independent of GPUI, GPUI Kit, Android UI libraries, Swift UI libraries, CSS, React, or any network protocol.
2. The initial semantic contract covers appearance identity, OKLCH colors, typography, radius, spacing, density, border widths, control/list metrics, icon metrics, shadows/elevation, and motion values required by actual consuming components.
3. Desktop GPUI/GPUI Kit, Android Kotlin, and future iOS Swift each map the same semantic contract into native renderer types. UI implementation code is platform-specific; the design contract and visual identity are shared.
4. `Darkmatter` is the default dark theme and `Ayu Light` is the default light theme.
5. `System` is a selector rather than a third palette. It resolves to Darkmatter for OS dark appearance and Ayu Light for OS light appearance. Explicit theme selection overrides system appearance until changed by the user.
6. Cross-Lab's baseline remains sharp geometry with radius `0`, but radius is represented by theme tokens rather than hard-coded throughout feature code. A theme may intentionally configure another radius without changing business logic.
7. Theme choice and token values are presentation state only. They must not change device trust, risk, capability availability, authorization, network classification, session semantics, or any other security-domain decision.
8. CSS, React components, Tailwind classes, screenshots, and similar owner-provided artifacts may be used as visual source material. They are translated into native Cross-Lab components/tokens and do not introduce a browser/React runtime or web architecture into Cross-Lab.
9. Platform layout may adapt to input method, safe areas, window size, navigation conventions, and accessibility requirements while preserving Cross-Lab tokens, hierarchy, states, and visual identity. Native platform behavior does not imply stock Material or Cupertino product styling.
10. Theme data is local presentation configuration. It is not part of the Cross-Lab peer wire protocol and is not transported through the mobile Rust FFI merely to synchronize appearance.
11. The exact Darkmatter palette must come from the owner-provided authoritative theme source. It must not be reconstructed from screenshots or guessed values. Palette/content changes that preserve the semantic schema do not require a new ADR.
12. Breaking changes to the cross-platform semantic theme schema are versioned and require compatibility handling appropriate to their consumers.

## Alternatives considered

### Platform-default styling

Rejected. Allowing GPUI desktop, Android Material defaults, and Apple Cupertino defaults to independently define product appearance would fragment Cross-Lab's visual identity and force feature surfaces to encode platform-local presentation assumptions.

### GPUI Kit theme schema as the universal contract

Rejected. GPUI Kit is a selected desktop dependency/reference. Making its types the universal contract would couple mobile presentation to desktop implementation details and violate the rule that third-party libraries do not define Cross-Lab domain/product semantics.

### CSS as the runtime canonical format

Rejected. CSS is useful owner-provided visual source material, but GPUI, Kotlin-native UI, and Swift-native UI do not share a CSS runtime. A renderer-neutral semantic document gives every native renderer the same values without introducing a browser layer.

### Shared Flutter mobile UI

Rejected for the Cross-Lab baseline. Flutter would share mobile presentation code but would not remove Kotlin/Swift platform integration for lifecycle, secure storage, networking, Bluetooth/Wi-Fi, media, biometrics, and future deep OS capabilities. It would add another runtime/language boundary without unifying the desktop GPUI renderer.

### Rust-only mobile UI

Rejected for the baseline. It would require Cross-Lab to own more Android/iOS framework, lifecycle, accessibility, and platform bridging while still retaining a distinct GPUI desktop renderer. Shared Rust remains focused on reusable domain/network/security logic.

## Security impact

The theme system has no authority-bearing role. Theme documents must never be interpreted as policy, capability, permission, trust, network-security, or privilege configuration.

Theme parsing must be bounded and fail safely to a known valid presentation profile. Untrusted theme files, if user-installable themes are added later, must not gain filesystem, network, process, plugin, or code-execution authority through theme loading.

Theme/appearance logs must not include secrets or sensitive device payloads.

## Compatibility impact

This establishes a cross-platform presentation API contract without changing Cross-Lab wire protocol, identity credentials, trust records, logical sessions, capability negotiation, transport APIs, or mobile domain FFI.

Desktop and mobile renderers may evolve independently as long as they continue to honor the compatible semantic theme schema.

The current Master Architecture statement `radius 0` becomes: radius `0` is the default Cross-Lab geometry, while radius is controlled through the semantic theme contract rather than hard-coded as a renderer invariant.

## Operational impact

Cross-Lab must validate baseline theme documents and keep theme identifiers stable enough for persisted local appearance preferences.

Desktop, Android, and future iOS builds need a deterministic way to package the same canonical theme data. Exact packaging/generation mechanics are implementation details chosen by the M10 plan, provided they do not create divergent hand-maintained copies.

The Darkmatter palette cannot be considered visually complete until the authoritative owner-provided theme source is present in the project. Ayu Light may use maintained GPUI Kit material as a reference only after the normal license/reuse checks.

## Consequences

Cross-Lab gains one visual language across native renderers without creating one cross-platform UI runtime.

Feature code consumes semantic presentation roles rather than raw colors, font sizes, radii, or spacing constants. Theme switching can therefore change color and approved geometry/typography/density metrics without rewriting feature screens.

Desktop can reuse GPUI Kit's mature theme/component infrastructure through an adapter while mobile remains native Kotlin/Swift.

React/CSS references supplied later can be reproduced closely without changing the native application architecture.

The Master Architecture is reconciled as part of the same M10 design integration checkpoint before production implementation depends on the contract.
