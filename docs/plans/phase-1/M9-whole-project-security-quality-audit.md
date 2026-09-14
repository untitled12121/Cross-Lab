# M9 Whole-Project Security and Quality Audit Plan

**Status:** Active pre-Task-4 hardening review

**Goal:** Re-audit the Phase 1 foundation after the first M9 hardening pass for security bypasses, stale authority, protocol/logic defects, resource-lifetime bugs, privacy leaks, unsafe API boundaries, dependency/CI gaps, and maintainability problems. Keep M9 Task 4 paused until this review is completed and explicitly approved.

## Governing constraints

- Master Architecture, Security Boundaries, Threat Model, Identity/Keys, Pairing/Trust/Revocation, Policy/Authorization, Session/Transport, and Protocol V1 remain authoritative.
- Do not silently change trust, protocol, privilege, recovery, transport, update, or major repository boundaries.
- Implementation bugs already contradicted by accepted specifications may be fixed directly with RED -> verified failure -> minimal GREEN -> full verification.
- Architecture-sensitive authority/API changes require a focused design/ADR and explicit approval before implementation.
- Keep Iroh candidate code isolated under `experiments/m9-networking`; do not begin M9 Task 4 during this audit.

## Review scope

1. Crypto/key handling and transcript/domain separation.
2. Identity, authority delegation, credential currentness, and key rotation.
3. Pairing freshness, one-time secret handling, enrollment state, and trust commit.
4. Trust/revocation provenance and lifecycle.
5. Policy context provenance, approval evidence, rule evaluation, and operation authority.
6. Session authentication, reconnect, credential/trust currentness, sequence/replay handling.
7. Control-plane requests/retries/cancellation/events and bounded state.
8. Data-stream admission, operation binding, cancellation, and resource reclamation.
9. Memory/Quinn/Iroh transport bounds, task ownership, close/drop behavior, and metadata privacy.
10. Wire parsing, fuzz coverage, error semantics, and Debug/log redaction.
11. CI/supply-chain hygiene and cross-platform verification readiness.
12. API/design quality, misuse resistance, type safety, duplication, and deferred-risk documentation.

## Confirmed findings

### A. Pairing invitation lifetime is not enforced

`PairingInvitation` contains no creation/deadline state. `expire()` is only a manual transition, so a stale invitation remains usable indefinitely if a caller forgets to expire it. This contradicts the accepted pairing specification requiring local creation/deadline metadata and timeout/expiry consumption.

**Action:** RED tests for deadline enforcement and terminal expiry, then add explicit local monotonic deadline input/state. Do not trust peer wall-clock time.

### B. First pairing can issue a nonzero credential epoch

`PairingInviterFlow::issue_joiner_credential` accepts arbitrary `credential_epoch`; existing simulator fixtures even pair first-time devices at epoch 5. Identity v1 requires the initial credential epoch to be 0 and ordinary rotation to advance exactly one.

**Action:** RED test, then make initial pairing issue epoch 0 by construction rather than caller choice.

### C. Pairing bootstrap identifiers/secrets lack secure generation APIs

`PairingId` and `PairingSecret` expose deterministic `from_bytes` only. Production callers can therefore accidentally supply predictable values despite the threat model requiring high-entropy one-time bootstrap material.

**Action:** add CSPRNG-backed generation APIs while retaining deterministic constructors for tests. Add randomness/error tests without weakening secret redaction/zeroization.

### D. Credential rotation can leave an old authenticated session active

`SessionContext` records the authenticated peer credential epoch, but `ControlDispatcher::validate_peer_trust` currently checks trust revision only. `TrustRecord::advance_credential_epoch` changes accepted epoch without changing revision. Therefore a session authenticated with epoch N can continue ordinary control after local trust advances to N+1.

**Action:** prove with RED control/session lifecycle test. Prefer explicit accepted-credential-epoch currentness validation against the authenticated session snapshot rather than overloading trust revision semantics. Ensure mismatch is fatal and session-scoped authority is cancelled. Then verify stream/operation behavior has an equivalent currentness path or document the exact remaining gap before changing architecture.

### E. Credential-epoch acceptance API is weakly authorized

