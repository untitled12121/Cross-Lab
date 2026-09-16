# Current Development State

This file is the durable resume guide for active Cross-Lab development. Git/code/tests are factual state; this file records the intended handoff.

## Current Phase

**Phase 1 — Core Simulator / Real Transport Transition**

## Current Milestone

**M9 — Remote Networking ADR. Tasks 1–7 are merged and post-merge verified. Task 8 implementation and evidence collection are complete on `m9-task8-network-evidence`; the branch now needs temporary-workflow cleanup, final exact-head verification, PR integration, and post-merge verification.**

M1–M8 are complete. ADR-0009 remains undecided/reserved. Iroh stays isolated under `experiments/m9-networking` until ADR-0009 selects the remote-networking architecture.

## Canonical Baseline

- Active branch: `m9-task8-network-evidence`.
- Task 8 exact successful implementation/evidence head: `99b168101e9fc31bdfbcea110bdb95d26978dac4`.
- Exact successful evidence run: GitHub Actions `35107158922`, job `104831250278`.
- That run passed the Task 8 dependency-tree step, full deterministic workspace gate, netprobe build, controlled Linux namespace/NAT/relay gate, environment capture, and same-host measurements.
- Evidence report commit: `4986b8064bdf47ad6ed52eb1907be11d82a0129f` (`docs/research/M9-networking-evidence.md`).
- Controlled NAT result on the successful evidence run:
  - owner-relay fallback verified;
  - immutable `NetworkClass::Remote` verified;
  - authenticated control verified;
  - authenticated data verified;
  - direct path remained unavailable in the endpoint-dependent/symmetric NAT topology within the bounded observation window;
  - forced reconnect verified a fresh channel binding, fresh Cross-Lab session, and authenticated control after reconnect.
- The evidence report explicitly records **`Libp2p trigger: no`**. Task 9 is therefore skipped and `rust-libp2p` must not be added for M9.
- Task 7 exact PR head: `dfad21610e1b1b93bb1bc81eb8cc989324fd2960` via PR #27; exact-head CI `35042144793` and post-merge main CI `35042588052` passed the complete repository gate. Task 7 merge: `d897e803c9b66236344101b400c002e86dcc5523`.
- Foundation remediation merge: `450615a7c361384cfbe68bc8991b7ca03e4482fa` via PR #23; exact-head Rust CI `34974781151`, Fuzz Smoke `34974781112`, and post-merge Rust CI `34979066740` passed.
- `paste 1.0.15` / `RUSTSEC-2024-0436` remains the sole allowed maintenance warning, isolated to the Iroh experiment and requiring re-evaluation before any production networking promotion.

## Accepted Foundation State

Accepted artifacts remain:

- `docs/adr/ADR-0010-authoritative-owner-authority-currentness.md`
- `docs/adr/ADR-0011-bounded-request-replay-semantics.md`
- `docs/superpowers/specs/2026-09-14-foundation-currentness-replay-design.md`
- `docs/superpowers/plans/2026-09-14-foundation-authority-replay-remediation.md`

The accepted foundation keeps identity-owned `OwnerAuthorityState` authoritative for owner/delegated-role currentness; root or Device Signing replacement invalidates ordinary sessions and session-scoped authority; Administrative/Recovery-only rotation does not; `(SessionId, message_seq)` remains exact-envelope replay/order protection; `RequestId` remains bounded retry/correlation history; high-level stream admission derives currentness from local trust/policy state; and canonical identity/protocol wire vectors remain unchanged.

No Task 8 work changed the Master Architecture, protobuf schemas, canonical transcripts, signature formats, ADR-0008 exporter profile, production Quinn transport, or ADR-0009 status.

## M9 Task 8 Evidence State

