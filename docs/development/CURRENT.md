# Current Development State

This file is the durable resume guide for active Cross-Lab development. Git/code/tests are factual implementation state; this file records the intended handoff.

## Current Phase

**Phase 1 — Core Simulator / Real Transport Transition**

## Current Milestone

**M8 — Quinn Transport: implementation and architecture reconciliation complete on `m8-quinn-transport`; final exact-head CI and integration to `main` are the only remaining M8 steps.**

M1–M7 are already integrated into canonical `main`. M8 remains isolated in PR #18 until the documentation-closeout head passes the full gate.

## M8 Result

M8 adds the first real encrypted Cross-Lab IP transport while preserving the existing transport-neutral domain architecture.

Implemented and verified behavior includes:

- isolated `crosslab-transport-quic` adapter using Quinn `0.11.11` on Tokio `1.53.1`;
- accepted ADR-0008 TLS-exporter channel binding profile `quic-tls-exporter-v1` with 32-byte output, label `EXPORTER-Cross-Lab-QUIC-Channel-Binding-v1`, and context `crosslab.quic.transport.v1`;
- private bounded `u32` big-endian QUIC record framing with declared-length validation before allocation;
- ownership-preserving `TooLarge`, `Full`, and `Closed` transport outcomes;
- bounded ordered control bridge with local backpressure and terminal remote-close propagation;
- bounded unidirectional data-stream bridge with opening-frame admission, chunk backpressure, FIN, RESET, STOP, cancellation, connection-loss propagation, and joined connection-owned tasks;
- the existing Cross-Lab session-auth hello/proof flow running over the Quinn TLS-exporter binding without a second authentication transcript or certificate-to-`DeviceId` mapping;
- ordinary capability advertisement, control request/response/event behavior through the existing `SimNode` path over Quinn;
- operation-bound data streams through the existing `SimStreamRuntime` path over Quinn;
- reconnect using a fresh Quinn connection, exporter binding, nonces/proofs, `SessionId`, sequence state, capability state, and operation authority;
- replayed old proofs and old operation/session authority rejected on reconnect;
- accepted signed peer revocation terminating active authority and preventing a fresh Cross-Lab session from reaching `Active`;
- bounded saturation, cancellation, active-stream shutdown, and task joining.

No Quinn/Tokio/rustls/socket/TLS-certificate type entered the public/domain state of `crosslab-core`, `crosslab-protocol`, `crosslab-policy`, or `crosslab-identity`. Quinn/TLS protects the connection but does not define Cross-Lab identity or policy authority.

## M8 Resource Defaults

`QuicTransportConfig::default()` currently uses explicit nonzero bounds:

- control queue: 8 records;
- incoming stream queue: 8 streams;
- outgoing stream slots: 8 streams;
- per-stream chunk queue: 8 chunks;
- maximum control record: `256 KiB + 4` bytes;
- maximum opening record: `4 KiB + 4` bytes;
- maximum chunk: `64 KiB`;
- remote unidirectional stream limit: 32;
- remote bidirectional stream limit: 1;
- stream receive window: `512 KiB`;
- connection receive window: `4 MiB`;
- idle timeout: 30 seconds.

These are adapter defaults, not protocol permission or capability authority.

## Dependency and Research State

Production baseline:

- Quinn `0.11.11`, pinned with only `runtime-tokio` + `rustls-ring`;
- Tokio `1.53.1`;
- rustls `0.23.44` only where concrete loopback trust construction requires its public types;
- rcgen `0.14.10` dev-only for ephemeral loopback certificates.

The uploaded Quinn repository was inspected as research/reference material, and behavior that matters to the adapter was reconciled against the pinned `quinn-0.11.11` API. No Iroh or rust-libp2p dependency entered M8.

## Verification Evidence

Task 6 — Cross-Lab session authentication over Quinn:

- verified head `7a1294bd96f600af56e76e550ed853621c047e3b`;
- CI `34687210505` — locked metadata, rustfmt, workspace check, Clippy `-D warnings`, and full workspace tests passed.

Task 7 — Quinn-backed control/data/reconnect/revocation/failure lifecycle:

- verified head `f01378f1fe579149db1b4c5edd05d0e68a88d39b`;
- CI `34687943523` — full required gate passed;
- `crosslab-transport-quic` ran 27 tests at this checkpoint, including the new M8 lifecycle scenarios.

Task 8 architecture/documentation reconciliation before this `CURRENT.md` closeout:

- verified head `d1b44e32aca5219ec9b2cd8c86e599524bbae870`;
- CI `34688112428` — `cargo metadata --locked`, `cargo fmt --check`, workspace check, Clippy `-D warnings`, and `cargo test --workspace --all-features` all passed;
- ADR-0008 is indexed as Accepted;
- the M8 design status/research notes are reconciled with implemented reality;
- dependency review confirms Quinn/Tokio/rustls remain outside identity/policy/protocol/core;
- `.github/workflows/fuzz.yml` is not applicable to M8 because M8 does not change its watched protocol/policy/fuzz paths; no fuzz pass is claimed.

This `CURRENT.md` update is documentation-only. Its resulting exact branch head must pass the same full CI gate before PR #18 is merged.

## Intentional M8 Limits

M8 does not implement discovery, NAT traversal, relay selection, Iroh, rust-libp2p, route scoring/migration, TCP/TLS fallback, datagram consumers, production certificate provisioning, persistence, UI/platform adapters, privileged services, or installable Linux/Android applications.

Those exclusions are deliberate milestone boundaries, not missing M8 requirements.

## Integration Procedure

1. verify the exact documentation-closeout head of `m8-quinn-transport` with the full repository CI gate;
2. mark PR #18 ready and merge only that exact verified head to `main`;
3. verify canonical `main` after merge;
4. update this file on `main` with the canonical M8 integration commit/CI if needed for an unambiguous durable checkpoint;
5. do not delete valuable branch state until canonical `main` is verified.

## Exact Next Development Task

After M8 is integrated and canonical `main` is green, begin **M9 — Remote Networking ADR**.

Per the Master Architecture, M9 must prototype and measure Iroh and, where justified, rust-libp2p approaches against the verified Quinn baseline, then select the remote connectivity/NAT/relay architecture through an ADR.

M9 starts with research/design and benchmarks. Do not introduce Iroh/libp2p production dependencies, remote relay authority, or a second transport/session domain model before that decision is approved.

## Resume Procedure

1. inspect `main`, active branches/PRs, recent commits/workflows, and this file;
2. read the Master Architecture, active plan, relevant ADRs, and focused transport/session specifications;
3. reconcile documentation with actual code before changing behavior;
4. inspect relevant uploaded research repositories before implementing related systems;
5. work in small verifiable milestones and keep `CURRENT.md` current;
6. preserve architecture boundaries and record material changes through ADRs;
7. merge only exact verified heads and verify canonical `main` after integration.