`TrustRecord::advance_credential_epoch(next_epoch)` validates only numeric progression; it does not consume a verified new credential or current owner-authority evidence even though the identity spec requires owner-domain authorization for accepting a higher epoch.

**Classification:** architecture/API-boundary hardening. Do not redesign silently. Produce a focused design for a verified credential-rotation transition before replacing this API.

### F. Verified approval evidence is publicly mintable

`VerifiedApproval::owner_confirmation(scope)` is public and `AuthorizationContext::with_verified_approval` accepts it. Any ordinary crate with policy access can manufacture the typed evidence that satisfies an `Ask`/OwnerConfirmation rule. This contradicts the provenance rule that approval evidence originates from a local trusted approval mechanism.

**Classification:** authority-boundary design issue. Requires a focused approval-verifier/broker boundary design and explicit approval before implementation.

### G. Trusted-record authority is publicly mintable

`TrustRecord::trusted(...)` directly creates fully trusted membership state. Current pairing uses it correctly only after mutual confirmation and credential acceptance, but the constructor itself is broadly available and can bypass the intended trust-commit ceremony in other callers.

**Classification:** authority-boundary design issue. Requires a trust-store/commit boundary design rather than a cosmetic visibility change.

### H. Peer-supplied retry class can weaken duplicate semantics

`ControlRequest.retry_class` is sender-controlled. The receiver does not validate retry semantics against a locally authoritative capability-operation declaration. Completed `Idempotent` requests are removed from active state and can be redispatched with the same `RequestId`; there is no bounded completed-result cache. A malicious peer can mark an operation idempotent even when local semantics are not.

**Classification:** protocol/capability-model design issue. Until an authoritative local operation retry registry/cache is designed, generic code must not treat peer classification as authority. Produce focused design before behavior change.

### I. Completed nonretryable request state has a bounded-liveness conflict

`seen_nonretryable` is bounded by `state_capacity` but never ages out during the session. Once filled, the session reaches `ResourceLimit`. This preserves replay safety but limits session lifetime. Protocol V1 also requires duplicate NonRetryable IDs to remain rejected for the session, so simple eviction would weaken replay semantics.

**Classification:** protocol-state design issue. Resolve together with retry/result-cache design, not via ad-hoc eviction.

### J. Capability events have no authorization/subscription gate

`ControlDispatcher::validate_event` checks only that a capability was negotiated; system events are accepted unconditionally. Protocol V1 explicitly states events do not bypass policy merely because they are one-way and that event families define authorization/subscription requirements.

**Classification:** protocol/capability-model design issue. Define event-family authorization/subscription metadata and fail-closed defaults before widening event use.

### K. Ordinary Debug output can expose control payloads and authentication material

`ControlRequest`, successful `ControlResponse`, `Event`, `EnvelopeBody`, `ControlEnvelope`, and simulator `NodeEvent` derive `Debug` while carrying raw operation bodies. Pairing confirmations and session-auth proofs also derive `Debug` while carrying reusable/captured authentication material. Threat-model logging rules prohibit ordinary payload/auth-material leakage.

**Action:** RED tests using distinctive sentinel bytes, then custom redacted `Debug` implementations that retain safe identifiers/lengths/state but not raw bodies, confirmations, signatures, transcript digests, or secrets.

### L. Quinn connection Drop does not deterministically terminate owned tasks

Explicit `shutdown()` joins owned tasks, but dropping `QuicTransportConnection` drops `JoinHandle`s and can detach tasks until they observe channel/connection closure. The architecture requires owned bounded task lifetimes and no detached background work.

**Action:** RED lifecycle test for drop behavior if observable. Add RAII close/task-abort cleanup while preserving async joined `shutdown()` as the graceful path. Apply the same rule to candidate runtime only if evidence shows the same leak and without beginning Task 4.

## Reviewed areas with no new concrete defect so far

