# M9 Whole-Project Security and Quality Audit

**Status:** Reconciled pre-merge  
**Goal:** Re-audit the Phase 1 foundation for security bypasses, stale authority, protocol/logic defects, resource-lifetime bugs, privacy leaks, unsafe API boundaries, dependency/CI gaps, and maintainability problems before M9 remote-networking Task 4.

## Governing constraints

- Master Architecture, Security Boundaries, Threat Model, Identity/Keys, Pairing/Trust/Revocation, Policy/Authorization, Session/Transport, and Protocol V1 remain authoritative.
- ADR-0010 defines authoritative owner-currentness semantics.
- ADR-0011 defines bounded request replay semantics.
- Iroh candidate code remains isolated under `experiments/m9-networking`; this audit does not promote a networking architecture.
- No protobuf schema, canonical transcript, signature format, or protocol-version change is part of the foundation remediation.

## Audit disposition

The originally confirmed findings have been re-evaluated against the current implementation and exact-head verification. Immediate code findings are resolved; architecture-sensitive currentness/replay findings were resolved through accepted ADR-0010/0011 before implementation. Remaining platform/persistence/networking obligations are explicitly deferred below rather than implied complete.

## Confirmed findings and final disposition

### A. Pairing invitation lifetime was not enforced — Resolved

Pairing invitations now carry local lifetime/deadline state and terminal expiry behavior. Expired or consumed invitations cannot commit trust or be reused. The implementation does not rely on peer wall-clock time.

### B. First pairing could issue a nonzero credential epoch — Resolved

Initial pairing now establishes credential epoch 0 by construction. Ordinary credential rotation remains an exact next-epoch transition.

### C. Pairing bootstrap identifiers/secrets lacked secure generation APIs — Resolved

`PairingId` and pairing-secret material have CSPRNG-backed generation APIs. Deterministic byte constructors remain available for tests without becoming the production generation path.

### D. Credential rotation could leave an old authenticated session active — Resolved

Authenticated sessions snapshot accepted peer credential state. Local acceptance of a verified successor credential advances trust revision/current credential epoch, and stale authenticated session authority is rejected/cancelled. Session lifecycle tests cover old-session invalidation.

### E. Credential-epoch acceptance API was weakly authorized — Resolved

`TrustRecord::accept_successor_credential` now consumes the exact successor `DeviceCredential` plus local `OwnerAuthorityState`. It verifies current Device Signing authority, same owner/device, trusted state, and exactly the next credential epoch before mutation.

### F. Verified approval evidence was publicly mintable — Resolved

Ordinary callers cannot construct `VerifiedApproval`. Signed `OwnerApprovalEvidence` is verified against current Administrative authority before typed verified evidence can satisfy owner-confirmation policy. Compile-fail API coverage locks the minting boundary.

### G. Trusted-record authority was publicly mintable — Resolved

Ordinary callers cannot construct trusted membership directly. `TrustRecord` trusted state is created through verified pairing evidence; compile-fail API coverage locks that boundary.

### H. Peer-supplied retry class could weaken duplicate semantics — Resolved by ADR-0011

Peer-declared retry class is not authority. `(SessionId, message_seq)` is the exact-envelope replay/order boundary, while `RequestId` is bounded correlation/duplicate/retry history. Every accepted request attempt still passes current local trust/capability/policy/operation authorization.

### I. Completed nonretryable request state had a bounded-liveness conflict — Resolved by ADR-0011

Completed request history is bounded. A legitimately aged-out `RequestId` becomes a new authenticated request attempt rather than durable exactly-once state; exact old sequence replay remains rejected. Characterization tests prove aged-out requests are still subject to current local policy.

### J. Capability events lacked an authorization/subscription gate — Resolved for current capability events; future system-event families deferred

Capability-scoped events now require negotiated capability state plus an exact local capability/event subscription. Existing authenticated system events remain a deliberately narrow protocol path. Any future authority-sensitive system-event family requires explicit local authorization/subscription rules before introduction.

### K. Ordinary Debug output could expose payload/authentication material — Resolved

Protocol, simulator, authentication, and transport debug representations redact raw control payloads, pairing confirmations, session-auth proof/transcript material, and endpoint descriptions while retaining safe diagnostic state.

### L. Quinn Drop did not deterministically terminate owned tasks — Resolved

Explicit shutdown remains the graceful joined path. Drop now closes/terminates connection-owned work so transport task authority is not left detached. Lifecycle regression coverage verifies close/drop ownership behavior.

