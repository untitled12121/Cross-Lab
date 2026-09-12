# Current Development State

This file is the durable resume guide for active Cross-Lab development. Git history, repository contents, and verification results remain factual if this document becomes stale.

## Current Phase

**Phase 1 — Core Simulator**

## Current Milestone

**M5 — Pairing + Authenticated Logical Session Simulator: in progress.**

M1–M4 and M5 Tasks 1–7 are verified and integrated into canonical `main`. M5 Task 8 is implemented and exact-code-head verified on `m5-control-sim`; integration into `main` is the current handoff step. Task 9 is the final M5 closeout slice and must not begin until Task 8 is merged and canonical `main` is reverified.

## Branch State

- `main` — canonical integrated branch through M5 Task 7; verified integration-record head before Task 8: `fd3a31acb731ba2f56720c96b0320e3fa685d9db`.
- `m5-session` — historical M5 Tasks 1–4 branch; PR #10 merged.
- `m5-session-auth` — historical M5 Task 5 branch; PR #11 merged.
- `m5-memory-transport` — historical M5 Task 6 branch; PR #12 merged.
- `m5-session-state` — historical M5 Task 7 branch; PR #13 merged.
- `m5-control-sim` — active M5 Task 8 branch; PR #14 open pending documentation-inclusive verification and integration.
- `planning` — planning/documentation branch; no active implementation belongs here.

No temporary `*-red` branches are required for TDD. Failing contract-test checkpoints remain ordinary commits on the active implementation branch. The connected GitHub workflow writes directly to committed branches, so repository history is the durable implementation state.

## Architecture Baseline

- `docs/architecture/MASTER-ARCHITECTURE.md` — Revision 2.1, source of truth.
- `docs/architecture/CORE-SIMULATOR.md` — Phase 1 simulator/milestone specification.
- `docs/architecture/SESSION-TRANSPORT.md` — logical-session and transport contract.
- `docs/architecture/PAIRING-TRUST-REVOCATION.md` — pairing/trust semantics.
- `docs/protocol/PROTOCOL-V1.md` — protocol v1 wire/canonical registry.
- ADR-0003 — single-use pairing secret and directional HMAC confirmations.
- ADR-0004 — Protocol Buffers for ordinary v1 wire encoding with independent canonical signing transcripts.
- ADR-0006 — focused `crosslab-crypto` foundation boundary.
- ADR-0007 — event namespace and session-close wire registry.

## M5 Scope Guardrails

M5 contains pairing, credential/trust commit, bounded in-memory transport, authenticated logical sessions, capability exchange, and sequenced control request/response/event simulation. It does **not** add real networking, Quinn, persistence, UI/platform adapters, M6 data-stream admission, plugins, or privileged services. Deterministic state machines and bounded queues remain synchronous/nonblocking; no general async runtime is introduced.

## Integrated M5 Progress

### Tasks 1–4 — pairing through trust commit

Complete, verified, and integrated through PR #10. The integrated implementation includes single-use invitation state, canonical pairing transcript and directional confirmations, bounded pairing bootstrap wire messages, explicit inviter/joiner state machines, credential issuance for the joiner public key, proof of private-key possession, and trust commit only after all pairing checks succeed. Security coverage includes replay, wrong-secret, nonce/pairing-ID/device-key substitution, invalid proofs/delegation/owner, cancelled/consumed invitations, and frozen canonical vectors.

Task 4 final PR head `588366d69dcd89d3a9a4f709fbca833eb263be57` passed CI `34643647466` and fuzz smoke `34643647458`; PR #10 merged at `b54395109b8fe7ed49862f7f8c508659a2fd7a36`; post-merge CI `34643788450` passed.

### Task 5 — session-auth domain and bootstrap wire

Complete, verified, and integrated through PR #11. The implementation includes the 15-field canonical `SessionAuthTranscriptV1`, role-separated direct Ed25519 proofs, deterministic `SessionId`, explicit unverified credential import, bounded pre-session hello/proof messages outside `EnvelopeV1`, strict profile/length/enum/range/feature validation, frozen transcript/proof/session vectors, and registered canonical digest/proof labels.

