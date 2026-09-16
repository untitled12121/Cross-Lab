# Current Development State

This file is the durable resume guide for active Cross-Lab development. Git/code/tests are factual state; this file records the intended handoff.

## Current Phase

**Phase 1 — Core Simulator / Real Transport Transition**

## Current Milestone

**M9 — Remote Networking ADR. Tasks 1–7 are merged and post-merge verified. Task 8 is active: reproducible local benchmarks and safe cross-process rendezvous through Step 4 are complete and verified; Step 5 controlled Linux namespace/NAT/relay orchestration is next.**

M1–M8 are complete. ADR-0009 remains undecided/reserved. Iroh stays isolated under `experiments/m9-networking` until the evidence gate selects a remote-networking architecture.

## Canonical Baseline

- Active Task 8 branch: `m9-task8-network-evidence`.
- Task 8 Step 4 exact verified head: `95acc6f94f1b5b828363337938450061f24cb3df`.
- Task 8 Step 4 widened verification CI `35078260860` passed `cargo test -p crosslab-m9-networking`, `cargo fmt --all -- --check`, `cargo check -p crosslab-m9-networking --all-targets`, and `cargo clippy -p crosslab-m9-networking --all-targets -- -D warnings`.
- Safe-rendezvous RED CI `35063846134` failed for the intended missing `netprobe` module; focused rendezvous GREEN CI `35064080543` passed on `26c487480ec23bd1cec96cb7dd627638b25354c4`.
- Typed-netprobe-command RED CI `35064311543` failed for the intended missing argument types/command variants.
- The first typed-command GREEN attempt exposed the intended runner seam: `baseline::run_local` became non-exhaustive once top-level netprobe variants existed. Boundary RED CI `35077091137` reproduced that exact failure after adding the regression test.
- Focused typed-command/local-runner GREEN CI `35077644965` passed on `717eca6a7e2edd2cf0251daca563f66b962480b5`; netprobe commands are explicitly rejected by the local benchmark runner rather than leaking process orchestration into baseline logic.
- The first widened Step 4 run `35077889864` passed the full M9 test suite and failed only rustfmt; formatter-only commit `95acc6f94f1b5b828363337938450061f24cb3df` produced the final green gate above.
- Task 8 Step 3 exact verified head: `bf499d60501355b8f83f44a1a168340f1bf969eb`; CI `35063462476` passed the same crate-level tests/fmt/check/Clippy gate.
- Task 8 local benchmark modes now include typed `local-quinn`, `local-iroh-direct`, `local-iroh-relay`, and `local-all`; reproducibility headers; the `m9-networking` binary; Linux RSS/FD observations; and protected-connect, Cross-Lab-auth, control-RTT, fixed uni-throughput, and shutdown measurements.
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

No Task 8 work has changed the Master Architecture, protobuf schemas, canonical transcripts, signature formats, ADR-0008 exporter profile, production Quinn transport, or ADR-0009 status.

## M9 Networking State

- Quinn remains the verified local/LAN baseline.
- Iroh `1.2.0` remains isolated under `experiments/m9-networking`; no Iroh type has entered Cross-Lab domain/public production APIs.
- Authenticated Iroh sessions remain immutable `NetworkClass::Remote`; endpoint identity/path state is routing metadata only and cannot create policy/session/operation authority.
- Task 7 proves owner-controlled relay operation, authenticated control/data over relay, relay-to-direct path observation on one live authenticated session, sequence continuity, operation-authority continuity, unchanged Remote classification/binding/session through the live path change, and fresh auth/binding/session requirements on reconnect.
- Task 8 Step 3 provides reproducible same-host benchmark/report machinery and measures protected connect, Cross-Lab session auth, control RTT, fixed-size unidirectional throughput, shutdown, and Linux RSS/FD observations across Quinn/Iroh modes.
- Task 8 Step 4 adds strict public-routing-only rendezvous serialization/parsing/reconstruction. The persisted format can contain only `endpoint_id`, optional `relay_url`, and optional `ip`; extra fields are rejected, including secret-bearing additions.
- `Rendezvous::endpoint_addr` reconstructs Iroh routing explicitly from those public values. Private keys, relay tokens, Cross-Lab credentials/proofs, exporter bytes, and payloads are not represented by the persisted type.
- Top-level `Command` now includes typed `netprobe-relay`, `netprobe-server`, and `netprobe-client` arguments. Netprobe commands have no `EvalConfig` and are explicitly outside `baseline::run_local`.
- Step 5 controlled namespace/process orchestration, Step 6 deterministic full workspace gate, Step 7 same-host evidence capture, Step 8 controlled NAT/relay evidence, and Step 9 evidence report remain unfinished.
- Security authority remains local and fail-closed.

