# Development Roadmap

This file is a concise navigation view. `docs/architecture/MASTER-ARCHITECTURE.md` remains authoritative.

## Phase 0 — Architecture and Security Specification

**Status: Complete**

P0.1-P0.9 are complete. Closeout record: `docs/plans/phase-0/PHASE-0-CLOSEOUT.md`.

## Phase 1 — Core Simulator and First Platform Slice

**Status: M1-M10 implemented; M10 physical evidence pending**

- M1 — Repository Foundation — **Complete**
- M2 — Crypto + Identity — **Complete**
- M3 — Trust + Policy — **Complete**
- M4 — Protocol — **Complete**
- M5 — Pairing + Authenticated Logical Session Simulator — **Complete**
- M6 — Authorized Data Streams — **Complete**
- M7 — Failure and Security Lifecycle — **Complete**
- M8 — Quinn Transport — **Complete**
- M9 — Remote Networking Evaluation ADR — **Complete**
- M10 — First Linux Desktop + Android Vertical Slice — **Implemented; physical evidence pending**

M10's host/runtime/UI foundations and normal product pairing lifecycle are implemented, including ADR-0016 bounded DNS-SD discovery, provisional Quinn pairing, reciprocal durable trust, and native Linux/Android lifecycle UI. Physical Linux + Android evidence remains a separate owner-hardware gate.

## Phase 2 — Linux + Android MVP

**Status: Active after M10 implementation**

Target:

- discovery and secure pairing;
- automatic trusted-device connection/reconnect;
- device status/presence;
- permissions/capability controls;
- clipboard;
- resumable file transfer;
- notifications;
- audit/history;
- revocation/device removal;
- polished native Linux GPUI and Android Compose UX.

**Integrated software:** Phase 2 contains normal Linux + Android QR/LAN pairing and trust, authenticated Quinn sessions/reconnect, trusted-device inventory, per-peer default-deny authorization, bidirectional clipboard, resumable single-file transfer, permission-gated foreground notifications, protected metadata-only audit and owner-signed terminal revocation. Accepted ADR-0020–0023 and Master Architecture v2.11 govern these boundaries. Later hardening through [PR #119](https://github.com/untitled12121/Cross-Lab/pull/119) includes resource limits, cancellation, native listener recovery, stale transfer stream handling, and strict release CI routing. See [CURRENT.md](CURRENT.md) for the exact integrated commit and full software verification result.

**Milestone sequence:** finish and verify the software product first; then the owner separately validates the normal product on real devices. Device observations do not substitute for completed code or CI, and host CI does not assert device behavior. Cross-Lab 0.1 is not published and Phase 3 remains deferred until the approved release criteria close.

Cross-Lab 0.1 succeeds when two real Linux/Android devices can perform the required MVP operations without mandatory cloud infrastructure.

## Phase 3 — Adaptive Networking

**Status: Not started**

Starts only after Phase 2 is complete. Scope remains Ethernet/LAN optimization, BLE discovery, Wi-Fi Direct/platform peer Wi-Fi, USB, Internet P2P/NAT traversal, owner relay, route scoring, and controlled route switching.

## Later Phases

- Phase 4 — Windows
- Phase 5 — Realtime Media and Peripherals
- Phase 6 — Display and Remote Control
- Phase 7 — Owner-Hosted Infrastructure
- Phase 8 — Cross-Device Authentication
- Phase 9 — Recovery Implementation
- Phase 10 — Apple Mobile Completion
- Phase 11 — Advanced Platform Integration
- Phase 12 — Distributed Compute
- Phase 13 — SDK and Ecosystem

Later-phase dependencies and abstractions are not introduced early without a concrete requirement and, where architecture changes, an ADR.
