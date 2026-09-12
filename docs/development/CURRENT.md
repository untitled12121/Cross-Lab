# Current Development State

This file is the durable resume guide for active Cross-Lab development. Git/code/tests are factual implementation state; this file records the intended handoff.

## Current Phase

**Phase 1 — Core Simulator / Real Transport Transition**

## Current Milestone

**M8 — Quinn Transport: Tasks 1–5 are GREEN; Task 6 session authentication over Quinn is next.**

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

The uploaded Quinn repository has been inspected as research/reference material. Production remains pinned to Quinn `0.11.11`. Task 5 additionally reconciled the exact `quinn-0.11.11` tag behavior for `SendStream::reset`, `SendStream::stopped`, `RecvStream::stop`, and `ReadExactError`, so FIN/reset/STOP mapping targets the pinned production API rather than Quinn `main`. Architecture was not copied from Quinn.

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

Implemented typed nonzero QUIC configuration, `QuicTransportConnection`, bounded ordered control queues, terminal connection state, remote-close monitoring, ownership-preserving control backpressure, and explicit shutdown that closes Quinn and joins connection-owned tasks.

GREEN checkpoint:

- head `5da78c5e0182704b84ffeeb63b5e7f57d7b47257`;
- CI `34685447091` — locked metadata, rustfmt, workspace check, Clippy `-D warnings`, and the full workspace test suite passed; the QUIC crate passed 9/9 tests.

### Task 5 — Bounded unidirectional data-stream bridge

Implemented:

- synchronous outgoing stream-slot reservation with an owned semaphore permit;
- bounded per-stream outbound chunk queues with ownership-preserving `Full`, `TooLarge`, and `Closed` outcomes;
- asynchronous Quinn `open_uni()` drivers that write one bounded opening record followed by ordered bounded chunk records;
- a bounded inbound accept queue and explicit concurrent-remote-stream semaphore;
- opening-record validation before an `IncomingUniStream` becomes visible to the application;
- bounded inbound chunk queues with application backpressure propagated to the Quinn reader task;
- graceful FIN mapped to `StreamReceiveError::Finished` only after queued chunks drain;
- sender `cancel()` and sender drop mapped to Quinn RESET and receiver `Cancelled`;
- receiver `cancel()`/drop mapped to Quinn STOP, observed by the sender through `SendStream::stopped()` so later sends fail closed;
- connection terminal state cancelling active send/receive stream authority and rejecting future open/accept operations;
- a connection-owned task registry that atomically stops accepting child stream tasks before shutdown drains and joins all owned task handles;
- private record framing now distinguishes clean FIN (`ReadExactError::FinishedEarly(0)` while reading a new record prefix) from reset/truncation without weakening existing framing bounds.

The Task 5 RED checkpoint was commit `8dc278bde3b560fdce2356f1a2c5e8636217f322`; CI `34685655519` passed lockfile/format/check/Clippy and failed all eight new stream tests specifically because the old placeholder returned `StreamOpenError::Full` for every valid open.

GREEN checkpoint:

- latest verified implementation head before this documentation-only refresh: `cd7ac6ca71bbc253ffdeee9584aa886dcca15d62`;
- CI `34686263644` — `cargo metadata --locked`, `cargo fmt --check`, workspace check, Clippy `-D warnings`, and `cargo test --workspace --all-features` all passed;
- `crosslab-transport-quic` passed 17/17 tests, including all eight Task 5 real-loopback scenarios for ordered chunks, opening/stream-slot bounds, chunk bounds/backpressure, FIN, RESET/drop, STOP, and connection-close cancellation.

The concrete Quinn constructor remains crate-private because making it public would leak Quinn types across the adapter boundary. Narrow staged non-test dead-code allowances remain only where production endpoint/bootstrap orchestration has not yet consumed private adapter helpers.

## Active Pull Request

- Draft PR #18: `feat: implement M8 Quinn transport`.
- Base: `main` at the original verified M8 baseline.
- Head branch: `m8-quinn-transport`.
- Do not merge until the entire M8 milestone is complete, reviewed, and the exact final head passes all required gates.

## Remaining M8 Work

The approved implementation plan still requires:

1. Task 6 — prove the existing Cross-Lab session-auth protocol over the Quinn exporter binding without creating a second authentication architecture;
2. Task 7 — prove control, authorized streams, reconnect, revocation, failure, saturation, cancellation, and shutdown integration scenarios over Quinn;
3. Task 8 — final dependency/boundary/security review, architecture reconciliation, full workspace verification, and durable M9 handoff;
4. only then merge the exact verified M8 head to `main` and verify canonical `main` after merge.

## Exact Next Task

Start **Task 6 RED** from `docs/plans/phase-1/M8-quinn-transport.md`.

Use the existing production session-auth domain APIs and bounded bootstrap wire format. Test-only Quinn orchestration must exchange the existing `SessionAuthHello` and role-separated proof messages over the same reserved bidirectional control stream, activate ordinary `LogicalSession` state using the Quinn TLS-exporter `ChannelBinding`, then promote that same stream into `QuicTransportConnection`.

RED coverage must prove at minimum:

- trusted peers can activate with `AuthenticatedConfidentialChannel` over the real Quinn exporter binding;
- a proof bound to the wrong connection binding is rejected before `Active`;
- a proof from an old connection cannot be replayed after reconnect;
- ordinary authenticated control traffic is not accepted before session authentication completes.

Do not introduce a Quinn certificate-to-`DeviceId` mapping, a second authentication transcript, or an adapter-specific authorization bypass.

## Resume Procedure

1. inspect canonical `main`, `m8-quinn-transport`, PR #18, recent commits/workflows, and this file;
2. read the Master Architecture, M8 design, ADR-0008, active implementation plan, and relevant transport/session contracts;
3. inspect relevant existing Cross-Lab code and uploaded research source before implementing related behavior;
4. execute small RED → verified failure → GREEN milestones;
5. after meaningful progress, run the full gate, commit/push, and update this file;
6. do not rely on chat history or stash as the only copy of valuable work;
7. merge only the exact verified completed M8 head and verify canonical `main` after merge.