## Deferred / External Risks

- Durable rollback-resistant persistence for `OwnerAuthorityState` and accepted authority epochs.
- Platform key-store / secure-enclave integration and real mobile lifecycle evidence.
- Explicit authorization/subscription design for future authority-sensitive system-event families.
- `main` branch protection and other repository-administration controls.
- Windows/macOS/mobile CI matrices as their platform slices land.
- `paste 1.0.15` / `RUSTSEC-2024-0436` in the isolated Iroh experiment.
- Controlled Linux NAT/relay topology evidence, recovery/path-change evidence under forced topology, and final same-host/resource measurements required before ADR-0009 can select a production remote-connectivity architecture.

## Exact Next Task

Continue **M9 remote-networking Task 8 Step 5 — Linux namespace topology and netprobe process orchestration** from `docs/plans/phase-1/M9-remote-networking.md` on branch `m9-task8-network-evidence`.

Start with RED tests for the fixed topology contract. The script must use namespaces `cl-m9-ra`, `cl-m9-rb`, `cl-m9-a`, `cl-m9-b`; bridge `cl-m9-br`; transit `172.30.90.0/24`; peer networks `10.90.1.0/24` and `10.90.2.0/24`; refuse pre-existing planned namespaces/bridge; enable forwarding only inside router namespaces; add namespace-local nftables masquerade; run the owner relay on `172.30.90.1`; block router-to-router direct UDP before allowing it; and install `trap cleanup EXIT INT TERM` that removes owned processes/network state/rendezvous files.

The script must orchestrate real `netprobe-relay`, `netprobe-server`, and `netprobe-client` runtime behavior. Keep ephemeral Iroh/private Cross-Lab test keys in process memory only. Persist routing metadata and boolean/measurement evidence only—never private keys, credentials/proofs, exporter bytes, relay credentials, or payload contents.

After Step 5 is focused/widened green, run the Step 6 full workspace gate, then collect Step 7/8 measurements and write `docs/research/M9-networking-evidence.md`. Do not start Task 9 unless that report explicitly records `Libp2p trigger: yes` with a concrete failed Iroh criterion that libp2p plausibly addresses.

## Resume Procedure

1. verify branch `m9-task8-network-evidence`, this file, the active M9 plan, Master Architecture, relevant session/transport ADRs, recent commits, and repository state;
2. preserve Step 4 exact verified head `95acc6f94f1b5b828363337938450061f24cb3df` and CI `35078260860` as the safe-rendezvous checkpoint;
3. attach/verify Step 5 RED tests for the fixed namespace/NAT/relay script before creating `scripts/netns.sh`;
4. implement the smallest deterministic topology script with collision refusal and owned cleanup;
5. implement/finish explicit binary dispatch and netprobe relay/server/client runtime needed by the script, reusing existing owner-relay, Iroh endpoint, auth, lifecycle, and path-observation code rather than duplicating authority logic;
6. verify focused Step 5 tests and the M9 crate gate, then run the required full workspace Step 6 gate;
7. collect same-host and controlled NAT/relay evidence without persisting private keys, credentials/proofs, exporter bytes, relay credentials, or payloads;
8. record relay fallback, authenticated control/data, direct-path appearance, unchanged Remote/binding/session through live path change, fresh binding/session after reconnect, recovery/path-change behavior, resource observations, and failures/anomalies;
9. write `docs/research/M9-networking-evidence.md` with an explicit evidence-grounded `Libp2p trigger: yes/no` decision;
10. remove temporary Task 8 workflows, checkpoint `CURRENT.md`, integrate Task 8 only after exact-head CI is green, and verify `main` post-merge before any conditional Task 9 work.
