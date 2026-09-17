# Current Development State

This file is the durable resume guide for active Cross-Lab development. Git/code/tests are factual state; this file records the intended handoff.

## Current Phase

**Phase 1 — Core Simulator / Real Transport Transition**

## Current Milestone

**M9 — Remote Networking ADR, Task 10 integration. ADR-0009 has owner approval and is being reconciled as Accepted on `m9-task10-adr-decision`. Task 8 is merged and post-merge verified; Task 9 is intentionally skipped because the evidence report records `Libp2p trigger: no`.**

M1–M8 are complete. The accepted M9 direction is Quinn for local/LAN and Iroh for remote/NAT/relay connectivity, with production Iroh promotion deferred to a consuming platform milestone until the recorded real-device, lifecycle, secure-key-storage, and dependency-review obligations are satisfied.

## Canonical Baseline

- Canonical `main` before Task 10 integration: `e14ff354f7eeb3c61eec10cef687c9af08876311`.
- Task 8 PR: #28 (`test(networking): complete M9 Task 8 evidence`).
- Final Task 8 PR head: `57568256d6df83e6b52be90742f94aa35821fd10`.
- Task 8 exact-head PR CI: GitHub Actions `35124216220` — passed.
- Task 8 merge commit: `e14ff354f7eeb3c61eec10cef687c9af08876311`.
- Task 8 post-merge `main` CI: GitHub Actions `35124835955` — passed.
- Task 8 successful implementation/evidence head: `99b168101e9fc31bdfbcea110bdb95d26978dac4`.
- Exact successful evidence run: GitHub Actions `35107158922`, job `104831250278`.
- Evidence report commit: `4986b8064bdf47ad6ed52eb1907be11d82a0129f`.
- Evidence report: `docs/research/M9-networking-evidence.md`.
- Task 10 proposal checkpoint: `6dccc6db5d01972ceccac6af242ed5c4dc64b043`.
- Active Task 10 branch: `m9-task10-adr-decision`.

## Accepted M9 Networking Decision

ADR-0009 selects **Quinn local/LAN + Iroh remote/NAT/relay** while preserving the existing Cross-Lab logical-session and security boundaries.

The accepted decision keeps these invariants:

- Cross-Lab identity, trust, policy, session, capability, operation, and stream authority remain Cross-Lab domain state;
- Iroh `EndpointId`, Iroh transport keys, relay URLs/tokens, paths, and addresses remain transport/routing state only;
- Iroh reuses ADR-0008 `quic-tls-exporter-v1` exactly after a full handshake;
- no Iroh/libp2p 0-RTT or early-data path carries Cross-Lab authority;
- every Iroh-backed Cross-Lab session remains `NetworkClass::Remote` through relay/direct path changes;
- a live path change does not mint new authority or reset binding, `SessionId`, sequence state, or active operation grants;
- a new connection requires fresh exporter binding, authentication, `SessionId`, negotiated state, and operation authority;
- owner-selected/self-hosted relay operation is required and no Cross-Lab-operated public service or mandatory vendor account is introduced;
- direct-path availability is opportunistic; endpoint-dependent/symmetric NAT may remain relay-only;
- seamless Quinn/Iroh session migration and adaptive route scoring remain later decisions.

The M9 experiment remains isolated under `experiments/m9-networking`; acceptance of ADR-0009 does not itself create or authorize a production `transports/iroh` adapter.

## Task 9 Decision

**Libp2p trigger: no. Task 9 is skipped.**

The controlled endpoint-dependent/symmetric-NAT topology did not obtain a direct path after UDP was enabled, but owner-relay fallback, authenticated control/data, and fresh reconnect semantics succeeded. The evidence did not identify an Iroh-specific failure that rust-libp2p Relay v2/DCUtR/AutoNAT would plausibly remove, so adding `rust-libp2p` would add scope and maintenance cost without addressing the observed limitation.

Do not add `rust-libp2p` for M9 unless a future approved evidence change explicitly reverses this decision.

