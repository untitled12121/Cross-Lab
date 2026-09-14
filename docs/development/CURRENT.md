# Current Development State

This file is the durable resume guide for active Cross-Lab development. Git, code, and tests are the factual implementation state; this file records the intended handoff.

## Current Phase

**Phase 1 — Core Simulator / Real Transport Transition**

## Current Milestone

**M9 — Remote Networking ADR, paused before M9 Task 4 while the second foundation security remediation plan is executed.**

M1–M8 are complete on canonical `main`. M9 remains isolated on `m9-remote-networking` in draft PR #19. M10 remains the first Linux + Android platform vertical slice after the remote-networking architecture is selected.

## Canonical Baseline

- Canonical M8 documentation head: `ab602d62181b3dcba056874d0013e40522a68e8f`, CI `34688627614` — full gate passed.
- `m9-remote-networking` was created from exactly that head.
- M9 Tasks 1–3 remain implemented and previously verified.
- M9 Task 4 must not begin until the active foundation remediation backlog below is closed and the exact final head passes the full gate.

## Active Plans

Primary M9 design and implementation plans remain under `docs/plans/phase-1/`.

The active security remediation plan is:

`docs/superpowers/plans/2026-09-13-foundation-security-remediation.md`

The discovery/audit record is:

`docs/plans/phase-1/M9-whole-project-security-quality-audit.md`

The remediation plan is executed regression-first. Existing Master Architecture, Identity/Keys, Pairing/Trust/Revocation, Policy/Authorization, Session/Transport, Protocol V1, Security Boundaries, Threat Model, and accepted ADRs remain authoritative.

## Completed Remediation Work

### Task 1 — Pairing currentness and initial credential epoch

Implemented on the active branch:

- removed caller-selected initial pairing credential epochs; initial enrollment issues epoch `0` by construction;
- rejects nonzero initial credentials at the joiner pairing boundary;
- added CSPRNG-backed `PairingId::generate()` and `PairingSecret::generate()` using the existing OS-backed crypto random source;
- added typed local `PairingInstant` creation/deadline metadata;
- invalid/expired invitations fail closed and become terminal;
- invitation currentness is enforced at inviter-flow creation, joiner-confirmation verification, credential issuance, and final trust commit;
- expiry occurring after credential acceptance but before trust commit is covered and cannot create trust;
- simulator/core deterministic fixtures use explicit local monotonic-style test ticks;
- pairing golden vectors were reconciled after initial epoch became fixed at zero.

The Task 1 changes are included in the verified Task 2 checkpoint below.

### Task 2 — Credential rotation and active-session invalidation

Exact verified code head:

`7cd243ff52115dad923b3cf4aba92a04bde79409`

Implemented:

- removed raw public numeric `TrustRecord::advance_credential_epoch(u64)`;
- added verified successor-credential acceptance using the signed `DeviceCredential`, current owner root/delegation evidence, exact owner/device binding, and exact `N + 1` progression;
- stale, skipped, wrong-device, and invalid-authority successors fail closed;
- accepting a successor credential atomically advances `accepted_credential_epoch`, `trust_revision`, and `last_transition_id` only after verification succeeds;
- added `LogicalSession::revalidate_peer_trust` so an active session can be invalidated against current local trust/credential state;
- inbound control validates both authenticated credential epoch and trust revision before sequence/body dispatch;
- credential-epoch drift is fatal in the simulator and closes session authority;
- existing operation/stream admission already binds authority to `trust_revision`, so credential rotation invalidates stale stream-operation authority through the existing conservative revision contract;
- revocation tests that previously mutated an epoch numerically now rotate through the same verified successor-credential contract.

Verification on exact head `7cd243ff52115dad923b3cf4aba92a04bde79409`:

- Rust CI `34747301958` — lockfile verification, dependency audit, rustfmt, workspace check, Clippy with `-D warnings`, and complete workspace tests all passed;
- Fuzz Smoke `34747301924` — fuzz lockfile verification, formatting, and all bounded fuzz targets passed.

### Task 3 — Trusted-state and approval provenance

Exact verified code head:

`685b8ec948671a989b7f32a46e021d629cdbfa2c`

Implemented:

- removed ordinary direct construction of trusted membership;
- added signed `PairingTrustTransition` evidence bound to the initial credential, transition ID, pairing-evidence digest, owner authority, and issuer key;
- initial trusted membership is established only after validating the epoch-0 credential and pairing trust transition against owner authority;
- pairing flow commits trust through that verified transition rather than minting `TrustState::Trusted` directly;
- removed public direct minting of `VerifiedApproval`;
- added signed `OwnerApprovalEvidence` with exact approval scope, owner identity, issuer key/delegation epoch, issue time, and expiry;
- local verification produces the private `VerifiedApproval` capability only after validating Administrative authority, owner/issuer binding, signature, and lifetime;
- approval evaluation remains pure and requires explicit local time when approval authority is relevant;
- scope and lifetime mismatch tests fail closed.

Verification on exact head `685b8ec948671a989b7f32a46e021d629cdbfa2c`:

- Rust CI `34794229945` — lockfile verification, dependency audit, rustfmt, workspace check, Clippy with `-D warnings`, and complete workspace tests all passed;
- Fuzz Smoke `34794229882` — fuzz lockfile verification, formatting, and all bounded fuzz targets passed.

### Task 4 — Request replay and local retry authority

Exact verified code head:

`cad5222d744eb86888b10cf884309bc65c5d95fa`

Implemented:

