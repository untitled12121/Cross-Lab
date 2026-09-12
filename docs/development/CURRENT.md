# Current Development State

This file is the durable resume guide for active Cross-Lab development. Git/code/tests are factual implementation state; this file records the intended handoff.

## Current Phase

**Phase 1 — Core Simulator / Real Transport Transition**

## Current Milestone

**M9 — Remote Networking ADR: Task 1 Quinn baseline is GREEN; Task 2 Iroh exporter proof is next.**

M1–M8 are complete and integrated into canonical `main`. M9 remains isolated on `m9-remote-networking`. M10 is the first Linux + Android platform vertical slice after M9 selects the remote-networking architecture.

## Canonical Baseline

M8 — Quinn Transport is integrated and independently verified on `main`.

- final M8 feature head: `a93c6367be3174241cbab65b870f16f1543971f9`;
- feature-head CI: `34688453044` — locked metadata, rustfmt, workspace check, Clippy `-D warnings`, and full workspace tests passed;
- PR #18 merged as `4a225976a16f34d7485fd259cacf767c84a60706`;
- post-merge main CI: `34688527098` — full required gate passed;
- canonical M8 documentation head: `ab602d62181b3dcba056874d0013e40522a68e8f`;
- documentation-head CI: `34688627614` — full required gate passed.

The M9 branch was created from exactly `ab602d62181b3dcba056874d0013e40522a68e8f`.

## M9 Research and Approved Design

Uploaded/current research baseline:

- Iroh `1.2.0`, Rust 2024, `rust-version = 1.91`, `MIT OR Apache-2.0`;
- rust-libp2p `0.57.0`, Rust 2024, workspace `rust-version = 1.88.0`, MIT;
- Iroh exposes full-handshake TLS exporter material, direct/relay path observability, explicit relay modes, and self-hosted relay support;
- Iroh `presets::Minimal` is the required M9 baseline so required tests do not silently depend on n0 address lookup/default relay infrastructure;
- rust-libp2p provides Relay v2/DCUtR/AutoNAT but its current public QUIC wrapper keeps the underlying Quinn connection private and brings a broader PeerId/Multiaddr/Swarm model.

No Iroh or libp2p dependency has been added to a Cross-Lab production/domain crate.

Approved design:

- `docs/plans/phase-1/M9-remote-networking-design.md`;
- original design commit: `9bed951d16aca0ceef08328162cb284ecc26f7d3`;
- design/review checkpoint CI: `34689499316` — full required gate passed.

Approved direction:

```text
Cross-Lab identity / trust / policy
             |
      LogicalSession
             |
     TransportConnection
        /            \
 local/LAN        remote/Internet
 Quinn M8          Iroh candidate
                      |
              direct path or relay
```

Required invariants:

- Quinn stays the verified local/LAN baseline;
- Iroh is evaluated first as the isolated remote/NAT/relay candidate;
- Iroh EndpointId/SecretKey and relay credentials never become Cross-Lab identity/trust/policy authority;
- required tests use explicit Minimal endpoint configuration and owner-selected/self-hosted relay infrastructure only;
- the Iroh candidate must reproduce ADR-0008 `quic-tls-exporter-v1` exactly after full handshake;
- no 0-RTT authority;
- every Iroh-backed Cross-Lab session stays `NetworkClass::Remote` through relay/direct path changes;
- path changes inside a live protected connection do not change binding, SessionId, sequences, policy classification, or operation authority;
- a new Iroh connection is a full Cross-Lab reconnect;
- no generic transport framework extraction or production Iroh/libp2p dependency is authorized by prototype success alone.

## M9 Implementation/Evaluation Plan

Committed plan:

- `docs/plans/phase-1/M9-remote-networking.md`;
- refined self-reviewed plan head: `c4fb6ec48a3325ede82d536051f6e145d9f8808d` (`docs: tighten M9 evaluation plan`);
- durable plan checkpoint: `9add4d7e809d5579fed71b5b279ac9bc7481fc90`, CI `34691111935` — full required gate passed.

The plan contains ten reviewer-sized gates:

