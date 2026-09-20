# Product Add Device / QR Pairing Implementation

**Status:** Active  
**Date:** 2026-09-20  
**Base:** PR #48 identity-store foundation

## Goal

Make normal local-first device enrollment usable from the Linux desktop and Android app without development provisioning.

## Task 1 — Product invitation presentation — implemented, verification pending

- Add secure random invitation creation in shared Rust.
- Render the ADR-0014 `crosslab:pair:v1:` payload as a QR code in the Linux Devices surface.
- Keep the bootstrap string secret-bearing, short-lived, and out of logs/debug output.
- Support explicit cancel/regenerate.

## Task 2 — Android QR scanner — implemented, verification pending

- Add explicit CAMERA permission.
- Use CameraX for lifecycle-bound preview/analysis.
- Use the bundled ML Kit barcode model so scanning works without Play Services/model download.
- Limit decoding to QR codes.
- Send the scanned payload directly into shared Rust validation; do not persist the raw QR text in Compose state after validation.
- Stop camera analysis after one valid Cross-Lab bootstrap is accepted.

## Task 3 — Product pairing coordinator

- Keep pairing state in shared Rust; UI receives presentation-safe state/events only.
- Use the existing ADR-0003 transcript/HMAC flow and existing credential proof-of-possession.
- Do not invent a second pairing protocol.
- Persist successful authority/credential/trust state through the ADR-0015 platform identity store as one logical security commit.
- Pairing failures/cancellation consume the invitation according to the existing specification.

## Task 4 — Local pairing discovery/transport

- Use an explicit, bounded LAN discovery path compatible with Master Architecture section 17.
- Discovery/address information is routing metadata only and never identity/trust authority.
- Reuse Quinn for the provisional IP pairing channel where practical.
- Record any selected discovery/transport profile in an ADR before promotion.

## Verification

- Android target-aware Cargo lock graph is checked by the normal app build.
- Full Rust format/check/clippy/test gate.
- Android unit/build/default/development wiring gates.
- QR parser negative cases and secret-redaction tests.
- Scanner analyzer unit tests where logic can be isolated from camera hardware.
- Pairing coordinator integration tests covering success, cancellation, replay, malformed bootstrap, proof failure, persistence failure, and rollback/currentness failure.
- Physical camera/LAN pairing evidence is collected later by the owner and is not inferred from CI.