Valid RED head `c37f598522b6aa114bd3f62c32a5590e96c1e7db` failed only on intentionally missing APIs after lockfile/rustfmt. Final PR head `8ad7c79eca6e8aa70c887d27d3a66aa713df9d9f` passed CI `34653899890` and fuzz `34653899783`; PR #11 merged at `8cb6f013055a4f05ff599cc0d31adac45d2756b4`; post-merge CI `34654018205` passed; final integration-record head `f2ace2758af626f442d41308799d19806ad46904` passed CI `34654100309`.

### Task 6 — bounded in-memory transport seam

Complete, verified, and integrated through PR #12. The core transport seam provides opaque channel binding, transport security classification, diagnostic metadata, bounded nonblocking control queues, frame-preserving backpressure errors, close/closed observation, and simulator-owned deterministic paired endpoints/fault injection. No protocol/parser or real-networking surface changed.

Valid RED head `e27a2c115b635986a7193559aeb7ab5c8898f309` failed on the intentionally absent Task 6 exports. Final PR head `9f2fcaf157245b6dd1f1ab1083e9507961cc7e8a` passed CI `34654964311`; PR #12 merged at `32bf75951c0e0e768efac80c4320059e04550483`; post-merge CI `34655051368` passed.

### Task 7 — logical session activation and capability exchange

Complete, verified, and integrated through PR #13. The implementation adds explicit session lifecycle states, fail-closed credential/trust/protocol/required-feature/channel-binding/proof activation gates, authenticated `SessionContext`, deterministic zero-initialized directional sequence state, post-`Active` highest-compatible capability intersection, and explicit close/revocation transitions. Capability negotiation remains descriptive session metadata and never creates policy authority.

Valid RED head `3a7af8ad57af1ecec2ae52753f80498e0c1a105c` failed only because Task 7 APIs were intentionally absent. Exact code head `606f9022c1c4e702f88d1d71cbee907dc26d5b1b` passed CI `34656446809` and fuzz `34656446822`. Documentation-inclusive head `94f418469a24dcd0d9598311a1d20bfda1bbef3a` passed CI `34658806668` and fuzz `34658806735`; PR #13 merged at `e17075245b566522bb4c822cc22b3ee2245fe7a7`; post-merge CI `34658951190` passed; final Task 7 integration-record head `fd3a31acb731ba2f56720c96b0320e3fa685d9db` passed CI `34659038543`.

## Task 8 — Sequenced control request/response/event simulator

**Complete and exact-code-head verified; integration pending.**

Implemented:

- focused `crosslab-core` `ControlDispatcher` that reuses M4 `ControlEnvelope`, `ControlSequence`, request/response/cancel/event/session-close types, Task 7 `SessionContext`, and the existing policy evaluator instead of duplicating protocol or authorization logic;
- exact session-ID and negotiated-protocol checks before ordinary control dispatch;
- independent directional sequence validation with replay/gap/exhaustion rejection;
- bounded outgoing request correlation, inbound request state, and nonretryable request-ID history using ordered maps/sets with explicit capacity limits;
- no silent eviction of nonretryable history; capacity exhaustion returns a typed `ResourceLimit` error;
- send-side dispatcher state and sequence advance only after the encoded frame successfully enters the bounded transport queue, preserving retry/backpressure correctness;
- receiver-side request admission checks the negotiated capability/version, local runtime capability, and existing `PolicyState`; `Deny` and `Ask` never reach capability dispatch and never mint authority;
- capability-scoped events require a negotiated capability while reserved system events remain protocol-level events;
- response correlation and cancellation semantics use `RequestId` and reject unknown/late responses;
- `SimNode` composition over the existing `MemoryTransportPair`, logical session, dispatcher, local capability set, and policy state;
- malformed wire, wrong session/protocol, sequence replay/gap, or bounded-state exhaustion fail the simulated connection closed; authorization denial remains a normal rejected operation while the authenticated session stays active;
- registered `SessionClose` handling closes the logical session and transport explicitly;
- capability advertisements are dispatched only inside an already active session and update negotiated session metadata through the existing Task 7 API;
- no M6 data-stream admission, real networking, async runtime, or privileged/platform behavior.

The plan listed `apps/sim/src/main.rs` as a possible modification. No binary edit is required: Task 8 composition is exposed through the simulator library and exercised by integration tests; the empty binary has no additional behavior to own. This intentionally keeps the slice smaller without altering architecture.

