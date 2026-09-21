# Phase 2 Trusted Session / Presence Foundation

**Status:** Active  
**Date:** 2026-09-22  
**Base:** M10 product pairing merged in PR #53

## Goal

Turn durable reciprocal pairing trust into the normal Linux ↔ Android authenticated session lifecycle required by the Phase 2 MVP.

## Task 1 — Production-ready session authentication inputs

- Allow session proofs to use the existing `SigningProvider` boundary so platform-protected device keys remain non-exportable.
- Allow one endpoint to authenticate against a bounded set of durable trusted peers instead of one development-provisioned peer.
- Resolve the peer trust record only after the protected session hello identifies the presented device credential.
- Preserve fresh channel binding, nonce/proof, authority-currentness, trust/current credential checks, and fresh `SessionId` on every reconnect.

## Task 2 — Privacy-conscious local session discovery

- Add a normal-session LAN profile separate from the one-time pairing discovery profile.
- Keep discovery metadata non-authoritative and free of OwnerId/DeviceId/trust/capability data.
- Bound candidate processing, retry/backoff, and network-change handling.
- Record the compatibility/security profile in an ADR before promotion.

## Task 3 — Production Quinn session endpoint

- Reuse the existing Quinn authenticated-session implementation and ADR-0008 channel binding.
- Do not reuse development provisioning.
- Use protected TLS/QUIC only as the channel; Cross-Lab credential/trust authentication remains authoritative.
- Support clean disconnect and fresh-auth reconnect.

## Task 4 — Platform lifecycle and UI

- Linux and Android load durable ProductIdentityState and local signing providers.
- Automatically connect trusted peers while the app/runtime is active.
- Surface device presence/connectivity/authentication state in GPUI and Compose.
- Preserve explicit Disconnect/Reconnect controls without turning network presence into trust.

## Verification

- provider-backed session proof tests;
- multiple trusted peers / unknown peer rejection;
- fresh SessionId after reconnect;
- wrong/revoked peer rejection;
- bounded discovery/candidate tests;
- Linux + Android build/unit gates;
- physical LAN evidence remains separate from CI.
