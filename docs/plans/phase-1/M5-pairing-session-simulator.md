# M5 Pairing + Authenticated Logical Session Simulator Implementation Plan

> **For agentic workers:** execute this plan task-by-task with TDD. Keep all M5 work on one short-lived implementation branch; do not create temporary red-test branches.

**Goal:** Prove Cross-Lab pairing and authenticated logical-session semantics end-to-end over a deterministic bounded in-memory transport without introducing real networking or platform code.

**Architecture:** `crosslab-crypto` owns narrow HMAC-SHA-256 mechanics; `crosslab-protocol` owns bounded bootstrap wire messages and strict conversions; `crosslab-core` owns pairing/session orchestration and the transport-neutral state machines; `crosslab-sim` owns the deterministic `InProcessTest` transport and scenarios. Ordinary control traffic uses the already-implemented M4 envelope/protocol types after authentication.

**Tech stack:** Rust 2024, Rust 1.98.1, existing Ed25519/BLAKE3/Prost stack, RustCrypto `hmac = 0.13.0` and `sha2 = 0.11.0` for ADR-0003 HMAC-SHA-256. Both are MIT OR Apache-2.0 and support the pinned toolchain.

**Specs:**
- `docs/architecture/CORE-SIMULATOR.md`
- `docs/architecture/SESSION-TRANSPORT.md`
- `docs/architecture/PAIRING-TRUST-REVOCATION.md`
- `docs/protocol/PROTOCOL-V1.md`
- `docs/adr/ADR-0003-pairing-bootstrap-profile-v1.md`

## Global Constraints

- Identity remains independent of transport identity.
- Pairing uses one single-use 256-bit secret delivered out of band; it is never sent over the provisional transport.
- Pairing trust is committed only after directional secret confirmation, valid credential issuance/verification, and joiner proof of possession.
- Session authentication binds owner/device credentials, fresh nonces, negotiated protocol/features, and channel binding.
- Capability exchange occurs only after session authentication and grants no authorization by itself.
- All queues are bounded; M5 adds no busy polling and no real network transport.
- No Quinn/Iroh/libp2p, persistence, UI, platform adapters, plugins, privileged services, or M6 data-stream admission.
- All security failures are typed and fail closed.

---

### Task 1: Pairing confirmation cryptography

**Files:**
- Modify: `Cargo.toml`
- Modify: `Cargo.lock`
- Modify: `crates/crypto/Cargo.toml`
- Create: `crates/crypto/src/hmac.rs`
- Modify: `crates/crypto/src/lib.rs`
- Test: `crates/crypto/tests/hmac.rs`

**Produces:** `hmac_sha256(key, message) -> [u8; 32]` and constant-time `verify_hmac_sha256` without exposing third-party MAC types.

- [ ] Write fixed HMAC-SHA-256 vector and tamper-rejection tests.
- [ ] Verify the tests fail because the API does not exist.
- [ ] Add pinned RustCrypto dependencies and the minimal wrapper.
- [ ] Verify focused tests and full workspace baseline.
- [ ] Commit the green slice.

### Task 2: Pairing domain transcript and invitation state

**Files:**
- Create: `crates/core/src/pairing/mod.rs`
- Create: `crates/core/src/pairing/invitation.rs`
- Create: `crates/core/src/pairing/transcript.rs`
- Modify: `crates/core/src/lib.rs`
- Modify: `crates/core/Cargo.toml`
- Test: `crates/core/tests/pairing.rs`

**Produces:** typed `PairingId`, `PairingSecret`, invitation lifecycle (`Pending | Consumed | Cancelled | Expired`), `PairingTranscriptV1`, and directional confirmation helpers using canonical transcript v1.

- [ ] Write tests for transcript equality, role-separated confirmations, wrong secret/nonce/key/pairing-id rejection, and single-use invitation terminal states.
- [ ] Verify RED.
- [ ] Implement the minimum domain/state machine.
- [ ] Verify focused and workspace tests.
- [ ] Freeze pairing transcript/HMAC golden vectors and commit.

### Task 3: Pairing bootstrap protobuf messages

**Files:**
- Modify: `crates/protocol/proto/crosslab/protocol/v1/protocol.proto`
- Create: `crates/protocol/src/wire/pairing.rs`
- Modify: `crates/protocol/src/wire/mod.rs`
- Modify: `crates/protocol/src/lib.rs`
- Test: `crates/protocol/tests/pairing_wire.rs`

**Produces:** bounded pre-session v1 pairing hello/confirmation/credential-accepted wire messages with exact identifier/key/nonce/signature length validation. Bootstrap messages are not placed inside `EnvelopeV1`.

- [ ] Write strict round-trip and malformed-length/unknown-profile tests.
- [ ] Verify RED.
- [ ] Extend the tracked `.proto` schema without reusing existing tags.
- [ ] Implement strict wire/domain conversion.
- [ ] Add a fixed pairing bootstrap wire vector and run workspace/fuzz verification.

### Task 4: Pairing orchestration and trust commit

**Files:**
- Create: `crates/core/src/pairing/flow.rs`
- Modify: `crates/core/src/pairing/mod.rs`
- Test: `crates/core/tests/pairing_flow.rs`

