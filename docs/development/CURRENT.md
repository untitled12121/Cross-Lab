# Current Development State

This file is the durable resume guide for active Cross-Lab development. Git history, repository contents, and verification results remain factual if this document becomes stale.

## Current Phase

**Phase 1 — Core Simulator**

## Current Milestone

**M4 — Protocol: active on `protocol` in PR #9.**

M1–M3 are integrated into canonical `main`. M4 is not complete or ready to merge yet.

## Branch State

- `main` — canonical integrated branch; contains verified M1, M2, and M3.
- `protocol` — active M4 implementation branch and head of PR #9.
- `planning` — planning/documentation branch; no active implementation belongs here.

Do not begin M5 implementation until M4 is complete, verified, and integrated into `main`.

## Architecture Baseline

- `docs/architecture/MASTER-ARCHITECTURE.md` — Revision 2.1, source of truth.
- `docs/architecture/CORE-SIMULATOR.md` — Phase 1 simulator and milestone specification.
- `docs/protocol/PROTOCOL-V1.md` — primary M4 protocol specification.
- ADR-0004 — Protocol Buffers for ordinary v1 wire encoding with independent canonical signing transcripts.
- Existing identity, trust, policy, and crypto ADR/spec decisions remain authoritative inputs to protocol conversion.

## M4 Completed So Far

The active branch now contains test-first protocol-domain and wire-contract slices for:

- bounded protocol version/range negotiation;
- bounded supported/required feature negotiation;
- `u32` big-endian bootstrap, normal-control, and data-stream framing limits;
- random 128-bit protocol identifiers and directional control sequence validation;
- stable typed protocol error codes and bounded safe diagnostics;
- bounded capability advertisements with strict domain/wire conversion;
- control request, response, cancellation, retry-class, and typed failure domains;
- strict protobuf conversion for request/response/cancellation, including exact identifier lengths and fail-closed enum handling;
- data-stream open headers with exact session/stream/operation identifiers and bounded framing;
- established-session control envelopes carrying protocol version, session ID, message sequence, and implemented body types;
- tracked v1 `.proto` schema alongside Rust wire types;
- fixed protobuf/framing golden vectors for data-stream open and control-envelope messages;
- independent `fuzz/` workspace with bounded smoke targets for control-frame parsing, data-stream header parsing, and capability/operation identifier validation;
- a dedicated `Fuzz Smoke` GitHub Actions job using nightly Rust and pinned cargo-fuzz/libFuzzer tooling without adding fuzz dependencies to the production workspace.

No pairing orchestration, authenticated logical-session runtime, real networking, persistence, UI, platform adapter, plugin, or privileged-service implementation has been introduced by M4.

## M4 Protocol Detail Requiring Resolution

`PROTOCOL-V1.md` requires both `event` and `session_close` envelope bodies, but it does not currently define enough stable wire detail to implement them without making a new public protocol decision:

- events require an `event_type` that is a validated typed value, but the v1 scalar representation/registry and system-event namespace representation are not assigned;
- session close requires a typed close reason, but the v1 close-reason registry is not assigned.

These choices affect public protocol compatibility. Do not silently invent them in code. Resolve and record the v1 representation/registry before adding the missing envelope variants.

## Verification

Implementation head `58abdc12efcd7e44f0917ddc435ae38ad9b3eaea` passed the normal PR CI baseline in run `34583107942`:

```text
cargo metadata --locked --no-deps --format-version 1
cargo fmt --check
cargo check --workspace --all-targets --all-features
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-features
```

The same head passed `Fuzz Smoke` run `34583108051`, including formatting of the independent fuzz workspace and bounded runs of all three current parser/identifier fuzz targets.

## Exact Next Task

1. approve the minimal v1 wire representation/registry for event type/system-event namespace and session-close reason;
2. implement `Event` and `SessionClose` domain types test-first;
3. add protobuf schema fields at the existing envelope body positions and strict wire/domain conversion;
4. add negative compatibility/parser coverage for the new values without adding M5 session orchestration;
5. run the full normal CI and fuzz-smoke verification;
6. review M4 against `PROTOCOL-V1.md` and `CORE-SIMULATOR.md`, update this handoff, then integrate PR #9 into `main` only when the milestone is complete.

After M4 integration, the next milestone is **M5 — Pairing + Authenticated Logical Session Simulator**.

## Resume Procedure

Before continuing in a new session:

1. inspect `main`, `protocol`, PR #9, recent commits, and CI state;
2. read `docs/architecture/MASTER-ARCHITECTURE.md`;
3. read this file;
4. read `docs/protocol/PROTOCOL-V1.md` and the M4 sections of `docs/architecture/CORE-SIMULATOR.md`;
5. reconcile documentation with the actual branch before coding;
6. continue from the exact next task above.
