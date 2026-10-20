# Cross-Lab Phase 2 — Security assessment

**Assessment date:** 2026-10-20  
**Reviewed baseline:** `main` at `fb92d0d82bc304c8ebc67fd6d175168f52de40c8` (product sources last full-tested at `e10fa6253e6087cd073077944e5b19f80f962ff2`)  
**Hardening evidence:** [PR #121](https://github.com/untitled12121/Cross-Lab/pull/121) merged at `b4aea96924ccc22f89a1d74f3e7c60d7cf2672f8`; exact-head [CI 39476668443](https://github.com/untitled12121/Cross-Lab/actions/runs/39476668443) passed Rust, Android, formatting and gate. [PR #122](https://github.com/untitled12121/Cross-Lab/pull/122) addresses a further Linux policy CAS race; verify its exact-head checks before considering that additional change integrated.  
**Assessment type:** risk-based source review, automated CI/RustSec evidence review, focused static search, and targeted security regression tests. **Not an independent line-by-line security certification, active exploit/pentest or real-device test.**

## Scope and threat model

The Git tree contains 562 blobs, including 313 Rust `.rs` files and 100 Kotlin `.kt` files. The assessment inventories the workspace/dependency lockfile, reviews existing full GitHub Actions job evidence, and inspects critical Linux/Android product paths rather than certifying every implementation or third-party dependency.

Normative security sources: [Foundation Threat Model](../../security/THREAT-MODEL.md), [Master Architecture](../architecture/MASTER-ARCHITECTURE.md) v2.11, [Session/Transport](../architecture/SESSION-TRANSPORT.md), [Policy Authorization](../architecture/POLICY-AUTHORIZATION.md), ADR-0020–0023, including [terminal revocation](../adr/ADR-0022-durable-owner-revocation.md).

Reviewed representative code: `transports/quic/src/{config,session,trusted_session,tls,stream}.rs`, `crates/runtime/src/{actor,node,stream,tests}.rs`, `crates/agent/src/presence/{runner,agent,tests}.rs`, `crates/identity-store/src/{envelope,product}.rs`, Linux identity/policy/transfer/audit adapters, Android identity/Keystore/signing/policy/clipboard/transfer/notification adapters, Android manifest and Gradle configuration, and CI scripts/workflows. Search screening covered handwritten `unsafe`, process spawning, unbounded state reads/queues, transport certificate-verifier exemptions and logging. No secrets or production private test inputs were used.

## Evidence: dependency and build security

**Integrated product baseline CI:** [run 39452949225](https://github.com/untitled12121/Cross-Lab/actions/runs/39452949225), source `e10fa6253e6087cd073077944e5b19f80f962ff2`, full profile, **success**: dependency audit, Rust formatting/check/Clippy/workspace tests, Linux normal/development builds, Android unit tests and normal/development APK builds, and strict selected-check gate. This is a prior exact-head software check, not proof of runtime resistance to all attackers.

**RustSec audit log at this exact commit:** cargo-audit 0.22.2 fetched 1,345 security advisories and scanned 1,064 Cargo.lock package entries. It reported **no vulnerability errors, but 10 allowed warnings**:

| Locked crate/version | RustSec finding | Classification |
| --- | --- | --- |
| basic-toml 0.1.10 | [RUSTSEC-2026-0374](https://rustsec.org/advisories/RUSTSEC-2026-0374) | Unmaintained |
| instant 0.1.13 | [RUSTSEC-2024-0384](https://rustsec.org/advisories/RUSTSEC-2024-0384) | Unmaintained |
| paste 1.0.15 | [RUSTSEC-2024-0436](https://rustsec.org/advisories/RUSTSEC-2024-0436) | Unmaintained |
| rustls-pemfile 2.2.0 | [RUSTSEC-2025-0134](https://rustsec.org/advisories/RUSTSEC-2025-0134) | Unmaintained |
| rustybuzz 0.20.1 | [RUSTSEC-2026-0206](https://rustsec.org/advisories/RUSTSEC-2026-0206) | Unmaintained |
| ttf-parser 0.25.1 | [RUSTSEC-2026-0192](https://rustsec.org/advisories/RUSTSEC-2026-0192) | Unmaintained |
| zbus_names 4.3.4 | [RUSTSEC-2026-0371](https://rustsec.org/advisories/RUSTSEC-2026-0371) | Unmaintained |
| zvariant 5.15.0 | [RUSTSEC-2026-0370](https://rustsec.org/advisories/RUSTSEC-2026-0370) | Unmaintained |
| zvariant_derive 5.15.0 | [RUSTSEC-2026-0372](https://rustsec.org/advisories/RUSTSEC-2026-0372) | Unmaintained |
| yoke-derive 0.8.3 | Cargo audit yanked warning | Yanked; no RustSec ID in log |

The reviewed Cargo.lock has `quinn-proto 0.11.18` and `memmap2 0.9.11`, satisfying published patched ranges for [RUSTSEC-2026-0338](https://rustsec.org/advisories/RUSTSEC-2026-0338) and [RUSTSEC-2026-0186](https://rustsec.org/advisories/RUSTSEC-2026-0186), respectively. The audit does not cover Android/Gradle/JNA dependencies, OS dependencies, private patches outside Cargo.lock, or future advisories. **Green cargo-audit is not a clean dependency inventory.**

## Confirmed findings and remediation

### SEC-P2-001 — committed identity loss could lead to fresh owner bootstrap (High, local state-loss/rollback precondition)

**Affected:** Android `AndroidIdentityStore.loadPayload()/loadBundleOrNull()` and Android owner bootstrap; Linux `LinuxIdentityStore.load_payload()/commit_payload_checked()` and Linux desktop bootstrap.

**Before patch:** A missing identity file was interpreted as a fresh installation without checking surviving currentness anchors. A concurrent or prior failed persistence step, local deletion, or snapshot rollback could cause bootstrap to produce a fresh owner identity instead of signaling unavailable committed security state. This risks losing the persisted evidence of trust and terminal device revocations. **It does not establish that a previously revoked peer can automatically authenticate or that remote attackers can cause this state.**

**Patch:** Fail closed whenever the protected store has a previously committed currentness key but the state file is absent. Require a sole currentness anchor for the exact revision; validate the local envelope/anchor revision before a commit. Refuse new owner bootstrap when signing authority already exists without a matching identity snapshot. The Linux side checks Secret Service keys, the Android side Android Keystore/wrapped signer state.

**Focused case:** deletion of the committed identity state while the protected anchor remains must return a typed failure, never `None`/`null`; a truly pristine installation is still permitted. Simulate incorrect revision, extra stale anchor and absent anchor. Rust `deleted_identity_state_with_existing_anchor_denies_rebootstrap` and Android `IdentityStoreCurrentnessGuardTest`. Actual Secret Service/Android Keystore fault injection is not claimed.

**Threat mapping:** TM-009, TM-017; ADR-0015 currentness model and ADR-0022 persist-before-apply signed terminal revocation.

### SEC-P2-002 — unbounded security-state file reads (Medium, local resource-exhaustion precondition)

**Affected:** Linux identity/policy `read_to_end`; Android identity, policy, signer and retained file-transfer `AtomicFile.readFully()`.

**Before patch:** Local state files were fully materialized in memory before encoded-field length checks. Oversized or growing local files could exhaust memory before cleanly returning a size error.

**Patch:** Hard upper bounds enforced during reads before decoding. Rust uses `Read::take(max+1)` with rejected overflow; Android uses `readBoundedState` with a fixed 8KiB read buffer and bounded aggregate. Caps reflect encoded fields (identity: 3×16MiB, policy: 3×2MiB, signer: 3×16KiB) or approved retained transfer state (1MiB).

**Focused cases:** Rust `oversized_identity_file_is_bounded_before_decoding` and `oversized_policy_file_is_bounded_before_decoding`; Android `BoundedAtomicStateTest` covers exact cap and a hostile continually growing input.

**Threat mapping:** TM-017 (bounded allocations and low-cost malformed-input rejection). This is local-file availability hardening, not a demonstrated network exploit.

### SEC-P2-003 — Linux policy commit race bypassed expected-revision CAS (High, concurrent local writer precondition)

**Affected:** `apps/desktop/src/features/policy_store/linux.rs`, `LinuxPolicyStore::commit`.

**Before patch:** Separate callers could both load and verify the same current revision, then independently issue new Secret Service currentness records and atomically write different next-revision envelopes. The caller's `expected_revision` check was not under a cross-process mutex; a last-writer-wins update or inconsistent anchor state could violate the owner policy compare-and-swap guarantee. This requires concurrent local writers or a compromised local process with write authority; it does not establish a remote privilege escalation.

**Patch:** Create a private `store-v1.lock` in the same protected policy-state directory and acquire an exclusive `File::try_lock` for the entire load/compare-and-swap, Secret Service anchor update, on-disk snapshot replacement, and anchor cleanup. A contending writer fails explicitly with `RevisionConflict`; no blocking executor call is used. Another caller can retry only after loading the new revision.

**Focused case:** `concurrent_policy_writers_cannot_share_the_same_revision` holds an OS lock in one writer, verifies a second writer's lock request is refused with a typed conflict, then verifies the lock is reacquirable after release. The test does not emulate a running Secret Service or system-crash interleavings.

**Threat mapping:** TM-009 and ADR-0019 (persisted exact owner policy with CAS and fail-closed currentness).

## Other boundaries reviewed — evidence and limits

- **Identity, pairing and revocation:** Ed25519 signing/delegation, durable signed trust/revocation, Android wrapping under Keystore AES-GCM, Linux Secret Service storage, default-deny on ambiguous state. Positive patterns; platform store race/crash cases deserve integration validation.
- **Transport and session:** Quinn/rustls TLS 1.3; bounded timeouts, receive windows and streams; explicit authenticated Cross-Lab credentials, fresh session proofs and channel binding; tests for proof replay and wrong binding. `ChannelOnlyServerCertVerifier` intentionally does not validate the ephemeral server certificate against a conventional CA, because Cross-Lab separately authenticates the peer after handshake. This is a **security-sensitive assumption**, not equivalent to ordinary publicly verified TLS; validate no user payload/authority before the Cross-Lab authenticated proof, and retain negative/fuzz testing.
- **Authorization and operations:** session/trust/peer-policy binding, per-operation checks, cancellation/revocation invalidation, and data-stream admission. The runtime contains tests rejecting stale, revoked, mismatched, unauthorized operation streams; this review did not formally verify all interleavings.
- **Android platform surface:** `allowBackup=false`; notification listener is protected by Android `BIND_NOTIFICATION_LISTENER_SERVICE`, plus independent OS/owner/peer/content consent; document picker/SAF permissions are used for file destinations. File publish via document providers may be non-atomic; no physical provider failure behavior is claimed.
- **Resource and privacy controls:** bounded QUIC queues/window limits, Android bounded worker pools, bounded protocol frames, plain-text notification redaction, protected audit/event minimization, Linux 0600 files. Check operational availability under real concurrent bursts separately.
- **Supply chain:** pinned GitHub Action commits and Gradle distribution SHA-256 are positive controls. RustSec's package coverage does not establish absence of Gradle/Maven advisories or that every unmaintained crate is exploitable.

## Residual risk and next independent security work

1. **Triage nine unmaintained crates and the yanked `yoke-derive 0.8.3`**, recording whether each is transitive or runtime-reachable and whether a tested replacement can be made without destabilizing GPUI/Linux stacks; do not blindly upgrade across pinned dependency constraints.
2. **Add Android Gradle/Maven dependency vulnerability and provenance screening.** RustSec does not examine JNA/AndroidX/MLKit or Gradle plugins.
3. **Targeted OS-backed fault injection:** missing/duplicated Keystore keys, Secret Service anchor rollback or duplicate, power-loss during state replacement; the included tests model critical guards but do not create genuine protected OS keys.
4. **Exercise adversarial protocol fuzzing** and QUIC packet/resource and authenticated-peer flood scenarios beyond existing smoke tests; separately analyze session/tls custom verifier's proof sequencing and early-data disabled invariant.
5. Conduct external/manual app security review before broad deployment, including Android SAF provider interruptions, protected notifications and UI approval origin.

**Conclusion:** The prior product baseline passed automated software CI/RustSec *vulnerability* screening but carried nonzero maintenance/supply-chain warnings. Identity currentness and bounded-read protections in PR #121 are merged and passed their exact-head Rust/Android/CI Gate checks. The additional Linux policy CAS guard in PR #122 requires its own exact-head verification and merge. No all-code or independent penetration-test certification is implied. Owner device acceptance remains a later, separately managed activity.
