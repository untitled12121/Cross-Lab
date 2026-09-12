# Current Development State

This file is the durable resume guide for active Cross-Lab development. Git history, repository contents, and verification results remain factual if this document becomes stale.

## Current Phase

**Phase 1 — Core Simulator**

## Current Milestone

**M5 — Pairing + Authenticated Logical Session Simulator: in progress.**

Verified M1–M4 and M5 Tasks 1–8 are integrated into canonical `main`. Task 9 is the final M5 closeout slice and must start only from the verified integration-record head created by this update.

## Branch State

- `main` — canonical integrated branch through M5 Task 8; Task 8 merge commit `b14f4c4a7b42b22c1c8de23be4c8c518e24dbc34` passed post-merge CI `34660239776`.
- `m5-session` — historical M5 Tasks 1–4 branch; fully contained in `main`, safe to delete.
- `m5-session-auth` — historical M5 Task 5 branch; fully contained in `main`, safe to delete.
- `m5-memory-transport` — historical M5 Task 6 branch; fully contained in `main`, safe to delete.
- `m5-session-state` — historical M5 Task 7 branch; fully contained in `main`, safe to delete.
- `m5-control-sim` — historical M5 Task 8 branch; fully contained in `main`, safe to delete.
- `planning`, `protocol`, and `trust-policy` — historical branches with zero commits ahead of `main`; safe to delete if no human archival preference requires keeping the names.
- Start Task 9 on a fresh short-lived branch from the verified current `main` head.

No temporary `*-red` branches are required for TDD. Failing checkpoints remain ordinary commits on the active implementation branch. Repository history is the durable implementation state.

## Architecture Baseline

- `docs/architecture/MASTER-ARCHITECTURE.md` — Revision 2.1, source of truth.
- `docs/architecture/CORE-SIMULATOR.md` — Phase 1 simulator and milestone specification.
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

Complete, verified, and integrated through PR #10. The implementation includes single-use invitation state, canonical pairing transcript and directional confirmations, bounded pairing bootstrap wire messages, explicit inviter/joiner state machines, credential issuance for the joiner public key, proof of private-key possession, and trust commit only after all pairing checks succeed.

Task 4 final PR head `588366d69dcd89d3a9a4f709fbca833eb263be57` passed CI `34643647466` and fuzz smoke `34643647458`; PR #10 merged at `b54395109b8fe7ed49862f7f8c508659a2fd7a36`; post-merge CI `34643788450` passed.

### Task 5 — session-auth domain and bootstrap wire

Complete, verified, and integrated through PR #11. The implementation includes the 15-field canonical `SessionAuthTranscriptV1`, role-separated direct Ed25519 proofs, deterministic `SessionId`, explicit unverified credential import, bounded pre-session hello/proof messages outside ordinary control traffic, strict profile/length/enum/range/feature validation, frozen transcript/proof/session vectors, and registered canonical digest/proof labels.

Final PR head `8ad7c79eca6e8aa70c887d27d3a66aa713df9d9f` passed CI `34653899890` and fuzz `34653899783`; PR #11 merged at `8cb6f013055a4f05ff599cc0d31adac45d2756b4`; post-merge CI `34654018205` passed; final integration-record head `f2ace2758af626f442d41308799d19806ad46904` passed CI `34654100309`.

### Task 6 — bounded in-memory transport seam

Complete, verified, and integrated through PR #12. The core seam provides opaque channel binding, transport security classification, diagnostic metadata, bounded nonblocking control queues, frame-preserving backpressure errors, close/closed observation, and simulator-owned deterministic paired endpoints/fault injection. No protocol/parser or real-networking surface changed.

Final PR head `9f2fcaf157245b6dd1f1ab1083e9507961cc7e8a` passed CI `34654964311`; PR #12 merged at `32bf75951c0e0e768efac80c4320059e04550483`; post-merge CI `34655051368` passed.

### Task 7 — logical session activation and capability exchange

Complete, verified, and integrated through PR #13. The implementation adds explicit session lifecycle states, fail-closed credential/trust/protocol/required-feature/channel-binding/proof activation gates, authenticated `SessionContext`, zero-initialized directional sequence state, post-`Active` highest-compatible capability intersection, and explicit close/revocation transitions. Capability negotiation remains descriptive session metadata and never creates policy authority.

Exact code head `606f9022c1c4e702f88d1d71cbee907dc26d5b1b` passed CI `34656446809` and fuzz `34656446822`. Documentation-inclusive head `94f418469a24dcd0d9598311a1d20bfda1bbef3a` passed CI `34658806668` and fuzz `34658806735`; PR #13 merged at `e17075245b566522bb4c822cc22b3ee2245fe7a7`; post-merge CI `34658951190` passed; final Task 7 integration-record head `fd3a31acb731ba2f56720c96b0320e3fa685d9db` passed CI `34659038543`.

### Task 8 — sequenced control request/response/event simulator

Complete, verified, and integrated through PR #14.

Implemented:

- focused `crosslab-core` `ControlDispatcher` reusing M4 envelopes, sequence validation, request/response/cancel/event/session-close types, Task 7 `SessionContext`, and the existing policy evaluator;
- exact SessionId/protocol checks plus directional replay/gap/exhaustion rejection;
- bounded outgoing correlation, inbound request state, and nonretryable request-ID history with no silent eviction;
- send-side state/sequence advancement only after successful bounded transport enqueue;
- receiver-side negotiated capability/version/runtime validation plus existing `PolicyState` authorization before request dispatch;
- capability-scoped/system event handling, cancellation, response correlation, capability advertisements, protocol failures, and registered session-close handling;
- simulator `SimNode` composition over the existing bounded transport and logical session;
- fail-closed malformed wire, wrong session/protocol, replay/gap, and bounded-state exhaustion;
- authorization denial remains a rejected operation and does not escalate session authority;
- no M6 data-stream admission, real networking, async runtime, persistence, UI/platform, or privileged behavior.

Verification evidence:

- initial test head `afca30af7234e8da9455b724e3ffb8cf6aa68e25` stopped at rustfmt and is not valid RED evidence;
- valid RED head `1bfaabc763f63b53e378aedf44cd3a1318084058`: CI `34659510366` passed lockfile/rustfmt and failed `cargo check` only because `crosslab_core::ControlDispatchError` and `crosslab_sim::node` did not yet exist;
- exact code head `af5a535e4531a480f85ea91f79503f852f842009` passed CI `34659955291` including lockfile, rustfmt, full workspace check, Clippy with warnings denied, and full tests;
- documentation-inclusive PR head `212212e4a353abf7d4b505dd633c9f95a04e3027` passed CI `34660116960`;
- Task 8 did not modify protocol/parser/fuzz surfaces, so Fuzz Smoke correctly did not path-trigger;
- PR #14 merged with preserved history at `b14f4c4a7b42b22c1c8de23be4c8c518e24dbc34`;
- post-merge canonical `main` CI `34660239776` passed.

## Exact Next Task

Complete **M5 Task 9 — End-to-end scenarios, parser fuzz expansion, and milestone closeout**, following `docs/plans/phase-1/M5-pairing-session-simulator.md` test-first.

Task 9 should:

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

1. inspect `main`, recent commits, branch/PR state, and current workflow results;
2. read `docs/architecture/MASTER-ARCHITECTURE.md`, this file, `docs/plans/phase-1/M5-pairing-session-simulator.md`, and relevant ADRs;
3. reconcile documentation with actual code before editing;
4. branch Task 9 from the verified canonical `main` head and follow the exact next-task checklist above;
5. never rely on chat history or stash as the only copy of incomplete work.