## Production-Promotion Obligations

Before Iroh becomes a production cross-device remote transport, the consuming milestone must close or explicitly carry these obligations:

- re-evaluate the Iroh dependency tree, including `paste 1.0.15` / `RUSTSEC-2024-0436`;
- validate Android background/network lifecycle and Kotlin/Rust integration on real devices;
- validate real-device network transitions, suspend/resume, sleep/wake, and reconnect behavior;
- define production storage, rotation, and privacy policy for any persistent Iroh transport key;
- use Android Keystore/StrongBox or other appropriate platform key storage for production key material;
- retain owner-controlled relay configuration and no mandatory public infrastructure;
- carry equivalent lifecycle/firewall/entitlement/key-storage validation when Windows, macOS, and iOS consume the remote transport.

These are production-promotion gates, not reasons to reopen the M9 architecture selection.

## Accepted Foundation State

Accepted foundation artifacts remain:

- `docs/adr/ADR-0010-authoritative-owner-authority-currentness.md`
- `docs/adr/ADR-0011-bounded-request-replay-semantics.md`
- `docs/superpowers/specs/2026-09-14-foundation-currentness-replay-design.md`
- `docs/superpowers/plans/2026-09-14-foundation-authority-replay-remediation.md`

Identity-owned `OwnerAuthorityState` remains authoritative for owner/delegated-role currentness. Root or Device Signing replacement invalidates ordinary sessions and session-scoped authority; Administrative/Recovery-only rotation does not. `(SessionId, message_seq)` remains exact-envelope replay/order protection; `RequestId` remains bounded retry/correlation history.

## Deferred / External Risks

- Durable rollback-resistant persistence for `OwnerAuthorityState` and accepted authority epochs.
- Real Android/iOS lifecycle, background-networking, entitlement, firewall, and secure-keystore evidence.
- Windows/macOS platform networking and firewall evidence.
- Production persistence/rotation/privacy policy for any stable Iroh transport key.
- `paste 1.0.15` / `RUSTSEC-2024-0436` in the isolated Iroh dependency graph.
- Explicit authorization/subscription design for future authority-sensitive system-event families.
- `main` branch protection and other repository-administration controls.

## Exact Next Task

Finish **M9 Task 10 integration**:

1. reconcile ADR-0009, the ADR index, Master Architecture, M9 design, and this file with the accepted decision;
2. inspect the exact Task 10 branch diff;
3. open the Task 10 PR and require exact-head CI to pass the repository gate;
4. merge with an expected-head guard;
5. require post-merge `main` CI to pass;
6. checkpoint this file on the canonical `main` integration state if the final merge/run IDs need to be recorded;
7. proceed directly to **M10 — First Platform Vertical Slice**, beginning Linux desktop + Android integration without prematurely promoting unvalidated remote networking into production.

## M10 Handoff Constraints

M10 should start from the stable Cross-Lab core/session/protocol foundation and build the smallest real Linux + Android vertical slice. It must keep platform code behind narrow adapters and a deliberately designed mobile FFI façade rather than exporting internal crates wholesale.

The first M10 planning pass must explicitly account for the Android obligations inherited from ADR-0009: lifecycle ownership, background networking, real-device network changes, secure key storage, and the production Iroh dependency review. Quinn remains available as the proven local/LAN baseline while those remote-transport obligations are validated.

## Resume Procedure

1. verify branch `m9-task10-adr-decision` until Task 10 is integrated, then resume from canonical `main`;
2. preserve Task 8 evidence head/run and the explicit `Libp2p trigger: no` result;
3. preserve ADR-0009 as the accepted remote-networking decision;
4. do not auto-promote experiment code or add `rust-libp2p`;
5. verify exact-head PR CI and post-merge `main` before declaring M9 complete;
6. after the final M9 integration checkpoint, create a fresh M10 feature branch from verified `main` and begin the Linux + Android vertical-slice plan.