- Quinn remains the verified local/LAN production baseline.
- Iroh `1.2.0` and `iroh-relay 1.2.0` remain isolated under `experiments/m9-networking`; no Iroh type has entered Cross-Lab domain/public production APIs.
- Reproducible local modes cover Quinn, Iroh direct, Iroh owner relay, and combined runs with protected-connect, Cross-Lab-auth, control-RTT, fixed uni-throughput, shutdown, Linux RSS, and Linux FD observations.
- The `m9-networking` netprobe supports typed owner-relay/server/client process modes.
- Persisted rendezvous metadata contains only `endpoint_id`, optional `relay_url`, and optional `ip`; extra fields are rejected. Private keys, tokens, credentials/proofs, exporter bytes, and payload contents are not persisted by the rendezvous/evidence path.
- The Linux namespace harness owns fixed namespaces `cl-m9-ra`, `cl-m9-rb`, `cl-m9-a`, `cl-m9-b`, bridge `cl-m9-br`, transit `172.30.90.0/24`, peer networks `10.90.1.0/24` and `10.90.2.0/24`, owner relay `172.30.90.1`, namespace-local forwarding/NAT, direct-UDP gating, collision refusal, and cleanup.
- The controlled topology successfully uses owner relay for authenticated Remote control/data but does not obtain a direct path after UDP is enabled. This is recorded as `direct_unavailable`, not hidden or converted into weaker auth/session semantics.
- Separate deterministic relay-to-direct tests verify unchanged Remote classification, exporter binding, `SessionId`, control sequence state, and active operation authority through a live path change.
- Forced reconnect requires fresh authentication/binding/session state and succeeds after the controlled direct-path miss.
- Task 8 Linux evidence does not close Android, iOS, Windows, macOS, real-device lifecycle, background-networking, entitlement, firewall, or secure-keystore obligations.
- Evidence report: `docs/research/M9-networking-evidence.md`.
- **Libp2p trigger: no. Task 9 is skipped.**

## Deferred / External Risks

- Durable rollback-resistant persistence for `OwnerAuthorityState` and accepted authority epochs.
- Platform key-store / secure-enclave integration and real mobile lifecycle evidence.
- Explicit authorization/subscription design for future authority-sensitive system-event families.
- `main` branch protection and other repository-administration controls.
- Windows/macOS/mobile CI matrices as their platform slices land.
- `paste 1.0.15` / `RUSTSEC-2024-0436` in the isolated Iroh experiment.
- ADR-0009 must explicitly account for the controlled symmetric-NAT direct-path limitation and the remaining mobile/platform evidence obligations before any production promotion.

## Exact Next Task

Finish **M9 Task 8 integration** on `m9-task8-network-evidence`:

1. verify the evidence-report/CURRENT documentation head;
2. delete the temporary branch-only `.github/workflows/m9-task8-red.yml` evidence workflow;
3. inspect the final branch diff and run/trigger the repository's normal exact-head CI path;
4. open the Task 8 PR only after the final branch contains no temporary workflow;
5. require exact-head PR CI to be green before merge;
6. merge Task 8 and verify `main` post-merge;
7. checkpoint `CURRENT.md` on `main` if the merge state/run IDs need to be recorded;
8. proceed directly to **Task 10 / ADR-0009**. Do not start Task 9 because the evidence report records `Libp2p trigger: no`.

## Resume Procedure

1. verify branch `m9-task8-network-evidence`, this file, `docs/research/M9-networking-evidence.md`, the active M9 plan, Master Architecture, recent commits, and repository state;
2. preserve Task 8 evidence head `99b168101e9fc31bdfbcea110bdb95d26978dac4` and successful evidence run `35107158922` as the implementation/evidence checkpoint;
3. preserve evidence report commit `4986b8064bdf47ad6ed52eb1907be11d82a0129f` and its explicit `Libp2p trigger: no` decision;
4. remove only the temporary Task 8 workflow, not normal repository CI;
5. verify the real final branch/PR head before claiming Task 8 complete;
6. verify post-merge `main` before starting ADR-0009;
7. do not add `rust-libp2p` or start Task 9 unless an approved future evidence change explicitly reverses the recorded trigger.
