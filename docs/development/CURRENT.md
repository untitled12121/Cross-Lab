# Current Development State

This file is the durable resume guide for active Cross-Lab development. Git/code/tests are factual state; this file records the intended handoff.

## Current Phase

**Phase 1 — Core Simulator / Real Transport Transition**

## Current Milestone

**M9 — Remote Networking ADR. Foundation authority/replay remediation is complete, merged, and post-merge verified. M9 remote-networking Task 4 is next.**

M1–M8 are complete. M9 networking research Tasks 1–3 are complete. The ADR-0010/ADR-0011 foundation remediation is complete.

## Canonical Baseline

- `main` remediation merge: `450615a7c361384cfbe68bc8991b7ca03e4482fa`.
- Merged PR: **#23 — M9 foundation: implement authority currentness and replay hardening**.
- Verified PR head: `092d34b7e7eb30e0aa5d5bfeb311db343d0e11d6`.
- Exact-head Rust CI `34974781151` passed lockfile verification, dependency audit, rustfmt, full workspace check, Clippy with `-D warnings`, and complete workspace tests.
- Exact-head Fuzz Smoke `34974781112` passed `control_frame`, `data_stream_open`, `identifiers`, `pairing_bootstrap`, and `session_auth` at 256 runs each under `nightly-2026-09-12`.
- Post-merge `main` Rust CI `34979066740` passed dependency audit, rustfmt, full workspace check, Clippy with `-D warnings`, and complete workspace tests.
- `paste 1.0.15` / `RUSTSEC-2024-0436` remains an unsuppressed maintenance warning isolated to the Iroh experiment and must be re-evaluated before production networking promotion.

## Accepted Foundation State

Accepted artifacts:

- `docs/adr/ADR-0010-authoritative-owner-authority-currentness.md`
- `docs/adr/ADR-0011-bounded-request-replay-semantics.md`
- `docs/superpowers/specs/2026-09-14-foundation-currentness-replay-design.md`
- `docs/superpowers/plans/2026-09-14-foundation-authority-replay-remediation.md`

The completed remediation establishes:

- identity-owned `OwnerAuthorityState` as the local source of truth for active owner root and delegated-role currentness;
- current Device Signing authority for credential issuance, verification, rotation, pairing, trust transitions, and session authentication;
- current Administrative authority for owner approval evidence;
- root or Device Signing replacement invalidates ordinary sessions and cancels session-scoped control/stream authority before close;
- Administrative/Recovery-only rotation does not invalidate ordinary sessions;
- `(SessionId, message_seq)` remains exact-envelope replay/order protection;
- `RequestId` remains bounded correlation/duplicate/retry history, and aged-out IDs are new authenticated attempts subject to current authorization;
- high-level stream admission derives trust/policy currentness from local `TrustRecord` and `PolicyState` rather than caller-selected revisions;
- obsolete caller-selected high-level authority/currentness APIs are removed from ordinary public paths;
- identity, policy, and protocol golden/wire vectors remain unchanged.

No Master Architecture revision, protobuf schema change, canonical transcript change, signature-format change, or Quinn channel-binding change was introduced by this remediation.

## Foundation Remediation Status

Tasks 1–8 are complete. Security assessment and whole-project audit are reconciled with the accepted ADR-0010/ADR-0011 implementation. PR #23 merged only after exact-head Rust CI and Fuzz Smoke passed, and the resulting `main` merge commit passed the full Rust CI gate.

## M9 Networking State

- Quinn remains the verified local/LAN baseline.
- Iroh `1.2.0` remains isolated under `experiments/m9-networking` until ADR-0009 selects the remote-networking architecture.
- M9 networking research Tasks 1–3 are complete.
- ADR-0009 remains reserved for the M9 remote-networking decision.
- Remote sessions remain `NetworkClass::Remote` for their lifetime.
- Iroh identity/address/path/relay/transport metadata must never become Cross-Lab identity/trust/policy authority.
- No 0-RTT authority.
- Route changes must not silently mutate channel binding, `SessionId`, sequence state, policy classification, or operation authority.
- A new transport connection requires fresh Cross-Lab authentication/authorization state.
- Security authority remains local and fail-closed.

## Deferred / External Risks

- Durable rollback-resistant persistence for `OwnerAuthorityState` and accepted authority epochs.
- Platform key-store / secure-enclave integration and real mobile lifecycle evidence.
- Explicit authorization/subscription design for future authority-sensitive system-event families.
- `main` branch protection and other repository-administration controls.
- Windows/macOS/mobile CI matrices as their platform slices land.
- `paste 1.0.15` / `RUSTSEC-2024-0436` in the isolated Iroh experiment, to re-evaluate before ADR-0009 production promotion.
- Remote NAT/relay/path/measurement evidence required before ADR-0009 can select a production remote-connectivity architecture.

## Exact Next Task

Resume **M9 remote-networking Task 4** from the active M9 networking plan. Before implementation:

1. re-read the Master Architecture, the active M9 plan, ADRs, and this file;
2. inspect current `main`, recent commits, and repository state;
3. inspect the isolated Iroh experiment and relevant uploaded Iroh/Quinn/libp2p research before changing networking architecture;
4. preserve Cross-Lab identity, session, authorization, and lifecycle authority above transport metadata;
5. implement the smallest measured Task 4 slice with focused tests and exact verification evidence;
6. do not write ADR-0009 until the required remote connectivity, relay, NAT/path, lifecycle, and comparative measurement evidence is complete.

## Resume Procedure

1. verify `main` and this file are still current;
2. read the active M9 networking plan and relevant ADRs/specs;
3. inspect current experiment code and research repositories before implementation;
4. execute M9 Task 4 in small verified checkpoints;
5. update this file after meaningful progress with exact commits, verification status, unfinished work, and the next task.
