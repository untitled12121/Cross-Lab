# Current Development State

This file is the durable resume guide for active Cross-Lab development. Git/code/tests are factual state; this file records the intended handoff.

## Current Phase

**M10 first real Linux + Android platform slice is in implementation.**

M1–M9 are complete. M9 selected Quinn for local/LAN and Iroh for remote/NAT/relay through accepted ADR-0009. M10 design and ADR-0012 were owner-approved on 2026-09-17.

## Canonical Baseline

- Verified canonical `main` before Task 4: `0c3dbe49a9e9ddec73d5397b8975dc50eab909a8` (`feat(runtime): extract shared application runtime`).
- M10 Task 3 PR: #32 (`feat(runtime): extract shared application runtime`).
- Task 3 exact-head CI: GitHub Actions `35203673921` — passed audit, fmt, check, clippy, and tests on `27deb4f01329708b64e3776d0fd4b1f1e629a011`.
- Task 3 merge commit: `0c3dbe49a9e9ddec73d5397b8975dc50eab909a8`.
- Post-merge Task 3 `main` CI: GitHub Actions `35205687239` — passed audit, fmt, check, clippy, and tests.
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

## Task 4 Verification Checkpoint

**M10 Task 4 — Productize Quinn endpoint/bootstrap mechanics without changing identity semantics — is implementation-complete and awaiting merge of the verified checkpoint.**

- Task 4 branch: `m10-task4-quinn-bootstrap`.
- Task 4 PR: #33 (`feat(quic): productize authenticated endpoint bootstrap`).
- Verified implementation head: `d612490386456d276dacb872080d979ffd079262`.
- Exact-head implementation CI: GitHub Actions `35234732959` — passed dependency audit, `cargo fmt --check`, workspace check, clippy with warnings denied, and full workspace tests.
- Documentation checkpoint head: `8d96923e95c55059e70e4ca47d475cdd12398d15`; require its exact-head CI before merge.
- The earlier product implementation checkpoint `d52fafad51a7c8c969ee9285f742053e5962ce5c` independently passed the same full gate in CI `35233220854` before the final negative-test expansion.

Task 4 now provides:

- transport-local Quinn client/server endpoint wrappers with explicit externally supplied TLS certificate/trust material;
- no accept-all verifier, implicit development certificate fallback, or TLS-certificate-to-`DeviceId` authority mapping;
- bounded authenticated bootstrap that derives ADR-0008 `quic-tls-exporter-v1` only after the full QUIC/TLS handshake;
- fresh random nonces, role-separated proofs, current owner-authority/trust validation, and no 0-RTT authority;
- `AuthenticatedQuicSession` as the only public success result, with `QuicTransportConnection` constructed only after `LogicalSession` is active;
- endpoint lifetime retained by the productized transport path;
- rejection coverage for wrong exporter binding, proof replay, unknown/untrusted peers, revoked peers, malformed and oversized bootstrap data, timeout/cancellation, plus fresh reconnect binding and `SessionId` rotation;
- existing M8/M9 session-auth and transport behavior retained under the full workspace test gate.

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

Finish Task 4 integration, then execute **M10 Task 5 — application runtime actor for lifecycle-safe consumers**:

1. require exact-head CI for Task 4 checkpoint `8d96923e95c55059e70e4ca47d475cdd12398d15`, merge PR #33 with that expected head, and verify post-merge `main` CI;
2. create a fresh Task 5 feature branch from that verified `main`;
3. add tests first for single-start ownership, duplicate-start handling, deterministic stop/drop shutdown, transport/network loss, fresh authenticated reconnect, bounded command/event state, and slow-subscriber behavior;
4. implement a small actor in `crates/runtime/src/actor.rs` plus typed commands in `crates/runtime/src/command.rs` using bounded Tokio `mpsc`/`watch` primitives already present in the workspace;
5. keep mutable runtime/session state actor-owned, avoid polling/global mutable state, and preserve fail-closed session authority across lifecycle transitions;
6. run the full Rust gate, update this file, merge only the verified head, and verify post-merge `main` before Task 6.

## Resume Procedure

1. verify canonical `main`, active feature branch, PR state, exact-head CI, and recent commits before editing;
2. read Master Architecture revision 2.3, ADR-0008, ADR-0009, ADR-0012, `SESSION-TRANSPORT.md`, and the active M10 implementation plan;
3. preserve Cross-Lab identity independence from transport identity and keep all authority fail-closed;
4. do not invent Darkmatter palette values while its authoritative source is absent;
5. keep development/test credential and TLS provisioning explicit and non-release;
6. verify, commit/push, and checkpoint this file after each meaningful M10 milestone.
