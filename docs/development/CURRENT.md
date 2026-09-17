# Current Development State

This file is the durable resume guide for active Cross-Lab development. Git/code/tests are factual state; this file records the intended handoff.

## Current Phase

**Phase 1 foundation complete; transitioning into the first real Linux + Android platform slice.**

## Current Milestone

**M10 — First Platform Vertical Slice, design/planning.**

M1–M9 are complete. M9 selected Quinn for local/LAN and Iroh for remote/NAT/relay connectivity through accepted ADR-0009. Production Iroh promotion remains gated by the real-device, lifecycle, secure-key-storage, and dependency-review obligations recorded below.

## Canonical Baseline

- Canonical `main`: `918a2988eb0d79dc57797e1e9d35dc46e9a97b58` — M9 Task 10 merge.
- M9 Task 10 PR: #29 (`docs(networking): accept ADR-0009 and complete M9`).
- Final Task 10 PR head: `bec61d122bca277bb9b50aa5ab164e882817758e`.
- Task 10 exact-head PR CI: GitHub Actions `35176929570` — passed.
- Task 10 merge commit: `918a2988eb0d79dc57797e1e9d35dc46e9a97b58`.
- Task 10 post-merge `main` CI: GitHub Actions `35179839850` — passed.
- Task 8 merge remains `e14ff354f7eeb3c61eec10cef687c9af08876311` with post-merge CI `35124835955` — passed.
- M9 evidence report: `docs/research/M9-networking-evidence.md`.
- Accepted remote-networking decision: `docs/adr/ADR-0009-remote-networking.md`.

## Completed M9 Decision

ADR-0009 selects **Quinn local/LAN + Iroh remote/NAT/relay** while preserving the existing Cross-Lab logical-session and security boundaries.

The accepted invariants are:

- Cross-Lab identity, trust, policy, session, capability, operation, and stream authority remain Cross-Lab domain state;
- Iroh `EndpointId`, transport keys, relay URLs/tokens, paths, and addresses remain transport/routing state only;
- Iroh reuses ADR-0008 `quic-tls-exporter-v1` exactly after a full handshake;
- no 0-RTT or early-data path carries Cross-Lab authority;
- every Iroh-backed Cross-Lab session remains `NetworkClass::Remote` through relay/direct path changes;
- a live path change does not mint new authority or reset binding, `SessionId`, sequence state, or active operation grants;
- a new connection requires fresh exporter binding, authentication, `SessionId`, negotiated state, and operation authority;
- owner-selected/self-hosted relay operation is required with no mandatory Cross-Lab-operated public service or vendor account;
- direct-path availability is opportunistic and endpoint-dependent/symmetric NAT may remain relay-only;
- seamless Quinn/Iroh session migration and adaptive route scoring remain later decisions.

The M9 experiment remains isolated under `experiments/m9-networking`; accepting ADR-0009 does not create or authorize a production `transports/iroh` adapter by itself.

## Task 9 Decision

**Libp2p trigger: no. Task 9 was intentionally skipped.**

The controlled endpoint-dependent/symmetric-NAT topology did not obtain a direct path after UDP was enabled, but owner-relay fallback, authenticated control/data, and fresh reconnect semantics succeeded. The evidence did not identify an Iroh-specific failure that rust-libp2p Relay v2/DCUtR/AutoNAT would plausibly remove, so `rust-libp2p` was not added.

## M10 Handoff Constraints

M10 must build the smallest real Linux desktop + Android vertical slice on the stable Cross-Lab core/session/protocol foundation.

Architecture constraints:

- Linux desktop uses Rust + GPUI + GPUI Kit and follows the feature-first `pages/<feature>/page.rs`, `layout.rs`, `_components/`, `components/ui/`, `features/` organization.
- Android uses Kotlin for platform/UI code with one deliberately designed shared-Rust mobile façade; internal crates are not exported independently.
- Platform-specific behavior remains behind narrow adapters; UI code does not own networking, crypto, persistence, or privileged operations.
- Quinn remains the proven local/LAN baseline for the first real-device slice.
- Do not promote Iroh into production merely because ADR-0009 selected it architecturally.

Before production Iroh use, M10 or the consuming milestone must:

- re-evaluate the Iroh dependency tree, including `paste 1.0.15` / `RUSTSEC-2024-0436`;
- validate Android background/network lifecycle and Kotlin/Rust integration on real devices;
- validate real-device network transitions, suspend/resume, sleep/wake, and reconnect behavior;
- define storage, rotation, and privacy rules for any persistent Iroh transport key;
- use Android Keystore/StrongBox or another appropriate platform key store for production key material;
- retain owner-controlled relay configuration and no mandatory public infrastructure.

## Accepted Foundation State

- ADR-0010: authoritative active-root/delegated-role currentness and fail-closed session invalidation.
- ADR-0011: bounded `RequestId` retry history with `(SessionId, message_seq)` as the exact-envelope replay/order boundary.
- `OwnerAuthorityState` remains authoritative for owner/delegated-role currentness.
- Root or Device Signing replacement invalidates ordinary sessions and session-scoped authority; Administrative/Recovery-only rotation does not.

## Deferred / External Risks

- Durable rollback-resistant persistence for `OwnerAuthorityState` and accepted authority epochs.
- Real Android/iOS lifecycle, background-networking, entitlement, firewall, and secure-keystore evidence.
- Windows/macOS platform networking and firewall evidence.
- Production persistence/rotation/privacy policy for any stable Iroh transport key.
- `paste 1.0.15` / `RUSTSEC-2024-0436` in the isolated Iroh dependency graph.
- Explicit authorization/subscription design for future authority-sensitive system-event families.
- `main` branch protection and other repository-administration controls.

## Exact Next Task

Design **M10 — First Platform Vertical Slice** before implementation:

1. inspect the current Cross-Lab platform-facing seams and relevant GPUI Kit / UniFFI / cross-device research material;
2. select the smallest useful Linux + Android end-to-end slice and its exact success criteria;
3. define desktop, Android, shared-Rust façade, transport, lifecycle, storage, and test boundaries;
4. review and approve the M10 design;
5. write the M10 design spec and implementation plan;
6. create a fresh M10 feature branch from verified `main` and implement in small verified milestones.

## Resume Procedure

1. start from canonical `main` and verify this M9 completion checkpoint plus the latest `main` CI;
2. preserve ADR-0009 and the explicit `Libp2p trigger: no` result;
3. do not auto-promote the M9 Iroh experiment or add `rust-libp2p`;
4. finish the M10 design/approval gate before implementation;
5. after approval, create the M10 feature branch, implement the documented first vertical slice, verify, commit, push, and update this file with the next exact task.
