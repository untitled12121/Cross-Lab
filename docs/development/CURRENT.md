# Current Development State

This file is the durable resume guide for active Cross-Lab development. Git/code/tests are factual state; this file records the intended handoff.

## Current Phase

**M10 first real Linux + Android platform slice is in implementation.**

M1–M9 are complete. M9 selected Quinn for local/LAN and Iroh for remote/NAT/relay through accepted ADR-0009. M10 design and ADR-0012 were owner-approved on 2026-09-17.

## Canonical Baseline

- Verified canonical `main`: `0c3dbe49a9e9ddec73d5397b8975dc50eab909a8` (`feat(runtime): extract shared application runtime`).
- M10 Task 3 PR: #32 (`feat(runtime): extract shared application runtime`).
- Task 3 exact-head CI: GitHub Actions `35203673921` — passed audit, fmt, check, clippy, and tests on `27deb4f01329708b64e3776d0fd4b1f1e629a011`.
- Task 3 merge commit: `0c3dbe49a9e9ddec73d5397b8975dc50eab909a8`.
- Post-merge `main` CI: GitHub Actions `35205687239` — passed audit, fmt, check, clippy, and tests.
- Active implementation plan: `docs/superpowers/plans/2026-09-17-m10-first-platform-slice.md`.
- Approved design: `docs/superpowers/specs/2026-09-17-m10-platform-shell-design.md`.
- Master Architecture revision 2.3 and accepted ADR-0012 remain the M10 design baseline.

## Completed M10 Runtime Checkpoint

Task 3 is integrated and verified:

- `crates/runtime` owns the reusable platform-neutral session/dispatcher/transport coordination previously duplicated in `apps/sim`.
- runtime snapshots expose typed, presentation-safe summaries and hide inactive `SessionId` plus credential/key/channel-binding/payload material;
- transport loss, peer revocation, owner-root replacement, and Device Signing authority replacement fail closed;
- capability-event subscriptions are bounded by the existing dispatcher state capacity;
- `apps/sim` consumes the shared runtime rather than retaining a second coordination implementation.

## Active Task

**M10 Task 4 — Productize Quinn endpoint/bootstrap mechanics without changing identity semantics.**

- Active branch: `m10-task4-quinn-bootstrap`.
- Branch base: verified `main` `0c3dbe49a9e9ddec73d5397b8975dc50eab909a8`.
- Keep Quinn/rustls concrete types inside `transports/quic` where practical.
- Preserve ADR-0008 `quic-tls-exporter-v1` exactly and derive it only after the full QUIC/TLS handshake.
- TLS certificate identity is transport protection, never Cross-Lab `DeviceId`, trust, or permission.
- No 0-RTT authority, accept-all certificate verifier, or implicit self-signed production fallback.
- Development/test TLS material must be explicitly supplied by the caller.
- A usable `QuicTransportConnection`/runtime path must not be exposed before Cross-Lab session authentication succeeds.
- Reconnect always creates a fresh binding, authentication exchange, and `SessionId`.

The uploaded Quinn research archive could not be reopened in the project container because the container transport repeatedly timed out. Do not keep retrying that path. ADR-0008 already records the vetted uploaded-Quinn findings, and the current Cross-Lab Quinn adapter/session-auth code is the implementation reference for Task 4.

## First M10 Slice

The first slice remains **authenticated local device connection + device status** between Linux desktop and Android:

- Linux desktop: Rust + GPUI + GPUI Kit with feature-first page/layout/_components/components-ui/features organization.
- Android: Kotlin + Jetpack Compose over one narrow shared-Rust mobile façade.
- Shared coordination: `crosslab-runtime`.
- Mobile FFI: one narrow UniFFI façade; internal crates are not exported independently.
- Transport: Quinn local/LAN using existing Cross-Lab channel-bound authentication/session semantics.
- Scope: authenticated connection, safe device/trust/connectivity/session status, clean disconnect/fresh reconnect, revocation, and bounded lifecycle ownership.
- Pairing/bootstrap UI, clipboard, file transfer, BLE/Wi-Fi Direct, iOS, Windows, and production Iroh promotion remain outside this first slice.

## Security / Lifecycle Constraints

- UI code does not own networking, cryptography, persistence internals, or privileged operations.
- Status snapshots/FFI DTOs never expose private keys, credentials, authentication secrets, channel-binding bytes, or sensitive payloads.
- ADR-0008 leaves production Quinn certificate provisioning/pinning lifecycle undecided; Task 4 productizes explicit transport-local provisioning without silently resolving that product decision.
- Persistent Android production identity material still requires a reviewed secure-storage adapter such as Android Keystore/StrongBox.
- Do not promote Iroh in M10; preserve ADR-0009 and `Libp2p trigger: no`.
- Do not invent the missing canonical Darkmatter palette.

## Deferred / External Risks

- Durable rollback-resistant persistence for `OwnerAuthorityState` and accepted authority epochs.
- Real Android/iOS lifecycle/background-networking/secure-keystore evidence beyond the current slice.
- Windows/macOS platform networking and firewall evidence.
- Production Quinn certificate issuance/pinning lifecycle.
- Production persistence/rotation/privacy policy for stable Iroh transport keys.
- `paste 1.0.15` / `RUSTSEC-2024-0436` in the isolated Iroh dependency graph.
- `main` branch protection and other repository-administration controls.

## Exact Next Task

Execute Task 4 test-first:

1. add the public endpoint/session API regression proving authenticated client/server success and proving no usable transport/runtime path exists before session authentication;
2. observe the expected RED failure against the absent product API;
3. extract the existing proven Quinn bootstrap mechanics into transport-local `endpoint.rs` and `session.rs` without changing identity/session semantics;
4. preserve bounded bootstrap records, exact ADR-0008 exporter binding, fresh nonces/proofs, trust/current-authority validation, explicit TLS provisioning, and fail-closed timeout/cancellation;
5. migrate/extend negative tests for wrong binding, replay, untrusted/revoked peers, malformed/oversized bootstrap data, timeout/cancellation, and fresh reconnect;
6. run exact-head CI, checkpoint this file, merge only the verified head, and verify post-merge `main` before Task 5.

## Resume Procedure

1. verify `main`, `m10-task4-quinn-bootstrap`, recent commits, PR state, and CI before editing;
2. read Master Architecture revision 2.3, ADR-0008, ADR-0009, ADR-0012, `SESSION-TRANSPORT.md`, and the active M10 implementation plan;
3. inspect current `transports/quic` code before modifying it and reuse the existing session-auth/record/channel-binding mechanics;
4. preserve Cross-Lab identity independence from transport identity and keep all authority fail-closed;
5. verify, commit/push, and checkpoint this file after each meaningful milestone.