**Consumes:** existing `OwnerRootRecord`, `AuthorityDelegation`, `DeviceCredential`, trust types, Task 2 transcript/confirmations, Task 3 wire-domain types.

**Produces:** inviter/joiner pairing state machines that only emit `TrustRecord::Trusted` after final proof of possession.

- [ ] Write S-002 and N-010..N-015 style tests first.
- [ ] Verify RED.
- [ ] Implement fail-closed orchestration and invitation consumption rules.
- [ ] Verify no partial failure can produce Trusted state.
- [ ] Run full baseline and commit.

### Task 5: Session-auth domain and bootstrap wire contract

**Files:**
- Create: `crates/core/src/session/mod.rs`
- Create: `crates/core/src/session/auth.rs`
- Modify: `crates/core/src/lib.rs`
- Modify: `crates/protocol/proto/crosslab/protocol/v1/protocol.proto`
- Create: `crates/protocol/src/wire/session_auth.rs`
- Modify: `crates/protocol/src/wire/mod.rs`
- Test: `crates/core/tests/session_auth.rs`
- Test: `crates/protocol/tests/session_auth_wire.rs`

**Produces:** `SessionAuthTranscriptV1`, role-separated initiator/responder proofs, deterministic `SessionId` derivation, and bounded hello/proof wire messages.

- [ ] Write transcript/proof/SessionId golden tests and wrong nonce/binding/role/key tests.
- [ ] Verify RED.
- [ ] Implement canonical transcript fields exactly from `SESSION-TRANSPORT.md`.
- [ ] Add strict bootstrap wire conversion and bounds.
- [ ] Run focused/full verification and commit.

### Task 6: Bounded in-memory transport seam

**Files:**
- Create: `crates/core/src/transport/mod.rs`
- Modify: `crates/core/src/lib.rs`
- Modify: `apps/sim/Cargo.toml`
- Create: `apps/sim/src/transport.rs`
- Modify: `apps/sim/src/main.rs`
- Test: `apps/sim/tests/memory_transport.rs`

**Produces:** narrow transport-neutral connection metadata/channel-binding/control-send/receive/close seam plus deterministic `MemoryTransportPair` with explicit bounded capacities and `InProcessTest` security class.

- [ ] Write bounded-ordering, saturation, disconnect, close-propagation, and distinct-binding tests.
- [ ] Verify RED.
- [ ] Implement without a general async runtime or unbounded queues.
- [ ] Verify deterministic failure behavior and full baseline.
- [ ] Commit.

### Task 7: Logical session activation and capability exchange

**Files:**
- Create: `crates/core/src/session/state.rs`
- Create: `crates/core/src/session/capabilities.rs`
- Modify: `crates/core/src/session/mod.rs`
- Test: `crates/core/tests/session.rs`

**Produces:** explicit `Created -> Authenticating -> Active -> Closing -> Closed` state machine plus revocation terminal path, authenticated peer context, protocol/feature negotiation, directional sequence initialization, and post-auth capability intersection.

- [ ] Write S-003/S-004 and N-020..N-027 style tests.
- [ ] Verify RED.
- [ ] Implement state transitions and activation gate.
- [ ] Verify capability advertisement cannot create policy authority.
- [ ] Run full baseline and commit.

### Task 8: Sequenced control request/response/event simulator

**Files:**
- Create: `crates/core/src/control/mod.rs`
- Create: `apps/sim/src/node.rs`
- Modify: `apps/sim/src/main.rs`
- Test: `apps/sim/tests/session_scenarios.rs`

**Produces:** authenticated M4 envelope dispatch for capability advertisements, request/response correlation, permitted events, cancellation, and protocol/session close handling over the memory control channel.

- [ ] Write S-006 plus duplicate/gap/nonretryable request tests.
- [ ] Verify RED.
- [ ] Implement bounded dispatch with existing protocol sequence validators.
- [ ] Verify malformed/invalid traffic fails closed and does not bypass policy.
- [ ] Run full baseline and commit.

### Task 9: M5 end-to-end scenarios, fuzzing, and closeout

**Files:**
- Create/modify: `apps/sim/tests/m5_scenarios.rs`
- Modify: `fuzz/Cargo.toml`
- Add: `fuzz/fuzz_targets/pairing_bootstrap.rs`
- Add: `fuzz/fuzz_targets/session_auth.rs`
- Modify: `.github/workflows/fuzz.yml`
- Modify: `docs/development/CURRENT.md`

**Produces:** S-001..S-006 coverage applicable through M5, required M5 security negatives, parser fuzz smoke for pairing/session-auth bootstrap, and durable M6 handoff.

- [ ] Add end-to-end pairing -> fresh authenticated session -> capability exchange -> sequenced control scenario.
- [ ] Add wrong-secret/replay/wrong-binding/stale-credential/owner-mismatch failures.
- [ ] Add bounded parser fuzz targets and smoke CI commands.
- [ ] Run `cargo metadata --locked`, `cargo fmt --check`, workspace check, Clippy `-D warnings`, tests, and fuzz smoke.
- [ ] Review full diff against M5 scope, update `CURRENT.md`, and integrate only if every required check is green.
