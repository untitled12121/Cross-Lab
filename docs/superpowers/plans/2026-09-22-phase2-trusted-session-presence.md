# Phase 2 Trusted Session / Presence Foundation

**Status:** Trusted-session presence platform lifecycle implemented; physical LAN evidence pending  
**Date:** 2026-09-22  
**Base:** M10 product pairing merged in PR #53

## Goal

Turn durable reciprocal pairing trust into the normal Linux ↔ Android authenticated session lifecycle required by the Phase 2 MVP.

## Task 1 — Production-ready session authentication inputs — implemented

- Allow session proofs to use the existing `SigningProvider` boundary so platform-protected device keys remain non-exportable.
- Allow one endpoint to authenticate against a bounded set of durable trusted peers instead of one development-provisioned peer.
- Resolve the peer trust record only after the protected session hello identifies the presented device credential.
- Preserve fresh channel binding, nonce/proof, authority-currentness, trust/current credential checks, and fresh `SessionId` on every reconnect.

## Task 2 — Privacy-conscious local session discovery — implemented

- Add a normal-session LAN profile separate from the one-time pairing discovery profile.
- Keep discovery metadata non-authoritative and free of OwnerId/DeviceId/trust/capability data.
- Bound candidate processing, retry/backoff, and network-change handling.
- Record the compatibility/security profile in an ADR before promotion.

## Task 3 — Production Quinn session endpoint — implemented

- Reuse the existing Quinn authenticated-session implementation and ADR-0008 channel binding.
- Do not reuse development provisioning.
- Use protected TLS/QUIC only as the channel; Cross-Lab credential/trust authentication remains authoritative.
- Support clean disconnect and fresh-auth reconnect.

## Task 4 — Platform lifecycle and UI — implemented

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

## Foundation checkpoint

PR #54 establishes the security/transport foundation: platform SigningProvider-backed proofs, bounded multi-peer durable trust resolution, ADR-0017 discovery compatibility constants/role selection, production TLS 1.3 Quinn trusted-session endpoints, shared channel-only TLS verification, verified trusted-peer projection from ProductIdentityState, and reconnect coverage proving a fresh SessionId.

## Platform lifecycle checkpoint

PR #55 implements the next vertical slice: Linux Avahi and Android NsdManager normal-session discovery, the narrow `crosslab-agent` trusted-presence coordinator, durable `ProductIdentityState` + platform `SigningProvider` session startup, bounded candidate/retry handling, automatic authenticated Quinn connect/accept/reconnect, explicit Disconnect/Reconnect, and GPUI/Compose presence state. DNS-SD and transport metadata remain routing/channel state only; authenticated Cross-Lab identity and current durable trust remain authoritative. Physical Linux/Android LAN evidence remains separate from CI.
