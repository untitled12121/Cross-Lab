# Current Development State

This file is the durable resume guide for active Cross-Lab development. Git/code/tests are factual state; this file records the intended handoff.

## Current Phase

**Phase 1 — Core Simulator / Real Transport Transition**

## Current Milestone

**M9 — Remote Networking ADR. Foundation authority/replay remediation and remote-networking Tasks 1–7 are merged and post-merge verified. Task 8 is active: the reproducible local benchmark surface through Step 3 is complete and verified; safe cross-process rendezvous is next.**

M1–M8 are complete. ADR-0009 remains undecided/reserved; Iroh stays isolated under `experiments/m9-networking` until the M9 evidence gate selects a remote-networking architecture.

## Canonical Baseline

- Task 8 active branch: `m9-task8-network-evidence`.
- Task 8 Step 3 exact verified head: `bf499d60501355b8f83f44a1a168340f1bf969eb`.
- Task 8 Step 3 verification CI `35063462476` passed `cargo test -p crosslab-m9-networking`, `cargo fmt --all -- --check`, `cargo check -p crosslab-m9-networking --all-targets`, and `cargo clippy -p crosslab-m9-networking --all-targets -- -D warnings`.
- Task 8 Quinn-auth RED CI `35058755454` failed for the intended missing `SessionAuthMicros` metric before the benchmark implemented Cross-Lab session authentication.
- Task 8 focused Quinn-auth GREEN CI `35059207156` passed on `968515848cf545ff7bbf98c01080dba7dabb7c25`.
- Task 8 local benchmark work now includes typed `local-quinn`, `local-iroh-direct`, `local-iroh-relay`, and `local-all` modes; reproducibility headers; the `m9-networking` binary; Linux RSS/FD observations; and protected-connect, Cross-Lab auth, control RTT, fixed uni throughput, and shutdown metrics for Quinn/Iroh modes.
- Quinn auth measurement remains experiment-only and reuses the existing Cross-Lab hello/proof/session-activation semantics with exact ADR-0008 `quic-tls-exporter-v1`; the verified M8 production Quinn adapter remains unchanged.
- Task 7 exact PR head: `dfad21610e1b1b93bb1bc81eb8cc989324fd2960` via PR **#27 — test(networking): validate owner relay and Iroh path changes**.
- Task 7 exact-head PR CI `35042144793` passed lockfile verification, dependency audit, rustfmt, full workspace check, Clippy with `-D warnings`, and complete workspace tests.
- Task 7 merge: `d897e803c9b66236344101b400c002e86dcc5523`.
- Task 7 post-merge Rust CI `35042588052` passed the same full gate on `main`.
- Task 7 RED run `35037785170` failed for the intended missing `scenarios::relay` surface before relay orchestration existed.
- Task 7 GREEN run `35038289013` passed the focused relay tests and committed implementation `365932cd5a640cb763556dd69c5c9150ed8f5f06`.
- Task 7 widened semantic gate `35038546481` passed `relay`, `session`, and `lifecycle` on `18e31b2caa94688d10e0767f2527378777411c38`.
- Task 7 pre-PR full gate `35039448229` passed lockfile verification, dependency audit, rustfmt, full workspace check, Clippy with `-D warnings`, and complete workspace tests on `f565d08bbe54c039bb20b80a9aceb81bd2c86374`.
- Final requirements review found that the original Task 7 relay-to-direct evidence did not explicitly prove control-sequence continuity or survival of already-issued operation authority across the same live Iroh path change, despite both being M9 invariants.
- Task 7 invariant RED run `35040355796` failed for the intended missing scenario helpers after tests were added for sequence progression and pre-path-change operation use.
- Task 7 invariant GREEN run `35040628969` passed the focused `relay + session + lifecycle` gate and committed the repair as `d38a6d49afdee7aa7b455cdb3da517c5249e9853`.
- Task 7 repaired pre-PR full gate `35040810828` passed the complete repository gate on implementation/gate head `97e892dbd2554c4c17cc9cd9da888d16e8521a67`.
- After that gate, only `CURRENT.md` and deletion of all three temporary Task 7 workflows changed before the exact PR head; no implementation code changed after `97e892db...`.
- Task 6 merge: `ec213a20176787c6da18217603e163b8259eae7e` via PR #26; post-merge Rust CI `35018281624` passed the full gate.
- Task 5 merge: `80118866d38525a33ee2f7cae85c2d82c9037f0d` via PR #25; post-merge Rust CI `35011352926` passed.
- Task 4 merge: `3a403661ed02bcaf59f6cdfe45ad8425d7e2275a` via PR #24; post-merge Rust CI `35003931830` passed.
- Foundation remediation merge: `450615a7c361384cfbe68bc8991b7ca03e4482fa` via PR #23; exact-head Rust CI `34974781151`, Fuzz Smoke `34974781112`, and post-merge Rust CI `34979066740` passed.
- `paste 1.0.15` / `RUSTSEC-2024-0436` remains the sole allowed maintenance warning, isolated to the Iroh experiment and requiring re-evaluation before production networking promotion.

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

