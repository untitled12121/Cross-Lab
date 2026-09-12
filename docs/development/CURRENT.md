# Current Development State

This file is the durable resume guide for active Cross-Lab development. Git/code/tests are factual implementation state; this file records the intended handoff.

## Current Phase

**Phase 1 — Core Simulator**

## Current Milestone

**M5 — Pairing + Authenticated Logical Session Simulator: implementation complete and exact-code verified; canonical integration pending.**

M1–M4 and M5 Tasks 1–8 are integrated into canonical `main`. M5 Task 9 is complete on `m5-closeout`, with exact code verification recorded below. PR #15 must be documentation-inclusive verified and merged before M6 begins.

## Branch State

- `main` — canonical branch through verified M5 Task 8; final Task 8 integration-record head `3d7789868431eaae11cd9dee2053c7f6976e0989` passed CI `34661243555`.
- `m5-closeout` — active M5 Task 9 branch; PR #15.
- `m5-session`, `m5-session-auth`, `m5-memory-transport`, `m5-session-state`, `m5-control-sim` — historical M5 branches fully contained in `main`; safe to delete after human convenience checks.
- `planning`, `protocol`, `trust-policy` — historical branches with zero commits ahead of `main`; safe to delete if their names are not wanted for archival navigation.

Deleting merged branch refs does not delete commits or pull-request history. The connected GitHub tool does not expose branch-ref deletion, so branch cleanup is a GitHub UI/CLI action.

## Architecture Baseline

- `docs/architecture/MASTER-ARCHITECTURE.md` — Revision 2.1, source of truth.
- `docs/architecture/CORE-SIMULATOR.md` — Phase 1 simulator specification and milestone staging.
- `docs/architecture/SESSION-TRANSPORT.md` — logical-session and transport contract.
- `docs/architecture/PAIRING-TRUST-REVOCATION.md` — pairing/trust semantics.
- `docs/protocol/PROTOCOL-V1.md` — protocol v1 wire/canonical registry.
- ADR-0003 — single-use pairing secret and directional HMAC confirmations.
- ADR-0004 — Protocol Buffers for ordinary v1 wire encoding with independent canonical signing transcripts.
- ADR-0006 — focused `crosslab-crypto` foundation boundary.
- ADR-0007 — event namespace and session-close wire registry.

## M5 Scope Guardrails

M5 proves pairing, credential/trust commit, bounded in-memory transport, channel-bound authenticated logical sessions, capability exchange, and sequenced control request/response/event simulation. It does **not** add real networking, Quinn/Iroh/libp2p integration, persistence, UI/platform adapters, plugins, privileged services, or M6 authorized data-stream admission. Queues/state are bounded and the simulator uses deterministic synchronous/nonblocking orchestration without a general async runtime.

## M5 Integrated Progress

### Tasks 1–4 — pairing through trust commit

Complete, verified, and integrated through PR #10. Includes single-use invitations, canonical pairing transcript/directional confirmations, pairing bootstrap wire messages, inviter/joiner state machines, public-key credential issuance, joiner proof of private-key possession, and trust commit only after final proof.

Task 4 final head `588366d69dcd89d3a9a4f709fbca833eb263be57` passed CI `34643647466` and Fuzz Smoke `34643647458`; PR #10 merged at `b54395109b8fe7ed49862f7f8c508659a2fd7a36`; post-merge CI `34643788450` passed.

### Task 5 — session-auth domain and bootstrap wire

Complete, verified, and integrated through PR #11. Includes canonical `SessionAuthTranscriptV1`, role-separated direct Ed25519 proofs, deterministic `SessionId`, explicit unverified credential import, strict bounded session-auth bootstrap parsing, golden vectors, and canonical registry documentation.

Final PR head `8ad7c79eca6e8aa70c887d27d3a66aa713df9d9f` passed CI `34653899890` and Fuzz Smoke `34653899783`; PR #11 merged at `8cb6f013055a4f05ff599cc0d31adac45d2756b4`; post-merge CI `34654018205` passed; integration-record head `f2ace2758af626f442d41308799d19806ad46904` passed CI `34654100309`.

### Task 6 — bounded in-memory transport seam

Complete, verified, and integrated through PR #12. Includes opaque channel binding, `InProcessTest` transport security classification, diagnostic metadata, bounded ordered control queues, frame-preserving backpressure errors, deterministic close/disconnect/directional-close behavior, and no real networking or async runtime.

Final PR head `9f2fcaf157245b6dd1f1ab1083e9507961cc7e8a` passed CI `34654964311`; PR #12 merged at `32bf75951c0e0e768efac80c4320059e04550483`; post-merge CI `34655051368` passed.

### Task 7 — logical session activation and capability exchange

Complete, verified, and integrated through PR #13. Includes explicit lifecycle state, credential/trust/protocol/feature/channel-binding/proof gates, authenticated `SessionContext`, fresh directional sequence initialization, post-auth capability intersection, and close/revocation transitions without converting capability advertisement into policy authority.

