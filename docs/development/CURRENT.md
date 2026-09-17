# Current Development State

This file is the durable resume guide for active Cross-Lab development. Git/code/tests are factual state; this file records the intended handoff.

## Current Phase

**Phase 1 — Core Simulator / Real Transport Transition**

## Current Milestone

**M9 — Remote Networking ADR. Task 8 is merged and post-merge verified. Task 9 is skipped because the evidence report records `Libp2p trigger: no`. Task 10 is active on `m9-task10-adr-decision`: prepare ADR-0009 for owner review without changing the normative architecture baseline before approval.**

M1–M8 are complete. ADR-0009 is still Proposed/unaccepted. Quinn remains the selected local/LAN production baseline and Iroh remains an isolated M9 candidate until ADR-0009 is explicitly accepted.

## Canonical Baseline

- Canonical `main` head after Task 8: `e14ff354f7eeb3c61eec10cef687c9af08876311`.
- Task 8 PR: #28 (`test(networking): complete M9 Task 8 evidence`).
- Final Task 8 PR head: `57568256d6df83e6b52be90742f94aa35821fd10`.
- Exact-head PR CI: GitHub Actions `35124216220` — passed.
- Task 8 merge commit: `e14ff354f7eeb3c61eec10cef687c9af08876311`.
- Post-merge `main` CI: GitHub Actions `35124835955` — passed.
- Task 8 successful implementation/evidence head: `99b168101e9fc31bdfbcea110bdb95d26978dac4`.
- Exact successful evidence run: GitHub Actions `35107158922`, job `104831250278`.
- Evidence report commit: `4986b8064bdf47ad6ed52eb1907be11d82a0129f`.
- Evidence report: `docs/research/M9-networking-evidence.md`.
- Active Task 10 branch: `m9-task10-adr-decision`.

## M9 Evidence Decision State

The evidence currently supports a Proposed ADR selecting **Quinn local/LAN + Iroh remote/NAT/relay**, with production promotion deferred to a consuming platform milestone.

Preserved evidence/invariants:

- owner-controlled relay fallback works without mandatory public relay infrastructure;
- Iroh reproduces ADR-0008 `quic-tls-exporter-v1` after a full handshake;
- Cross-Lab authentication, bounded control/data, reconnect, revocation, cancellation, and shutdown semantics remain owned by existing Cross-Lab layers;
- every Iroh-backed Cross-Lab session remains `NetworkClass::Remote`;
- deterministic live relay-to-direct tests preserve binding, `SessionId`, control sequence state, and active operation authority;
- a new connection requires fresh binding/session/authority;
- the controlled endpoint-dependent/symmetric-NAT topology remained relay-only within the bounded direct-path observation window;
- forced reconnect after that direct-path miss refreshed binding/session state and restored authenticated control;
- `Libp2p trigger: no`; Task 9 is skipped and `rust-libp2p` must not be added for M9.

The M9 experiment remains isolated under `experiments/m9-networking`; no Iroh type has entered Cross-Lab domain/public production APIs.

## Proposed ADR Review Checkpoint

`docs/adr/ADR-0009-remote-networking.md` is being prepared as **Proposed** only.

The proposal records:

- Quinn remains local/LAN;
- Iroh is the remote/NAT/relay architecture if the ADR is accepted;
- Iroh `EndpointId`/transport keys remain transport-only;
- ADR-0008 exporter semantics are reused exactly after a full handshake;
- Iroh sessions remain `Remote` through path changes;
- owner-selected/self-hosted relay operation is required and no public Cross-Lab service is mandatory;
- direct hole punching is opportunistic, with relay fallback required for endpoint-dependent/symmetric NAT cases;
- Task 9 remains skipped;
- the M9 experiment is retained for evidence/regression, not auto-promoted into production;
- real-device/mobile lifecycle, secure key storage, and dependency review remain M10+ obligations;
- `paste 1.0.15` / `RUSTSEC-2024-0436` must be re-evaluated before production Iroh promotion.

No Master Architecture status, ADR index status, or M9 design completion state should change until the owner reviews and approves ADR-0009.

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

Finish the **Task 10 Proposed-ADR review checkpoint**:

1. verify the Proposed `docs/adr/ADR-0009-remote-networking.md` against the M9 evidence and Task 10 decision rule;
2. keep ADR-0009 `Proposed` and make no normative Master Architecture status change before explicit owner approval;
3. obtain owner review/approval of the ADR decision;
4. after approval, mark ADR-0009 Accepted and reconcile `docs/adr/README.md`, `docs/architecture/MASTER-ARCHITECTURE.md`, `docs/plans/phase-1/M9-remote-networking-design.md`, and this file;
5. run the exact final repository gate;
6. commit/push the accepted-doc reconciliation, open the Task 10 PR, require exact-head CI, merge with an expected-head guard, and verify post-merge `main`;
7. hand off directly to M10 — First Platform Vertical Slice — with the recorded Android/mobile networking validation obligations.

## Resume Procedure

1. start from branch `m9-task10-adr-decision`;
2. verify canonical base `main` = `e14ff354f7eeb3c61eec10cef687c9af08876311`;
3. read the Master Architecture, this file, `docs/plans/phase-1/M9-remote-networking-design.md`, `docs/plans/phase-1/M9-remote-networking.md`, `docs/research/M9-networking-evidence.md`, ADR-0008, and Proposed ADR-0009;
4. preserve Task 8 evidence head/run and the explicit `Libp2p trigger: no` result;
5. do not add `rust-libp2p`;
6. do not mark ADR-0009 Accepted or edit normative networking status before explicit owner approval;
7. after approval, reconcile architecture/docs, run the full gate, integrate Task 10, and verify `main`.