## Additional authority-currentness findings resolved by ADR-0010

The second remediation pass established a single local currentness source rather than allowing high-level callers to provide roots, delegations, or minimum epochs:

- identity owns `OwnerAuthorityState`;
- owner-root, Device Signing, Administrative, and Recovery roles have independent current state and monotonic floors;
- root succession clears active delegated roles while preserving floors;
- credential, approval, pairing, trust-transition, and session-authentication paths resolve current authority locally;
- old root/delegated authority cannot authorize ordinary high-level work after replacement;
- active root or Device Signing replacement invalidates ordinary sessions and cancels session-scoped control/stream authority;
- Administrative/Recovery-only rotation does not invalidate ordinary sessions;
- high-level stream admission derives trust/policy revisions from local `TrustRecord` and `PolicyState`, rejecting revoked or mismatched peer trust.

## Reviewed areas with no new concrete defect in this gate

- Workspace production crates remain free of unsafe-code reliance.
- Ed25519 validation/signing, HMAC verification, canonical transcript/domain separation, and fixed-width protocol fields remain covered by the full test suite.
- Protocol frame readers continue to validate declared lengths before body allocation.
- Memory and Quinn control/data queues remain bounded and preserve caller-owned bytes on local backpressure/oversize failures.
- Quinn stream admission remains operation-bound and bounded; FIN/RESET/STOP/connection loss map to typed Cross-Lab outcomes.
- Iroh identity/address/path/relay metadata remains routing/transport metadata only and is not Cross-Lab trust/policy authority.
- No 0-RTT authority has been introduced.
- Golden identity, policy, and protocol wire vectors remain byte-for-byte unchanged under the remediation.

## Verification evidence

Code-bearing remediation commit: `cf48aaa26fb4099864c6afbd5218043e551745f1`.

Human-authored verification checkpoint: `b3d2b35d8e95103f7e283585ff1951737eae07ab`.

Exact checkpoint evidence:

- Rust CI `34973222111` passed lockfile verification, `cargo audit`, rustfmt, full workspace check, Clippy with `-D warnings`, and `cargo test --workspace --all-features`.
- Fuzz Smoke `34973222116` passed the committed exact gate on `nightly-2026-09-12`: `control_frame`, `data_stream_open`, `identifiers`, `pairing_bootstrap`, and `session_auth`, each with `-runs=256`.
- Identity golden vectors for authority delegation and device credentials passed.
- Policy trust-revocation golden vector passed.
- Protocol control-envelope, pairing-confirmation, data-stream-open, session-close, and system-event golden frame vectors passed.
- Replay, authority-currentness, stream-currentness, debug-redaction, resource-hardening, simulator lifecycle, reconnect, and Quinn lifecycle regressions passed in the same full workspace run.

`cargo audit` reported no vulnerability failure and one allowed maintenance warning: `paste 1.0.15` / `RUSTSEC-2024-0436`, isolated to the Iroh experiment dependency stack.

## Deferred / known risks

These items remain outside this in-memory Phase 1 foundation gate:

- Durable, rollback-resistant persistence for owner authority/currentness state and accepted epochs.
- Platform keystore / secure-enclave integration and recovery material handling in native privileged/platform boundaries.
- Explicit authorization/subscription design for future authority-sensitive system-event families.
- Authorized-operation time remains a raw monotonic-compatible integer at this stage; platform slices should prevent accidental wall-clock/monotonic mixing.
- Route-migration/NAT/relay/measurement work remains M9 remote-networking scope; remote sessions must remain `NetworkClass::Remote` for their lifetime and route changes must not become authority.
- `paste 1.0.15` / `RUSTSEC-2024-0436` remains an unsuppressed Iroh-experiment maintenance warning to re-evaluate before ADR-0009 promotion.
- `main` branch protection remains a repository-administration item.
- Windows/macOS/mobile CI matrices remain future platform-slice gates; Linux CI is not treated as proof of those platforms.
- Real mobile lifecycle evidence remains an M10 obligation.

## Completion gate

The code-level audit remediation is verified. Final completion still requires:

1. reconcile this audit, the security assessment, active remediation plan, and `docs/development/CURRENT.md` on the final PR head;
2. require exact-final-head Rust CI and Fuzz Smoke success;
3. confirm the PR diff contains no unintended protobuf/schema/canonical-vector change;
4. merge PR #23 at the expected verified head;
5. require post-merge `main` Rust CI green.

M9 remote-networking Task 4 remains paused until those merge/post-merge gates are satisfied.