Task 8 integration tests cover:

- S-006 authorized request -> correlated response -> permitted event with exact sequencing;
- capability-advertisement dispatch after authentication;
- duplicate and gap sequence rejection with fail-closed state;
- duplicate nonretryable request-ID rejection after completion;
- unauthorized request rejection without handler dispatch or session authority escalation;
- cancellation and late-response rejection;
- malformed control frame, invalid `SessionId`, and registered session-close behavior.

TDD and verification evidence:

- initial test head `afca30af7234e8da9455b724e3ffb8cf6aa68e25` stopped at rustfmt and is not counted as valid RED evidence;
- valid RED head `1bfaabc763f63b53e378aedf44cd3a1318084058`: CI `34659510366` passed lockfile/rustfmt and failed `cargo check` specifically because `crosslab_core::ControlDispatchError` and `crosslab_sim::node` did not yet exist;
- implementation head `d323771c98a0a2d8759a0bc295b7d039526f7144` initially required only rustfmt cleanup;
- final exact code head `af5a535e4531a480f85ea91f79503f852f842009` passed CI `34659955291`: lockfile, rustfmt, full workspace check, Clippy with warnings denied, and full workspace tests all passed;
- Task 8 does not modify `crates/protocol/**`, `crates/policy/**`, `fuzz/**`, or the fuzz workflow, so the protocol-parser fuzz workflow correctly did not path-trigger. Task 9 explicitly expands and reruns the relevant parser fuzz surface.

Scope diff from verified Task 7 base `fd3a31acb731ba2f56720c96b0320e3fa685d9db` to Task 8 code head is limited to:

- `apps/sim/Cargo.toml`;
- `apps/sim/src/lib.rs`;
- `apps/sim/src/node.rs`;
- `apps/sim/tests/session_scenarios.rs`;
- `crates/core/src/control/mod.rs`;
- `crates/core/src/lib.rs`.

## Exact Next Task

First finish **Task 8 integration**:

1. verify this documentation-inclusive PR #14 head with CI;
2. update PR #14 to the completed scope/evidence, mark ready, and merge only the verified head into `main` preserving history;
3. run post-merge canonical `main` CI;
4. record Task 8 as integrated in this file on `main` and verify that integration-record head.

Then start a fresh Task 9 branch from that verified `main` head and complete **M5 Task 9 — End-to-end scenarios, parser fuzz expansion, and milestone closeout**:

1. add `apps/sim/tests/m5_scenarios.rs` covering the composed M5 path: pairing/trust -> fresh channel-bound authenticated logical session -> capability exchange -> sequenced authorized control request/response/event;
2. add integration negatives for wrong pairing secret, old proof under fresh nonce, wrong channel binding, stale accepted credential epoch, and owner mismatch using existing APIs rather than parallel security logic;
3. add `fuzz/fuzz_targets/pairing_bootstrap.rs` and `fuzz/fuzz_targets/session_auth.rs` using the existing strict bootstrap decoders;
4. register both targets in `fuzz/Cargo.toml` and add bounded 256-run smoke commands in `.github/workflows/fuzz.yml`;
5. run exact-head lockfile/rustfmt/workspace check/Clippy/full tests and Fuzz Smoke;
6. perform a final M5 scope diff confirming no M6 data streams, real networking, UI, persistence, or privileged behavior entered the milestone;
7. update this file to **M5 complete** with exact verification evidence and **M6 — Authorized Data Streams** as the next milestone;
8. verify the documentation-inclusive Task 9 PR head, merge that exact head into `main`, run post-merge canonical CI, then write and verify the final M5 integration record on `main`.

## Resume Procedure

Before continuing in a new chat:

1. inspect `main`, recent commits, active branches/PRs, and current workflow results;
2. read `docs/architecture/MASTER-ARCHITECTURE.md`, this file, and `docs/plans/phase-1/M5-pairing-session-simulator.md`;
3. reconcile documentation with code before editing;
4. if PR #14 is not merged, finish Task 8 integration exactly as described above; otherwise begin Task 9 from the verified canonical `main` head;
5. never rely on chat history or stash as the only copy of incomplete work.
