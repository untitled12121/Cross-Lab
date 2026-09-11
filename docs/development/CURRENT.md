# Current Development State

This file is the durable resume guide for active Cross-Lab development. Git history, repository contents, and verification results remain factual if this document becomes stale.

## Current Phase

**Phase 1 — Core Simulator**

## Current Milestone

**M4 — Protocol: implementation complete and verified on `protocol`; PR #9 is ready for integration.**

M1–M3 are integrated into canonical `main`. Do not begin M5 implementation until PR #9 is integrated and canonical `main` is verified.

## Branch State

- `main` — canonical integrated branch; contains verified M1, M2, and M3.
- `protocol` — completed M4 implementation branch and head of PR #9.
- `planning` — planning/documentation branch; no active implementation belongs here.

## Architecture Baseline

- `docs/architecture/MASTER-ARCHITECTURE.md` — Revision 2.1, source of truth.
- `docs/architecture/CORE-SIMULATOR.md` — Phase 1 simulator and milestone specification.
- `docs/protocol/PROTOCOL-V1.md` — primary protocol v1 specification.
- ADR-0004 — Protocol Buffers for ordinary v1 wire encoding with independent canonical signing transcripts.
- ADR-0007 — accepted v1 event namespace and session-close wire registry.
- Existing identity, trust, policy, and crypto ADR/spec decisions remain authoritative inputs to protocol conversion.

## M4 Complete

M4 establishes the transport-independent protocol domain and bounded parser surface required by the later Core Simulator:

- bounded protocol version/range negotiation with highest-common-version selection;
- bounded supported/required feature negotiation;
- `u32` big-endian bootstrap, normal-control, and data-stream framing limits;
- random 128-bit request/event/stream identifiers and directional control-sequence validation;
- stable typed protocol error codes and bounded safe diagnostics;
- bounded capability advertisements with strict domain/wire conversion;
- control request, response, event, cancellation, retry-class, protocol-error, and session-close domain types;
- validated namespaced event types, explicit capability/system event scope, and reserved `crosslab.system.*` system namespace;
- stable session-close reasons with unknown/unspecified values rejected fail-closed;
- strict protobuf conversion with exact identifier lengths, canonical domain validation, and explicit enum validation;
- data-stream open headers carrying session, stream, operation, capability/version/operation, direction, and stream index bindings;
- established-session control envelopes carrying protocol version, session ID, message sequence, and all M4 body variants;
- tracked public v1 `.proto` schema alongside Rust wire types;
- fixed protobuf/framing golden vectors covering data-stream open, control envelope, system event, and session close;
- parser/compatibility tests for malformed frames, invalid identifiers/enums, missing envelope bodies, sequence replay/gaps, and bounded collections;
- independent `fuzz/` workspace with bounded targets for control-envelope parsing, data-stream header parsing, and capability/operation identifier validation;
- dedicated `Fuzz Smoke` GitHub Actions verification without adding fuzz dependencies to the production workspace.

The only cross-crate M4 change outside `crosslab-protocol` is the narrow `OperationId::from_bytes` constructor needed for strict wire-to-domain conversion. No pairing orchestration, authenticated logical-session runtime, real networking, persistence, UI, platform adapter, plugin, or privileged-service implementation was introduced.

## Protocol Decision Resolved

The previously underspecified event/session-close wire contract is resolved by accepted ADR-0007 and the tracked v1 protobuf schema:

- event envelope body tag `8` carries a 16-byte `event_id`, optional capability scope, validated namespaced `event_type`, and opaque body bytes;
- system events omit capability scope and use the reserved `crosslab.system.*` namespace;
- session-close envelope body tag `11` uses the stable v1 reason registry `Normal=1`, `LocalRequest=2`, `ProtocolError=3`, `AuthenticationLost=4`, `TrustRevoked=5`, `Shutdown=6`; protobuf value `0` and unknown values are rejected;
- optional session-close diagnostics use the existing 512-byte UTF-8 protocol diagnostic bound.

## Verification

Final implementation/test head `064b221923d4b7f130257d625cca2a49e9c7c461` passed normal PR CI run `34588958781` on Rust 1.98.1:

```text
cargo metadata --locked --no-deps --format-version 1
cargo fmt --check
cargo check --workspace --all-targets --all-features
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-features
```

The same head passed `Fuzz Smoke` run `34588958761`, including fuzz-workspace formatting and bounded runs of all three protocol parser/identifier fuzz targets.

A final PR scope review found only protocol-domain/schema/tests, the narrow policy ID reconstruction seam, dependency/lockfile changes for Prost, protocol fuzz tooling, and milestone documentation. No M5 runtime/session/transport work is present.

## Exact Next Task

1. integrate PR #9 into canonical `main`;
2. verify the integrated `main` state and its normal CI result;
3. update this handoff with the M4 merge commit and mark **M5 — Pairing + Authenticated Logical Session Simulator** active;
4. before M5 coding, read `docs/architecture/SESSION-TRANSPORT.md`, pairing/trust specifications, `CORE-SIMULATOR.md` M5 requirements, and current `main` code;
5. design and implement the smallest in-memory transport/session slice without introducing real networking or platform integration.

## Resume Procedure

Before continuing in a new session:

1. inspect `main`, PR #9/integration state, recent commits, branches, and CI;
2. read `docs/architecture/MASTER-ARCHITECTURE.md`;
3. read this file;
4. read `docs/architecture/CORE-SIMULATOR.md`, `docs/architecture/SESSION-TRANSPORT.md`, and relevant pairing/session protocol specifications;
5. reconcile documentation with actual code before coding;
6. continue from the exact next task above.