Exact code head `606f9022c1c4e702f88d1d71cbee907dc26d5b1b` passed CI `34656446809` and Fuzz Smoke `34656446822`; documentation-inclusive head `94f418469a24dcd0d9598311a1d20bfda1bbef3a` passed CI `34658806668` and Fuzz Smoke `34658806735`; PR #13 merged at `e17075245b566522bb4c822cc22b3ee2245fe7a7`; post-merge CI `34658951190` passed; integration-record head `fd3a31acb731ba2f56720c96b0320e3fa685d9db` passed CI `34659038543`.

### Task 8 — sequenced control request/response/event simulator

Complete, verified, and integrated through PR #14. Includes a focused bounded `ControlDispatcher`, exact SessionId/protocol and directional sequence validation, bounded request/correlation/nonretryable history, state advancement only after successful bounded enqueue, negotiated capability/runtime and `PolicyState` authorization before dispatch, response/cancel/event/capability/session-close handling, and `SimNode` composition over the existing transport/session boundaries.

Valid RED head `1bfaabc763f63b53e378aedf44cd3a1318084058` failed only for missing Task 8 APIs after lockfile/rustfmt passed in CI `34659510366`. Exact code head `af5a535e4531a480f85ea91f79503f852f842009` passed CI `34659955291`; documentation-inclusive head `212212e4a353abf7d4b505dd633c9f95a04e3027` passed CI `34660116960`; PR #14 merged at `b14f4c4a7b42b22c1c8de23be4c8c518e24dbc34`; post-merge CI `34660239776` passed; final integration-record head `3d7789868431eaae11cd9dee2053c7f6976e0989` passed CI `34661243555`.

## M5 Task 9 — end-to-end scenarios, parser fuzz expansion, and closeout

Implementation complete and exact-code verified on `m5-closeout`.

Implemented:

- `apps/sim/tests/m5_scenarios.rs` composes the existing production-domain slices instead of adding parallel security logic;
- positive composed path: pairing confirmations -> joiner credential issuance/proof -> trust commit -> fresh memory channel binding -> authenticated logical sessions -> bidirectional capability exchange -> policy-authorized sequenced request/response/event;
- integration negatives for wrong pairing secret, replayed session proof under a fresh nonce, proof bound to another channel, stale locally accepted credential epoch, and owner-mismatched peer trust;
- new decoder-only fuzz targets for `decode_pairing_bootstrap` and `decode_session_auth_bootstrap`;
- both targets registered in `fuzz/Cargo.toml` and added to the existing bounded 256-run Fuzz Smoke workflow;
- final scope diff from verified Task 8 base contains only the composed simulator test plus fuzz manifest/targets/workflow; no production code or M6 functionality changed.

Verification evidence:

- initial scenario head `b70bf41c2f32b8d4812cdfa114264d19b6727354` stopped at rustfmt and is not semantic verification evidence;
- formatted scenario head `3bdd5e46831158a8b48d018630d4c8445a886e67` passed lockfile, rustfmt, workspace check, Clippy with warnings denied, and full tests in CI `34661728554`;
- exact Task 9 code head `8a4ae668d6cbfd329d72914d8b47279263cf14d7` passed full workspace CI `34661829192`;
- the same exact code head passed Fuzz Smoke `34661829233`, including `control_frame`, `data_stream_open`, `identifiers`, `pairing_bootstrap`, and `session_auth`, each bounded to 256 runs;
- compare `3d7789868431eaae11cd9dee2053c7f6976e0989..8a4ae668d6cbfd329d72914d8b47279263cf14d7` contains exactly five Task 9 code/test-infrastructure files and no production/M6 changes.

## Exact Next Task

Finish **M5 canonical integration** before starting M6:

1. verify this documentation-inclusive PR #15 head with the normal full workspace CI; the previously verified `8a4ae668...` fuzz result remains exact for the unchanged fuzz/code surface;
2. update PR #15 with final scope/evidence and mark it ready;
3. merge only the exact verified documentation-inclusive head into `main` with preserved history;
4. verify post-merge canonical `main` CI and the path-triggered Fuzz Smoke result;
5. write the final M5 integration record on `main`, then verify that documentation-only head;
6. only then begin **M6 — Authorized Data Streams**.

## M6 Handoff

Per `docs/architecture/CORE-SIMULATOR.md`, M6 delivers operation-bound stream admission, bounded data queues/chunks, backpressure, cancellation, and single/multistream use semantics needed by tests. The existing M4 `DataStreamOpen` protocol header and M3 authorized-operation lifecycle should be reused rather than replaced.

Before implementing M6, create/read an approved M6 implementation plan derived from the Master Architecture and `CORE-SIMULATOR.md`, inspect the existing operation/data-stream contracts and relevant research repositories, and branch from the verified final M5 `main` head. Do not introduce real networking; Quinn remains M8 after the in-memory simulator exit criteria.

## Resume Procedure

Before continuing in a new chat:

1. inspect `main`, recent commits, branch/PR state, and workflow results;
2. read `docs/architecture/MASTER-ARCHITECTURE.md`, this file, `docs/architecture/CORE-SIMULATOR.md`, and the active milestone plan/ADRs;
3. reconcile documentation with actual code before editing;
4. complete any integration step listed above before starting the next milestone;
5. never rely on chat history or stash as the only copy of incomplete work.
