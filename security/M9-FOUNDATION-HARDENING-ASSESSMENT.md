# M9 Foundation Security Hardening Assessment

**Status:** Verified pre-merge  
**Scope:** Phase 1 foundation hardening required before M9 remote-networking Task 4.

## Result

The in-tree M9 foundation hardening work is implemented and verified. The remediation enforces accepted Cross-Lab authority, replay, lifecycle, provenance, privacy, and bounded-resource invariants; it does not introduce a new transport or privilege architecture.

The final code-bearing remediation commit is `cf48aaa26fb4099864c6afbd5218043e551745f1`. Human-authored verification checkpoint `b3d2b35d8e95103f7e283585ff1951737eae07ab` contains only durable-state documentation on top of that code and passed the full Rust CI and bounded fuzz gates against current `main`.

## Verified Security State

### Owner authority currentness

ADR-0010 is implemented through identity-owned `OwnerAuthorityState`.

- The active owner root and Device Signing, Administrative, and Recovery delegations are resolved from local authority state rather than caller-selected currentness floors.
- Delegated-role replacement is strictly monotonic and failed transitions do not mutate accepted authority state.
- Root succession clears active delegated roles while retaining their epoch floors, so old-root delegations cannot become current again.
- Ordinary credential, approval, pairing, trust-transition, pairing/session-authentication, and lifecycle paths consume `OwnerAuthorityState`.
- Low-level raw root/delegation validation remains private or restricted to cryptographic/import boundaries where currentness is not being established.
- Old roots and superseded delegated authorities cannot authorize ordinary high-level work after replacement.

### Credential and trust provenance

- Initial pairing issues credential epoch 0 by construction.
- Successor credential acceptance consumes the exact successor `DeviceCredential`, verifies it against current Device Signing authority, requires the same owner/device, and requires exactly the next credential epoch.
- `TrustRecord` trusted state is established only through verified pairing evidence; ordinary callers cannot mint trusted membership directly.
- Active sessions bind the accepted peer credential epoch, so locally accepting a successor credential invalidates authority held by an older authenticated session.

### Approval provenance

- `VerifiedApproval` cannot be minted by ordinary callers; it is produced only after validation of signed `OwnerApprovalEvidence` against current Administrative authority.
- Approval scope, lifetime, issuer, delegation epoch, and signature remain bound to the canonical transcript.

### Session authority lifecycle

- Session activation verifies both peer credentials against current local owner authority.
- `SessionContext` snapshots current owner-root and Device Signing authority identifiers/epochs.
- Root or Device Signing replacement invalidates ordinary active sessions and cancels session-scoped control/stream authority before transport close.
- Administrative- or Recovery-only rotation does not invalidate ordinary sessions.
- Reconnect creates a fresh transport binding, authentication transcript, `SessionId`, sequence state, and authorization state.

### Replay and duplicate semantics

ADR-0011 is implemented and characterized.

- `(SessionId, message_seq)` remains the exact authenticated-envelope replay/order boundary.
- Duplicate or lower sequences and sequence gaps fail closed.
- `RequestId` is bounded in-session correlation/duplicate/retry history, not durable authority.
- A legitimately aged-out `RequestId` is treated as a new authenticated request attempt and must pass current trust, capability, policy, and operation authorization.
- Peer-declared retry class does not grant authority or create a generic exactly-once guarantee.

### Stream currentness and operation authority

- High-level stream admission receives the local `TrustRecord` and `PolicyState` rather than caller-selected revision integers.
- Peer trust owner/device/state is validated against the authenticated session.
- Trust and policy revisions are derived locally before operation authority is consumed.
- Locally revoked peers, mismatched trust provenance, changed policy revision, expired operations, exhausted stream budgets, and cancelled authority all fail closed.
- Terminal/cancelled operation registrations are reclaimed so bounded state does not become an avoidable lifetime leak.

### Event authorization

- Capability-scoped events require both negotiated capability state and an exact local capability/event subscription.
- Authenticated system events remain a deliberately narrow protocol path. Future system-event families that carry new authority-sensitive semantics still require explicit local authorization/subscription design before use.

### Pairing freshness and secret handling

- Pairing invitations carry local lifetime/deadline state and terminal expiry semantics.
- Pairing identifiers and one-time secrets have CSPRNG-backed generation APIs while deterministic constructors remain available for tests.
- Pairing transcript/confirmation/credential-acceptance state remains bound to exact identities, nonces, keys, and canonical transcript material.

### Privacy and task ownership

- Control payloads, pairing confirmations, session-auth proofs/transcripts, transport endpoint descriptions, and simulator event payloads are redacted from ordinary `Debug` output.
- Quinn explicit shutdown joins owned tasks; Drop closes/terminates connection-owned work rather than leaving detached transport authority.
- The isolated Iroh candidate task registry rejects spawn after shutdown admission closes and joins registered work during shutdown.

### Supply-chain and CI reproducibility

- CI runs on `ubuntu-24.04`, uses Rust `1.98.1`, pins `actions/checkout` by commit, installs pinned `cargo-audit 0.22.2`, and requires `cargo audit` to exit successfully.
- Fuzz Smoke uses dated `nightly-2026-09-12`, pinned `cargo-fuzz 0.13.2`, committed `fuzz/Cargo.lock`, and exactly five bounded targets at 256 runs each.

## Verification Evidence

Exact checkpoint `b3d2b35d8e95103f7e283585ff1951737eae07ab` passed:

- Rust CI `34973222111`: lockfile verification, dependency audit, `cargo fmt --check`, workspace `cargo check --all-targets --all-features`, Clippy with `-D warnings`, and `cargo test --workspace --all-features`.
- Fuzz Smoke `34973222116`: `control_frame`, `data_stream_open`, `identifiers`, `pairing_bootstrap`, and `session_auth`, each with `-runs=256` under `nightly-2026-09-12`.
- Identity golden vectors: `authority_delegation_v1_golden_vector` and `device_credential_v1_golden_vector` passed unchanged.
- Policy golden vector: `trust_revocation_transition_v1_golden_vector` passed unchanged.
- Protocol wire vectors for control envelope, pairing confirmation, data-stream open, session close, and system event all passed unchanged.
- Authority-currentness, replay, stream-currentness, simulator lifecycle, reconnect, Quinn lifecycle, resource-hardening, and Debug-redaction regressions all passed in the same full workspace test run.

The PR CI job checked GitHub's synthetic merge of this exact branch head into current `main`, so the verification also covers the current merge result.

## Dependency Audit Note

`cargo audit` produced no vulnerability failure. It emitted one allowed maintenance warning:

- `paste 1.0.15`, `RUSTSEC-2024-0436` — unmaintained.

The warning remains isolated to the Iroh M9 experiment dependency stack and is not suppressed. Re-evaluate that stack before ADR-0009 promotes any candidate into production networking.

## Deferred / External Items

These are not silently treated as solved by the in-memory Phase 1 foundation:

- Durable, rollback-resistant persistence for `OwnerAuthorityState` and production authority epochs.
- Platform key-store / secure-enclave integration and real mobile lifecycle evidence.
- Explicit authorization/subscription rules for future authority-sensitive system-event families.
- Repository branch protection and other administration-only controls.
- Windows/macOS/mobile CI matrices as the corresponding platform slices land.
- The isolated Iroh `paste 1.0.15` maintenance warning and later M9 NAT/relay/path/measurement obligations.

## Gate

The code-level foundation gate is verified. M9 remote-networking Task 4 remains blocked until PR #23 is reconciled on its final documentation head, that exact head passes CI/Fuzz, the expected PR head is merged, and the resulting `main` commit passes post-merge Rust CI.