1. reproducible Quinn baseline + typed metrics;
2. direct Iroh + exact ADR-0008 exporter proof;
3. bounded record/control bridge;
4. bounded uni-stream bridge + experiment-only `TransportConnection`;
5. existing Cross-Lab session authentication over Iroh;
6. direct-Iroh control/data/reconnect/revocation semantics;
7. owner-controlled relay + path-change invariants;
8. reproducible benchmark CLI + controlled Linux NAT/relay gate + evidence report;
9. conditional rust-libp2p probe only if the evidence trigger is satisfied;
10. ADR-0009, Master Architecture reconciliation, exact-head verification/merge, and M10 handoff.

## M9 Completed Work

### Task 1 — Reproducible Quinn baseline and typed metrics

Added the non-publishable `experiments/m9-networking` workspace crate without adding Iroh/libp2p. The experiment currently provides:

- typed `EvalConfig` with one-sample/64 KiB test defaults and five-sample/4 MiB measurement defaults;
- typed `TransportKind`, `MetricKind`, `Measurement`, and deterministic TSV `Report` output;
- a raw Quinn `0.11.11` loopback baseline that measures full protected connection establishment, bounded 32-byte control ping/echo RTT, bounded unidirectional bulk throughput, and connection/endpoint shutdown;
- the baseline deliberately does not call or expose the crate-private production `QuicTransportConnection::new` constructor;
- Task 1 Tokio features were trimmed to `macros`, `rt-multi-thread`, `sync`, and `time`; unused `process`/`net` feature expansion was not retained.

RED evidence:

- RED head: `6ce81462d3489a8b5855f39978a81224f841fa09`;
- CI `34692635013` passed lockfile/format and failed workspace check only because the intended `config`, `metrics`, and `baseline` APIs were absent.

GREEN evidence:

- verified implementation head: `785f93f914f454b1c902c76fe101c31d1db14e2a`;
- CI `34692865511` passed lockfile verification, rustfmt, workspace check, Clippy `-D warnings`, and `cargo test --workspace --all-features`;
- `quinn_baseline_records_connect_control_bulk_and_shutdown` passed;
- `report_tsv_schema_is_stable` passed;
- no Iroh/libp2p dependency is present at this checkpoint.

## Required Evidence Before ADR-0009

Deterministic tests without public infrastructure must prove exporter equality/freshness, existing session authentication, control/data semantics, reconnect/replay rejection, revocation, relay-only authenticated connectivity, path-change observability, Remote classification invariance, bounded saturation/cancellation, connection loss, and joined shutdown.

Reproducible measurements compare Quinn and Iroh on the same host for protected connection/session establishment, control latency, bulk uni throughput, relay behavior, relay-to-direct upgrade, recovery, resource usage, and shutdown.

NAT traversal requires the controlled Linux translated-network gate from the plan where practical. Loopback relay tests alone are not accepted as NAT evidence. Real Android/mobile lifecycle remains an explicit M10 obligation and is not claimed by M9.

## Active Pull Request

- Draft PR #19: `docs: design M9 remote networking ADR`.
- Base: canonical `main` at `ab602d62181b3dcba056874d0013e40522a68e8f`.
- Head branch: `m9-remote-networking`.
- Keep draft until M9 evidence, ADR decision, final verification, and exact-head closeout are complete.

## Exact Next Task

Begin **Task 2 — Direct Iroh endpoints and exact ADR-0008 exporter proof** from `docs/plans/phase-1/M9-remote-networking.md`.

Task 2 may add Iroh `1.2.0` only to `experiments/m9-networking`, with default features disabled and only the reviewed experiment features. Required tests must use `presets::Minimal`, explicit address data, relay disabled, normal full handshakes, and the exact ADR-0008 exporter profile.

Do not add Iroh to production/domain crates, do not use `N0` defaults/public infrastructure, do not use 0-RTT authority, and do not substitute another channel-binding scheme if exporter compatibility fails.

## Resume Procedure

1. inspect canonical `main`, `m9-remote-networking`, PR #19, recent commits/workflows, and this file;
2. read the Master Architecture, approved M9 design, M9 implementation plan, ADR-0008, M8 design, and transport/session contracts;
3. inspect uploaded Iroh/rust-libp2p/Quinn source before changing related behavior;
4. execute the committed plan task-by-task and keep candidate dependencies isolated;
5. preserve RED -> GREEN evidence and run full verification after meaningful milestones;
6. keep this file current with exact commits/CI/manual evidence;
7. merge only the exact verified reviewed head and verify canonical `main` after integration.
