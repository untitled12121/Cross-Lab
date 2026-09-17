# Current Development State

This file is the durable resume guide for active Cross-Lab development. Git/code/tests are factual state; this file records the intended handoff.

## Current Phase

**Phase 1 foundation complete; M10 first real Linux + Android platform slice is in design review.**

## Current Milestone

**M10 — First Platform Vertical Slice, design/ADR review.**

M1–M9 are complete. M9 selected Quinn for local/LAN and Iroh for remote/NAT/relay connectivity through accepted ADR-0009. Production Iroh promotion remains gated by the real-device, lifecycle, secure-key-storage, and dependency-review obligations recorded below.

## Canonical Baseline

- Verified canonical `main`: `064526f688aed0a92dd5b0ae4815f54d5b97d6f6` (`docs: checkpoint M9 completion and M10 handoff`).
- Exact `main` CI for that checkpoint: GitHub Actions `35180292718` — passed.
- M9 Task 10 PR: #29 (`docs(networking): accept ADR-0009 and complete M9`).
- Final Task 10 PR head: `bec61d122bca277bb9b50aa5ab164e882817758e`.
- Task 10 exact-head PR CI: GitHub Actions `35176929570` — passed.
- Task 10 merge commit: `918a2988eb0d79dc57797e1e9d35dc46e9a97b58`.
- Task 10 post-merge `main` CI: GitHub Actions `35179839850` — passed.
- M9 evidence report: `docs/research/M9-networking-evidence.md`.
- Accepted remote-networking decision: `docs/adr/ADR-0009-remote-networking.md`.

## Active M10 Design Checkpoint

- Active branch: `m10-platform-shell-design`.
- Branch base: verified `main` `064526f688aed0a92dd5b0ae4815f54d5b97d6f6`.
- Design spec: `docs/superpowers/specs/2026-09-17-m10-platform-shell-design.md`.
- Proposed architecture decision: `docs/adr/ADR-0012-cross-platform-design-system.md`.
- ADR-0012 remains **Proposed** until owner review; the Master Architecture is intentionally unchanged at this checkpoint.
- No M10 production code has been added yet.

## Proposed First M10 Slice

The first slice is **authenticated local device connection + device status** between Linux desktop and Android:

- Linux desktop: Rust + GPUI + GPUI Kit.
- Android: Kotlin-native UI/platform layer over one narrow shared-Rust mobile façade, with Jetpack Compose proposed for the M10 UI renderer.
- Mobile FFI: UniFFI remains the preferred initial binding mechanism; exact compatible version must be verified before implementation.
- Transport: existing Quinn local/LAN transport and existing Cross-Lab authentication/session semantics.
- Scope: authenticated connection, device/trust/connectivity/session status, clean disconnect/reconnect, and revocation behavior.
- Pairing/bootstrap UI, clipboard, file transfer, BLE/Wi-Fi Direct, iOS, Windows, and production Iroh promotion are outside this first slice.

## Proposed Cross-Platform UI Contract

Cross-Lab owns one visual language across native renderers rather than allowing stock platform styling to define the product.

- `Darkmatter` is the baseline dark theme.
- `Ayu Light` is the baseline light theme.
- `System` follows OS appearance and resolves to one of those two themes.
- Canonical theme data is renderer-neutral and semantic, covering OKLCH colors plus typography, radius, spacing, density, borders, control/list metrics, icons, shadows, and motion.
- Radius `0` remains the default sharp geometry, but radius becomes a theme token instead of a hard-coded renderer invariant.
- Desktop GPUI/GPUI Kit and Android Kotlin map the same theme contract into native renderer types; future iOS Swift does the same.
- React/CSS/screenshots supplied by the owner are visual source material and are translated into native Cross-Lab components rather than introducing a web runtime.
- The authoritative Darkmatter source values are not currently identifiable in the project context; do not invent them. Visual acceptance requires the owner-provided source to be re-supplied.

## Completed M9 Decision

ADR-0009 selects **Quinn local/LAN + Iroh remote/NAT/relay** while preserving the existing Cross-Lab logical-session and security boundaries.

The M9 experiment remains isolated under `experiments/m9-networking`; accepting ADR-0009 does not create or authorize a production `transports/iroh` adapter by itself.

**Libp2p trigger: no. Task 9 was intentionally skipped.** The evidence did not identify an Iroh-specific failure that rust-libp2p Relay v2/DCUtR/AutoNAT would plausibly remove while owner-relay fallback and reconnect semantics succeeded.

## M10 Handoff Constraints

- Keep platform behavior behind narrow adapters; UI must not own networking, crypto, persistence, or privileged operations.
- Keep the Android Rust runtime application/service scoped rather than owned by a UI composable.
- Do not expose internal Rust crates independently through FFI.
- Do not collapse connectivity, trust, risk, lifecycle, and recovery into one convenience state that loses domain meaning.
- Do not promote Iroh into production in the first slice.
- If persistent Android production identity material is introduced, use an Android secure-storage adapter such as Keystore/StrongBox where appropriate; never silently fall back to insecure key persistence.

Before production Iroh use, the consuming milestone must still re-evaluate the Iroh dependency graph and validate real-device Android lifecycle/network transitions, secure key storage, and transport-key storage/rotation/privacy behavior.

## Deferred / External Risks

- Durable rollback-resistant persistence for `OwnerAuthorityState` and accepted authority epochs.
- Real Android/iOS lifecycle, background-networking, entitlement, firewall, and secure-keystore evidence beyond the M10 slice.
- Windows/macOS platform networking and firewall evidence.
- Production persistence/rotation/privacy policy for any stable Iroh transport key.
- `paste 1.0.15` / `RUSTSEC-2024-0436` in the isolated Iroh dependency graph.
- Explicit authorization/subscription design for future authority-sensitive system-event families.
- `main` branch protection and other repository-administration controls.

## Exact Next Task

Owner reviews the M10 design spec and Proposed ADR-0012. After approval:

1. accept/revise ADR-0012 and reconcile the Master Architecture plus ADR index;
2. invoke the implementation-planning workflow and write the detailed M10 plan from actual repository APIs and verified dependency versions;
3. implement the first Linux + Android slice in small test-backed milestones;
4. record exact CI/manual real-device evidence in this file before declaring M10 complete.

Do not start production M10 implementation before the design/ADR review gate is approved.

## Resume Procedure

1. verify `m10-platform-shell-design` against `main` `064526f688aed0a92dd5b0ae4815f54d5b97d6f6`;
2. read the M10 design spec and ADR-0012 before changing platform/UI architecture;
3. preserve ADR-0009 and `Libp2p trigger: no`;
4. do not invent Darkmatter palette values if the authoritative source is still absent;
5. after owner approval, accept the ADR/reconcile architecture, write the M10 implementation plan, then begin production code.