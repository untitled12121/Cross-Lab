# Current Development State

This file is the durable resume guide for active Cross-Lab development. Git/code/tests are factual implementation state; this file records the intended handoff.

## Current Phase

**Phase 1 — Core Simulator / Real Transport Transition**

## Current Milestone

**M8 — Quinn Transport: Tasks 1–4 are GREEN; Task 5 bounded unidirectional stream bridge is next.**

M1–M7 are complete and integrated into canonical `main`. M8 remains isolated on `m8-quinn-transport` and is not yet ready to merge.

## Canonical Main State

- Canonical `main` documentation head before M8: `b2e16f27a5c6d8307c58f4c6a4c760bb867ff223` (`docs: record M7 integration`).
- Post-M7 canonical CI: `34681661930` — passed.
- M8 branch was created from that exact verified head.

## M8 Planning and Architecture

Approved/committed planning artifacts:

- `docs/plans/phase-1/M8-quinn-transport-design.md`;
- `docs/adr/ADR-0008-quinn-channel-binding-profile-v1.md`;
- `docs/plans/phase-1/M8-quinn-transport.md`.

M8 preserves these boundaries:

- `crosslab-core` remains runtime/transport implementation neutral;
- Quinn, Tokio, rustls, socket, endpoint, and TLS certificate types stay inside `transports/quic`;
- Quinn/TLS provides confidentiality/integrity and channel-binding material, not Cross-Lab device identity authority;
- the accepted binding profile is `quic-tls-exporter-v1`, 32 bytes, label `EXPORTER-Cross-Lab-QUIC-Channel-Binding-v1`, context `crosslab.quic.transport.v1`;
- no `Connecting::into_0rtt` or 0-RTT authority is allowed;
- reconnect means a fresh protected connection and therefore fresh binding/session/authentication/authorization state;
- all application queues, record sizes, stream concurrency, windows, cancellation, and shutdown behavior remain explicitly bounded;
- no discovery, NAT traversal, relay, Iroh, libp2p, datagram consumer, route migration, persistence, UI/platform, privileged-service, or production certificate-provisioning scope enters M8.

## M8 Dependency and Research State

Production dependency baseline:

- Quinn `0.11.11`, pinned with only `runtime-tokio` + `rustls-ring`;
- Tokio `1.53.1`;
- rustls `0.23.44` where concrete loopback trust construction requires it;
- rcgen `0.14.10` dev-only for ephemeral loopback certificate fixtures.

The uploaded Quinn repository has been inspected. It is Quinn `main` at package version `0.12.0`; it is research/reference material only. Production remains pinned to separately verified published stable Quinn `0.11.11`. Relevant stream and connection behavior was reconciled against that source without copying its architecture.

## M8 Completed Work

### Task 1 — Transport-neutral outbound bounds

Implemented ownership-preserving `TooLarge` errors for control and stream opening frames, explicit memory-transport frame limits, rejection before capacity consumption, and simulator handling that does not misclassify oversize as transport loss.

GREEN checkpoint:

- head `691aa11f2c7a101a1cb65e1999e6da4a1fee2f4d`;
- CI `34684023292` — lockfile, rustfmt, workspace check, Clippy `-D warnings`, and all tests passed.

### Task 2 — Quinn crate and TLS-exporter binding

Implemented the isolated `crosslab-transport-quic` crate, exact dependency pins, private ADR-0008 TLS-exporter derivation, and real loopback proof that both peers derive the same binding while a fresh connection derives a different one.

GREEN checkpoint:

- head `a032189481fee095bdf683535b982972188469c1`;
- CI `34684420001` — all required gates passed.

### Task 3 — Bounded private record framing

Implemented private `u32` big-endian stream records with length validation before allocation, zero-length rejection where forbidden, truncated-body failure, exact-limit success, and no attacker-controlled `read_to_end`.

The original test helper deadlocked because Quinn bidirectional streams are lazy; the uploaded Quinn source confirmed the peer cannot `accept_bi()` until the opener writes. The test helper was corrected without changing production framing.

GREEN checkpoint:

- head `fa1d35ca370b5ca75d6c55b41c21a56b4a0f2b75`;
- CI `34684781173` — all required gates passed.

### Task 4 — Typed config and bounded control bridge

Implemented:

- public typed `QuicTransportConfig` with nonzero bounds for control/stream queues, frame/chunk sizes, concurrent remote streams, receive windows, and idle timeout;
- `QuicTransportConnection` implementing the transport-neutral `TransportConnection` contract;
- one bounded Tokio outbound control queue and one bounded inbound control queue;
- a single control writer preserving ordered records;
- a control reader that awaits inbound capacity so application backpressure propagates instead of reading indefinitely;
- a shared terminal state with a watch signal used by connection-owned tasks;
- a Quinn close monitor that maps remote connection loss into terminal transport state;
- explicit local close and async `shutdown()` that closes the QUIC connection and joins all owned control/monitor tasks;
- ownership-preserving `Full`, `TooLarge`, and `Closed` control behavior;
- active stream methods remain bounded placeholders until Task 5 (`TooLarge`/`Full`/`Empty`, terminal -> `Closed`).

Real loopback tests prove ordered delivery, deterministic bounded local saturation, oversize rejection without capacity consumption, remote-close terminal propagation, and explicit shutdown/task completion.

GREEN checkpoint:

- head `5da78c5e0182704b84ffeeb63b5e7f57d7b47257`;
- CI `34685447091` — `cargo metadata --locked`, `cargo fmt --check`, workspace check, Clippy `-D warnings`, and the full workspace test suite passed; `crosslab-transport-quic` ran 9/9 tests successfully.

The concrete constructor remains crate-private because making it public would leak Quinn stream/connection types across the adapter boundary. Until a production endpoint/bootstrap orchestration path consumes that constructor, narrow non-test dead-code allowances remain on the staged private binding/connection/record modules rather than weakening the architecture.

## Active Pull Request

- Draft PR #18: `feat: implement M8 Quinn transport`.
- Base: `main` at the original verified M8 baseline.
- Head branch: `m8-quinn-transport`.
- Do not merge until the entire M8 milestone is complete, reviewed, and the exact final head passes all required gates.

## Remaining M8 Work

The approved implementation plan still requires:

1. Task 5 — bounded authorized unidirectional stream bridge with opening-frame/chunk limits, stream-slot saturation, FIN/reset/stop/cancellation semantics, and terminal propagation;
2. Task 6 — prove the existing Cross-Lab session-auth protocol over the Quinn exporter binding without creating a second authentication architecture;
3. remaining reconnect/revocation/failure/shutdown integration scenarios from the M8 plan;
4. final dependency/boundary/security review and full workspace verification;
5. update architecture/docs for completed M8 factual state;
6. only then merge the exact verified M8 head to `main` and verify canonical `main` after merge.

## Exact Next Task

Start **Task 5 RED** from `docs/plans/phase-1/M8-quinn-transport.md`.

Write real loopback tests first for the unidirectional stream bridge proving at minimum:

- opening-frame delivery followed by ordered payload chunks;
- bounded outgoing stream-slot saturation with opening-frame ownership preservation;
- opening-frame and chunk oversize rejection before queue/stream allocation;
- bounded chunk backpressure with ownership preservation;
- graceful sender finish maps to receiver `Finished` after queued chunks drain;
- sender cancellation/reset maps to receiver `Cancelled`;
- receiver cancellation/stop causes later sender work to fail closed;
- connection loss cancels pending and active stream authority and future open/accept calls fail closed.

Verify the RED failure before implementing the minimal stream bridge.

## Resume Procedure

1. inspect canonical `main`, `m8-quinn-transport`, PR #18, recent commits/workflows, and this file;
2. read the Master Architecture, M8 design, ADR-0008, active implementation plan, and relevant transport/session contracts;
3. inspect relevant existing Cross-Lab code and uploaded research source before implementing related behavior;
4. execute small RED → verified failure → GREEN milestones;
5. after meaningful progress, run the full gate, commit/push, and update this file;
6. do not rely on chat history or stash as the only copy of valuable work;
7. merge only the exact verified completed M8 head and verify canonical `main` after merge.