No Master Architecture revision, protobuf schema change, canonical transcript change, signature-format change, or Quinn channel-binding change was introduced by the remediation.

## M9 Networking State

- Quinn remains the verified local/LAN baseline.
- Iroh `1.2.0` remains isolated under `experiments/m9-networking` until ADR-0009 selects the remote-networking architecture.
- M9 networking Tasks 1–7 are complete on `main` and post-merge verified; Task 8 is active on `m9-task8-network-evidence`.
- Task 8 Step 3 provides typed local benchmark commands with explicit sample/payload limits and a reproducible TSV header containing OS/architecture, Rust `1.98.1`, sample count, payload bytes, Quinn `0.11.11`, and Iroh `1.2.0`.
- Task 8 local measurements cover protected connect, Cross-Lab session authentication, control RTT, fixed-size unidirectional throughput, and shutdown across Quinn, Iroh direct, and Iroh relay modes.
- The Quinn Task 8 benchmark performs the existing Cross-Lab hello/proof/session activation over raw experiment-owned Quinn streams with the exact ADR-0008 exporter profile; it does not change production Quinn APIs or domain boundaries.
- Task 8 Linux resource reporting reads RSS from `/proc/self/status` and FD count from `/proc/self/fd`; unsupported targets report no value rather than adding unsafe platform calls.
- The Task 8 `m9-networking` executable runs the typed local benchmark commands and emits the reproducible TSV report.
- Task 8 Step 4 safe rendezvous, Step 5 controlled namespace topology, deterministic full workspace gate, same-host evidence collection, controlled NAT/relay evidence, and the evidence report remain unfinished.
- Task 4 established bounded Iroh control/uni-stream semantics behind the existing `TransportConnection` seam; no Iroh type entered Cross-Lab domain/public APIs.
- Task 5 reuses the existing Cross-Lab hello/proof/session activation protocol over Iroh with ADR-0008 `quic-tls-exporter-v1` binding after the full handshake. Iroh endpoint identity remains routing metadata only.
- Authenticated Iroh pairs are fixed to `NetworkClass::Remote` with no setter; no 0-RTT authority exists.
- Task 6 proves capability/control request-response-event flow, authorized operation-bound streams, unknown-operation rejection, fresh reconnect binding/`SessionId`, rejection of old proof/session/operation authority, signed revocation termination and reconnect denial, bounded saturation/cancellation/close/joined shutdown, and `Constraint::LocalOnly` rejection under immutable `NetworkClass::Remote`.
- Task 7 adds `iroh-relay 1.2.0` only to the isolated M9 experiment and runs a self-hosted relay bound locally; required tests do not depend on public n0 relay infrastructure.
- Task 7 proves an authenticated Cross-Lab session carries bounded control and unidirectional data through an owner-controlled relay with IP transports disabled.
- Task 7 proves a relay-assisted connection can observe a direct IP path without changing immutable `NetworkClass::Remote`, the authenticated ADR-0008 exporter binding, or the logical Cross-Lab `SessionId`.
- Task 7 explicitly proves control `message_seq` continues across the live relay-to-direct path observation rather than resetting or creating a new session sequence domain.
- Task 7 explicitly proves an operation grant issued before the live path change remains usable afterward on that same authenticated Iroh connection; the route observation does not mint, replace, or revoke authority.
- Task 7 path observation remains experiment orchestration only; it does not write policy, trust, operation, or session authority and does not promote Iroh into production/domain APIs.
- A new Iroh transport connection still requires fresh Cross-Lab authentication and authorization state; route changes within one live authenticated connection do not create new authority.
- Security authority remains local and fail-closed.