- Workspace forbids unsafe code.
- Ed25519 key handling uses strict verification; signing-key Debug is redacted and secret buffers are zeroized where currently represented in software.
- HMAC verification uses constant-time library verification.
- Canonical transcript/domain separation and fixed-width field encoding remain consistent with Protocol V1.
- Protocol frame readers validate declared lengths before allocating bodies.
- Memory and Quinn control/data queues are bounded and preserve caller-owned bytes on local backpressure/oversize failures.
- Quinn stream admission uses bounded semaphores/queues and maps FIN/RESET/STOP/connection loss to typed Cross-Lab outcomes.
- Current Iroh Task-3 control bridge is bounded and owns/join-shuts its registered tasks; stream/window limits intentionally remain a Task-4 obligation.
- Iroh endpoint identity remains routing metadata only; Cross-Lab identity/policy authority has not been delegated to it.
- Current CI pins Rust/actions/cargo-audit and the fuzz toolchain; full CI and fuzz were green at the pre-audit checkpoint.

## Deferred/known risks to record, not silently solve

- Current delegated-authority epoch state is not centrally persisted; callers must supply accepted current epochs. A real authority store is a later persistence/platform concern.
- AuthorizationContext/PolicyState/TrustRecord APIs are usable by trusted in-process code today; future UI/plugin isolation must not expose raw authority constructors across less-trusted boundaries.
- AuthorizedOperation time uses raw `u64`; platform milestones should introduce a monotonic-time/deadline type before wall-clock/monotonic sources can be mixed.
- Constraint snapshots are not dynamically re-evaluated inside `AuthorizedOperation`; current M9 network class is immutable per logical session by architecture, so this is not presently a route-migration bypass.
- `paste 1.0.15` / RUSTSEC-2024-0436 remains an unmaintained transitive warning in the isolated Iroh experiment stack and must be re-evaluated before ADR-0009 promotion.
- `main` branch protection is still disabled and requires repository administration.
- Windows/macOS/mobile CI matrices are not yet present; add platform gates as the corresponding platform slices land rather than pretending Linux-only CI proves platform support.

## Execution order

### Slice 1 — Debug/log privacy hardening

1. Write protocol/simulator RED tests proving payload/auth sentinel bytes appear in current `Debug` output.
2. Verify RED on the exact branch head.
3. Add the smallest custom redacted Debug implementations.
4. Run focused tests, then full workspace fmt/check/clippy/test and fuzz smoke where parser code changed.
5. Checkpoint exact commit/CI evidence.

### Slice 2 — Pairing freshness and initial credential currentness

1. Write RED tests for expired invitation rejection, initial epoch 0, and CSPRNG generation APIs.
2. Verify RED.
3. Add explicit local deadline handling and generation APIs; remove caller control over first credential epoch.
4. Update simulator fixtures and any affected vectors only when protocol bytes are intentionally unchanged.
5. Full verification and checkpoint.

### Slice 3 — Active-session credential-epoch currentness

1. Write a RED lifecycle test: authenticate at epoch N, locally accept N+1, then attempt ordinary control under the old session.
2. Verify the old session currently remains usable.
3. Add explicit peer credential epoch validation against `SessionContext` and make mismatch fatal/cancel session authority.
4. Add corresponding stream-side test/handling if stream admission can otherwise continue under the old session.
5. Full verification and checkpoint.

### Slice 4 — Transport drop ownership

1. Prove whether dropping a Quinn transport leaves live owned tasks/connection authority.
2. If reproducible, add RAII close/abort cleanup without weakening graceful joined shutdown.
3. Full verification and checkpoint.

### Slice 5 — Architecture-sensitive remediation design

Create a focused design/ADR proposal covering:

- trusted approval minting boundary;
- trusted-record/trust-store commit authority;
- verified credential-rotation acceptance;
- local capability operation metadata for retry semantics and bounded idempotent-result caching;
- capability/system event authorization/subscription semantics;
- long-lived duplicate/replay state strategy.

Do not implement these material authority/protocol changes until explicitly approved.

## Completion gate

The audit is not complete until:

- every immediate slice has fresh exact-head verification;
- architecture-sensitive findings have explicit durable disposition;
- `docs/development/CURRENT.md` records exact commits/workflow runs and remaining risks;
- PR #19 reflects the verified state;
- M9 Task 4 remains paused until the owner explicitly approves proceeding.