- peer-declared `RetryClass` no longer grants receiver-side duplicate/replay authority;
- active inbound request IDs and recently terminal inbound IDs are tracked by receiver-local state;
- completed and cancelled request IDs remain replay-protected inside a bounded local window;
- terminal replay state uses FIFO reclamation so long sessions do not permanently exhaust admission capacity;
- active request authority is never evicted merely to reclaim completed replay state;
- per-session replay state is cleared with the rest of dispatcher session state;
- message-sequence anti-replay, request/response ownership, and existing resource bounds remain intact.

Regression-first evidence covered duplicate idempotent IDs, cancelled-ID reuse, and completed-state liveness before the production change.

Verification on exact head `cad5222d744eb86888b10cf884309bc65c5d95fa`:

- Rust CI `34796612978` — lockfile verification, dependency audit, rustfmt, workspace check, Clippy with `-D warnings`, and complete workspace tests all passed;
- Fuzz Smoke `34796613004` — fuzz lockfile verification, formatting, and all bounded fuzz targets passed;
- dependency audit retains the previously documented allowed `paste 1.0.15` / `RUSTSEC-2024-0436` maintenance warning in the isolated Iroh experiment dependency graph.

### Task 5 — Event authorization/subscription boundary

Exact verified code head:

`7126e8057600282fb59cf0e533b0ba02c8b46033`

Implemented:

- reconciled event delivery around explicit receiver-local authority without changing Protocol V1 wire compatibility;
- added typed `EventSubscription` state keyed by exact capability and event type;
- negotiated capability support remains a prerequisite but no longer acts as implicit permission to deliver every capability event;
- peers cannot create or widen local event subscription authority through event metadata;
- unmatched capability events fail closed with `EventNotSubscribed`;
- exact local subscription permits only the subscribed capability/event pair;
- authenticated system events remain outside the capability-subscription grammar and continue through the existing session identity/trust/sequence boundary;
- simulator and authenticated Quinn fixtures now establish explicit local subscription authority before expecting capability-event delivery.

Verification on exact head `7126e8057600282fb59cf0e533b0ba02c8b46033`:

- Rust CI `34801746532` — lockfile verification, dependency audit, rustfmt, workspace check, Clippy with `-D warnings`, and complete workspace tests all passed;
- Fuzz Smoke `34801746547` — fuzz lockfile verification, formatting, and all bounded fuzz targets passed;
- dependency audit retains the previously documented allowed `paste 1.0.15` / `RUSTSEC-2024-0436` maintenance warning in the isolated Iroh experiment dependency graph.

## Exact Next Task

**Foundation remediation Task 6 — Quinn plain-`Drop` connection/task ownership.**

Keep the explicit async shutdown contract, but make ordinary owner drop fail closed instead of leaving authority-bearing transport tasks detached.

Required behavior:

1. add a regression proving dropping `QuicTransportConnection` without `shutdown().await` closes local stream/control authority and becomes peer-visible;
2. synchronous drop cleanup must close the QUIC connection and close task admission before background work can outlive the owning transport;
3. owned task handles must be drained from the registry and synchronously aborted on plain `Drop`;
4. explicit `shutdown().await` must continue to close first and then join owned tasks cleanly;
5. cleanup must remain idempotent when shutdown and drop occur in sequence.

## Remaining Remediation Backlog

After Task 6:

- finish custom redacted `Debug` for core session-auth proofs/transcripts and pairing transcript/security material;
- finish OS-CSPRNG constructors for all locally created random identifiers required by the specifications;
- explicitly resolve/design authoritative delegated-role epoch state rather than treating a caller-supplied/current object epoch as globally authoritative; ADR-0009 remains reserved for the M9 networking decision, so this authority-state decision must use a later monotonic ADR number;
- run the complete regression/security/fuzz/dependency gate on the final exact head;
- reconcile `security/M9-FOUNDATION-HARDENING-ASSESSMENT.md`, this file, and PR #19 before M9 Task 4 can resume.

## Important Known Design Constraint

Authoritative delegated-role epoch state is not centrally persisted yet. Current verification APIs can validate a delegation against a supplied minimum epoch, but that alone does not prove which delegation epoch local durable authority state has accepted. Do not silently claim this gap is solved; give it an explicit authority-state design before the final hardening gate.

ADR-0009 is reserved by the active M9 networking work. Do not reuse it for delegated-authority state.

## Repository / M9 Invariants

- Quinn remains the verified local/LAN baseline.
- Iroh `1.2.0` remains an isolated M9 remote/NAT/relay candidate under `experiments/m9-networking` until ADR-0009 selects an architecture.
- Iroh identities, addresses, paths, relay metadata, and transport credentials never become Cross-Lab identity/trust/policy authority.
- No 0-RTT authority.
- M9 remote sessions remain `NetworkClass::Remote` for their lifetime.
- Transport/path changes do not silently mutate binding, `SessionId`, sequence, policy classification, or operation authority.
- A new transport connection requires fresh Cross-Lab authentication/authorization state.
- `main` branch protection remains an external repository-administration item.

## Resume Procedure

1. inspect `m9-remote-networking`, PR #19, current branch head/status, recent commits/workflows, and this file;
2. read the Master Architecture, active remediation plan, audit, and relevant architecture/ADRs before changing authority or protocol contracts;
3. continue the exact next remediation task RED -> verified failure -> minimal GREEN -> focused verification -> full gate;
4. preserve clean crate boundaries and keep policy evaluation pure;
5. update this file after each meaningful verified checkpoint;
6. keep M9 Task 4 blocked until every remediation task is durably resolved and the exact final head passes the full verification suite.