## Deferred / External Risks

- Durable rollback-resistant persistence for `OwnerAuthorityState` and accepted authority epochs.
- Platform key-store / secure-enclave integration and real mobile lifecycle evidence.
- Explicit authorization/subscription design for future authority-sensitive system-event families.
- `main` branch protection and other repository-administration controls.
- Windows/macOS/mobile CI matrices as their platform slices land.
- `paste 1.0.15` / `RUSTSEC-2024-0436` in the isolated Iroh experiment, to re-evaluate before ADR-0009 production promotion.
- Controlled Linux NAT/relay topology evidence, recovery/path-change evidence under forced topology, and final same-host/resource measurements required before ADR-0009 can select a production remote-connectivity architecture.

## Exact Next Task

Continue **M9 remote-networking Task 8 Step 4 — safe cross-process rendezvous** from `docs/plans/phase-1/M9-remote-networking.md` on branch `m9-task8-network-evidence`.

Write RED tests first for the strict public-routing-only rendezvous format, then implement `src/netprobe.rs` and typed netprobe command arguments. The rendezvous surface may contain only `endpoint_id`, optional `relay_url`, and optional `ip`, reconstructing `EndpointAddr` from those values. It must not persist private keys, relay tokens, Cross-Lab credentials/proofs, exporter bytes, or payloads.

After Step 4 is focused-green, proceed to the fixed Linux namespace topology script, then the deterministic full workspace gate, same-host measurements, controlled NAT/relay evidence, and `docs/research/M9-networking-evidence.md`. Do not start Task 9 unless that report explicitly records `Libp2p trigger: yes` with a concrete failed Iroh criterion that libp2p plausibly addresses.

## Resume Procedure

1. verify branch `m9-task8-network-evidence`, this file, the active M9 plan, Master Architecture, relevant session/transport ADRs, recent commits, and repository state;
2. preserve Task 8 Step 3 exact verified head `bf499d60501355b8f83f44a1a168340f1bf969eb` and CI `35063462476` as the local-benchmark checkpoint;
3. write Step 4 RED tests for strict rendezvous serialization/parsing/reconstruction and typed netprobe CLI arguments;
4. verify the intended missing-netprobe/rendezvous failure before implementation;
5. implement the smallest `netprobe.rs` surface using only public routing metadata and explicit `EndpointAddr` reconstruction;
6. verify focused Step 4 tests plus formatting/check/Clippy before starting namespace orchestration;
7. implement the fixed namespace/NAT/relay script with deterministic refusal/cleanup behavior, then run the Task 8 deterministic full repository gate;
8. collect same-host and controlled NAT/relay evidence without persisting private keys, credentials/proofs, exporter bytes, relay credentials, or payloads;
9. write `docs/research/M9-networking-evidence.md`, including an explicit `Libp2p trigger: yes/no` decision grounded in observed Iroh criteria;
10. remove temporary Task 8 workflows, checkpoint `CURRENT.md`, integrate Task 8 only after exact-head CI is green, and verify `main` post-merge before deciding whether conditional Task 9 is triggered.
