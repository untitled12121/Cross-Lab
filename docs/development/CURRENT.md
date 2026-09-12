# Current Development State

This file is the durable resume guide for active Cross-Lab development. Git/code/tests are factual implementation state; this file records the intended handoff.

## Current Phase

**Phase 1 — Core Simulator / Real Transport Transition**

## Current Milestone

**M9 — Remote Networking ADR**

M1–M8 are complete and integrated into canonical `main`. M9 is the next Phase 1 milestone; M10 remains the first Linux + Android platform vertical slice after the remote-networking architecture is selected.

## Canonical M8 Integration

M8 — Quinn Transport is integrated and independently verified on `main`.

- final feature head: `a93c6367be3174241cbab65b870f16f1543971f9`;
- exact feature-head CI: `34688453044` — locked metadata, rustfmt, workspace check, Clippy `-D warnings`, and full workspace tests passed;
- PR #18: `feat: implement M8 Quinn transport` — merged;
- canonical merge commit: `4a225976a16f34d7485fd259cacf767c84a60706`;
- post-merge canonical-main CI: `34688527098` — the same full required gate passed.

The merged result is the factual M8 baseline for M9.

## M8 Result

M8 adds the first real encrypted Cross-Lab IP transport while preserving the existing transport-neutral domain architecture.

Implemented and verified behavior includes:

- isolated `crosslab-transport-quic` adapter using Quinn `0.11.11` on Tokio `1.53.1`;
- accepted ADR-0008 TLS-exporter channel binding profile `quic-tls-exporter-v1` with 32-byte output, label `EXPORTER-Cross-Lab-QUIC-Channel-Binding-v1`, and context `crosslab.quic.transport.v1`;
- private bounded `u32` big-endian QUIC record framing with declared-length validation before allocation;
- ownership-preserving `TooLarge`, `Full`, and `Closed` transport outcomes;
- bounded ordered control bridge with local backpressure and terminal remote-close propagation;
- bounded unidirectional data-stream bridge with opening-frame admission, chunk backpressure, FIN, RESET, STOP, cancellation, connection-loss propagation, and joined connection-owned tasks;
- existing Cross-Lab session-auth hello/proof flow over the Quinn TLS-exporter binding without a second authentication transcript or certificate-to-`DeviceId` mapping;
- ordinary capability advertisement, control request/response/event behavior through the existing `SimNode` path over Quinn;
- operation-bound data streams through the existing `SimStreamRuntime` path over Quinn;
- reconnect using a fresh Quinn connection, exporter binding, nonces/proofs, `SessionId`, sequence state, capability state, and operation authority;
- replayed old proofs and old operation/session authority rejected on reconnect;
- accepted signed peer revocation terminating active authority and preventing a fresh Cross-Lab session from reaching `Active`;
- bounded saturation, cancellation, active-stream shutdown, and task joining.

No Quinn/Tokio/rustls/socket/TLS-certificate type entered the public/domain state of `crosslab-core`, `crosslab-protocol`, `crosslab-policy`, or `crosslab-identity`. Quinn/TLS protects the connection but does not define Cross-Lab identity or policy authority.

## M8 Resource Defaults

`QuicTransportConfig::default()` uses explicit nonzero bounds:

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

M8 production baseline:

- Quinn `0.11.11`, pinned with only `runtime-tokio` + `rustls-ring`;
- Tokio `1.53.1`;
- rustls `0.23.44` only where concrete loopback trust construction requires its public types;
- rcgen `0.14.10` dev-only for ephemeral loopback certificates.

The uploaded Quinn repository was inspected as research/reference material, and behavior relevant to the adapter was reconciled against the pinned `quinn-0.11.11` API. No Iroh or rust-libp2p dependency entered M8.

`.github/workflows/fuzz.yml` was not applicable to M8 because M8 did not change its watched protocol/policy/fuzz paths; no fuzz pass is claimed.

## Intentional M8 Limits

M8 does not implement discovery, NAT traversal, relay selection, Iroh, rust-libp2p, route scoring/migration, TCP/TLS fallback, datagram consumers, production certificate provisioning, persistence, UI/platform adapters, privileged services, or installable Linux/Android applications.

Those exclusions are deliberate milestone boundaries, not missing M8 requirements.

## Exact Next Development Task

Begin **M9 — Remote Networking ADR**.

Per the Master Architecture, M9 must prototype and measure Iroh and, where justified, rust-libp2p approaches against the verified Quinn baseline, then select the remote connectivity/NAT/relay architecture through an ADR.

Start M9 by:

1. reading the Master Architecture, M8 design/ADR, and current transport/session contracts;
2. inspecting the uploaded Quinn, Iroh, and rust-libp2p research repositories, including versions, licenses, platform/runtime constraints, NAT traversal/relay behavior, identity coupling, resource model, and maintenance surface;
3. defining explicit evaluation criteria and benchmark/scenario coverage before choosing a library or architecture;
4. prototyping only the smallest isolated candidates needed to gather evidence;
5. measuring candidates against the M8 Quinn baseline for connection establishment, direct-path behavior, relay/fallback behavior where testable, reconnect/failure semantics, resource cost, integration complexity, and preservation of Cross-Lab identity/session boundaries;
6. recording the selected remote connectivity architecture in an ADR before introducing production Iroh/libp2p dependencies or remote relay authority.

M9 must not create a second Cross-Lab session/authentication model or make third-party endpoint/peer identifiers authoritative Cross-Lab identity.

## Phase 1 Completion Boundary

Phase 1 is not complete yet. After M9 selects the remote networking architecture, M10 must deliver the first platform vertical slice described by the Master Architecture before Phase 1 can be closed for real-device testing.

## Resume Procedure

1. inspect canonical `main`, active branches/PRs, recent commits/workflows, and this file;
2. read the Master Architecture, active plan, relevant ADRs, and focused transport/session specifications;
3. reconcile documentation with actual code before changing behavior;
4. inspect relevant uploaded research repositories before implementing related systems;
5. work in small verifiable milestones and keep `CURRENT.md` current;
6. preserve architecture boundaries and record material changes through ADRs;
7. merge only exact verified heads and verify canonical `main` after integration.
