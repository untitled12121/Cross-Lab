# Product LAN Pairing v1 Implementation Plan

**Status:** Active  
**Date:** 2026-09-21  
**Base:** PR #50 product pairing coordinator

## Task 1 — Protocol and provisional Quinn channel

- Accept ADR-0016.
- Add a versioned product-pairing wire envelope for authority/credential/trust/persistence messages.
- Preserve existing ADR-0003 hello/confirmation/proof structures.
- Add a one-connection, one-bidirectional-stream provisional Quinn channel with TLS 1.3, ALPN `crosslab-pairing-v1`, bounded records, no 0-RTT, explicit timeout/cancel, and ephemeral inviter TLS material.
- Add deterministic integration tests that drive the existing shared inviter/joiner coordinator over the channel.

## Task 2 — Linux DNS-SD advertisement

- Advertise `_crosslab-pair._udp.local.` only while a pending invitation/listener exists.
- Derive the instance name from PairingId only.
- Publish only `v=1` plus the SRV port.
- Stop advertisement on cancel/expiry/success/drop.

## Task 3 — Android bounded discovery

- After a valid QR decode, use Android `NsdManager` to search only for the pairing service.
- Resolve only the exact PairingId-derived instance.
- Hold/release a multicast lock only during bounded discovery where required.
- Stop on success, timeout, cancellation, lifecycle stop, or error.
- Do not retain raw QR/bootstrap text in Compose state.

## Task 4 — Product app integration

- Linux Add Device starts invitation + provisional server + DNS-SD advertisement.
- Android scan starts bounded discovery then the Rust pairing client.
- Wire persistence callbacks through the existing Linux/Android identity-store adapters.
- Surface presentation-safe pairing states/errors only.
- Successful UI state occurs only after local durable persistence and final protocol acknowledgement.

## Verification

- Full Rust fmt/check/clippy/test.
- Protocol golden/negative/bounds tests.
- Provisional QUIC timeout, ALPN, second-client, malformed/out-of-order, cancellation tests.
- Android unit/build/default/development gates.
- Discovery lifecycle unit tests where hardware-independent.
- Physical Linux ↔ Android camera/mDNS/QUIC pairing remains owner evidence and is not inferred from CI.
