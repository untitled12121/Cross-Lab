# Foundation Security Remediation Implementation Plan

> Execute this plan on `m9-remote-networking` before M9 Task 4. Preserve existing approved architecture; use regression-first TDD for each security finding.

**Goal:** Close the second foundation audit findings so pairing, trust/credential rotation, approval provenance, control replay/event authorization, transport task ownership, debug privacy, and production identifier generation match the Phase 0/Phase 1 specifications.

**Architecture:** Keep identity, policy, protocol, core orchestration, simulator, and transports separated. Security state remains locally authoritative. Cross-crate APIs must make invalid authority creation hard or impossible; transport-specific fixes stay in their transport crate. No M9 candidate type enters production/domain APIs.

**Verification:** Each slice starts with a failing regression, implements the smallest typed fix, then runs the focused test and the full Rust CI gate. Parser/security changes also retain Fuzz Smoke coverage.

---

## Task 1 — Pairing currentness and initial credential epoch

**Files:** `crates/core/src/pairing/{invitation,flow}.rs`, `crates/core/tests/pairing*.rs`, simulator/transport pairing fixtures.

- Keep the already verified RED test proving pairing can currently issue epoch 9 although initial enrollment requires epoch 0.
- Remove caller-selected epoch from `PairingInviterFlow::issue_joiner_credential`; pairing always issues epoch 0.
- Reject a nonzero credential at the joiner pairing boundary even if its signature is otherwise valid.
- Add local deadline metadata to `PairingInvitation` and require an explicit caller-supplied local time at authority-changing pairing steps; expired invitations transition terminal and cannot commit trust.
- Use deterministic integer/monotonic-style test time; never trust a peer timestamp.

## Task 2 — Credential rotation and active-session invalidation

**Files:** `crates/policy/src/trust/*`, `crates/core/src/{session,state/control}*`, affected tests.

- Replace raw public numeric trust-epoch advancement with a transition that verifies the successor `DeviceCredential` against current owner/delegation authority, exact owner/device identity, and exact `N + 1` epoch.
- Advance `trust_revision` when the accepted credential epoch changes so existing operations/sessions become stale.
- Make control/session currentness validate both trust revision and accepted credential epoch against the authenticated snapshot; mismatch fails closed.
- Cover old-session rejection after a locally accepted rotation and rejection of skipped/stale/wrong-device successor credentials.

## Task 3 — Trusted-state and approval provenance

**Files:** `crates/policy/src/{trust,authorization}/*`, `crates/core/src/pairing/*`, policy/core tests.

- Remove APIs that allow arbitrary callers to mint authoritative `Trusted` or `VerifiedApproval` state without evidence.
- Establish initial trust only through an owner-authorized, auditable transition produced after pairing proof verification.
- Model Phase 1 synthetic owner approval as typed locally verified evidence with explicit scope/lifetime and cryptographic/local-authority verification, not a public value constructor.
- Preserve the pure policy evaluator: it consumes already-verified evidence and performs no UI/network/private-key operations.

## Task 4 — Request replay state and local retry authority

**Files:** `crates/core/src/control/mod.rs`, control/simulator tests, protocol docs only if contract clarification is required.

- Stop treating peer-declared `RetryClass` as local permission to repeat a protected action.
- Reject duplicate request IDs under a bounded locally maintained replay window regardless of peer retry claim.
- Reclaim completed-request state without permanent capacity exhaustion; keep memory strictly bounded.
- Preserve ordered message-sequence replay protection and ownership-preserving resource errors.

## Task 5 — Event authorization

**Files:** `crates/core/src/control/mod.rs`, `crates/policy/*` only if the existing specification supports an event-policy key, simulator tests.

- First reconcile the Phase 0 policy/control specifications.
- If capability events are intended to require policy permission, add a typed local authorization check and regression coverage.
- If the existing contract defines trust + negotiated capability as the complete Phase 1 event gate, document that explicitly and test it; do not invent a new authorization grammar silently.

## Task 6 — Quinn drop/task ownership

**Files:** `transports/quic/src/{connection,stream,tests}.rs`.

- Prove dropping a transport connection without explicit async shutdown does not leave authority-bearing tasks detached.
- Add synchronous Drop cleanup that closes the connection, closes task admission, and aborts/drains owned tasks; explicit `shutdown().await` still joins tasks cleanly.
- Cover peer-visible close and idempotent shutdown/drop behavior.

## Task 7 — Debug/privacy hardening

**Files:** `crates/core/src/session/auth.rs`, `crates/core/src/pairing/transcript.rs`, privacy tests/assessment.

- Replace derived Debug on authentication proofs/transcripts and pairing transcript with deliberate redacted Debug output.
- Keep useful type/profile/length metadata only; never expose signatures, nonces, binding-derived material, bootstrap secrets, or payloads.
- Add regressions that use sentinel bytes and assert they never appear in ordinary Debug output.

## Task 8 — Production secure-random identifier construction

**Files:** typed ID modules across `identity`, `policy`, `core`, and `protocol`; manifests only where a justified crypto dependency is missing; tests.

- Add OS-CSPRNG-backed constructors for locally created random identifiers required by the specifications (`OwnerId`, `DeviceId`, pairing/transition/rule/operation/request/event/stream IDs as applicable).
- Keep byte constructors for verified parsing, wire conversion, and deterministic tests; do not silently use deterministic RNG in production constructors.
- Keep derived identifiers such as `SessionId` derived rather than randomly generated.

## Task 9 — Final reconciliation and gate

**Files:** `security/M9-FOUNDATION-HARDENING-ASSESSMENT.md`, `docs/development/CURRENT.md`, PR #19 metadata; architecture docs only where implementation revealed a real contract ambiguity.

- Re-run dependency audit, format, workspace check, Clippy `-D warnings`, full workspace tests, and Fuzz Smoke on the exact final head.
- Update the hardening assessment with each resolved finding and any explicitly deferred non-Phase-1 item.
- Update `CURRENT.md` with exact verified commit/run IDs and make M9 Task 4 the next task only after the hardening gate is fully GREEN.
- Keep PR #19 draft/unmerged until the remaining M9 plan is completed and verified.