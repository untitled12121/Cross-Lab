# M10 Owner and Device Management Usability Extension

**Status:** Approved by owner
**Approved:** 2026-09-20
**Milestone:** M10 usability extension
**Base:** `05cd8498fadb7ff166709c6470c412388414c2e5`

## Purpose

The first M10 platform slice proved authenticated Linux + Android runtime/session wiring, but the product shells expose too little of the already-implemented Cross-Lab owner/device model to be useful on real hardware.

This extension adds the smallest management surface needed to operate the current development slice without changing Cross-Lab identity, trust, pairing, transport, privilege, or recovery architecture.

## Architecture constraints

- Cross-Lab still has **no mandatory cloud account**. The owner trust domain remains the account-equivalent identity boundary.
- Owner/device identifiers shown in UI are presentation-only summaries; they grant no authority.
- UI never receives private keys, credentials, pairing secrets, channel-binding bytes, or raw transport objects.
- Disconnect and revoke actions route through the runtime/trust boundaries; UI does not mutate trust state directly.
- Development provisioning remains non-default and is not promoted into production identity storage.
- Production owner persistence, Android Keystore-backed keys, production Quinn certificate lifecycle, and durable trusted-device storage remain separate reviewed work.
- Pairing protocol semantics from P0.4/ADR-0003 are unchanged. A product pairing UX may wrap them later; this extension does not invent a second pairing model.

## Research notes

KDE Connect was reviewed for device-management UX patterns only: it keeps device state and pairing state outside individual feature/plugin UIs. COSMIC Connect Core was reviewed for FFI/event-boundary patterns only. Neither project defines Cross-Lab identity, trust, session, or authority semantics.

## Task 1 — Safe owner/device presentation

- Extend `crosslab-runtime::RuntimeStatus` with owner identity already present in the authenticated session context.
- Keep owner/local/peer identifiers presentation-safe and abbreviated in desktop UI.
- Extend the mobile snapshot with owner/local identity summaries through the existing narrow UniFFI facade.
- Add tests proving the new status fields do not expose sensitive material.

## Task 2 — Desktop control-center management

- Replace the one-screen shell with a small control-center shell containing **Devices** and **Owner** sections.
- Devices shows the current authenticated peer plus local device/owner summaries.
- Add runtime-backed **Disconnect** and development-only **Revoke** actions for the currently authenticated peer.
- Revoke must use the existing signed trust-transition path; disconnect must close the runtime session without shutting down the application.
- Empty states must explain how to connect without implying a cloud account exists.

## Task 3 — Android management parity

- Surface owner/local-device summaries from the shared mobile snapshot.
- Add a small Devices/Owner navigation surface.
- Add an explicit disconnect action for the current development connection while keeping the application runtime alive.
- Keep lifecycle/background handling separate from user-requested disconnect.

## Task 4 — Verification and checkpoint

- Rust full gate.
- Android JVM tests and debug/development assemblies.
- Linux desktop default/development builds.
- Confirm no new secret-bearing fields enter FFI/UI/debug output.
- Update `CURRENT.md` with exact head/CI and remaining physical evidence.
- Do not call M10 complete until the existing physical lifecycle/security evidence is collected.

## Deferred after this extension

A normal **Add device / QR pairing** product flow remains the next product-management slice. It should reuse the existing PairingInvitation/PairingInviterFlow/PairingJoinerFlow state machines, but it also needs a reviewed persistence/key-storage path before it can replace development provisioning on real devices.
