# Cross-Lab Foundation Threat Model

**Status:** Normative Phase 0 security analysis  
**Architecture baseline:** `docs/architecture/MASTER-ARCHITECTURE.md`, Revision 2.0  
**Security boundaries:** `docs/architecture/SECURITY-BOUNDARIES.md`  
**Applies from:** P0.2 — Threat Model and Trust Boundaries

## 1. Purpose and scope

This threat model defines the foundational security assumptions, attacker capabilities, protected assets, threats, required controls, and verification obligations that later Cross-Lab specifications and implementation must satisfy.

It is foundation-first. It covers security boundaries shared by every capability and intentionally defers platform- and feature-specific threat models until those features are designed.

P0.2 does not define exact cryptographic algorithms, pairing transcript encoding, wire serialization, key-storage APIs, or platform-specific privileged IPC mechanisms.

## 2. Security objectives

Cross-Lab must preserve the following properties:

1. Cross-Lab identity remains independent from transport and third-party library identifiers.
2. Sensitive operations require authenticated identity before trust and authorization are evaluated.
3. Trust membership never grants blanket capability authority.
4. Capability support never grants permission.
5. Policy is default-deny and operation-specific.
6. Data-plane streams cannot bypass control-plane authorization.
7. Privileged authority remains isolated and is revalidated at local privilege boundaries.
8. Revocation prevents new ordinary authority and has explicit effects on active sessions/operations.
9. Recovery authority remains independent from ordinary trust.
10. Optional relay/hub infrastructure is not an implicit identity, policy, or plaintext authority.
11. Security/privacy requirements are hard route-eligibility conditions rather than performance preferences.
12. External input, queues, streams, and task creation are bounded.
13. Sensitive material is excluded from ordinary logs and diagnostics.
14. Update distribution cannot bypass signed update authorization and rollback/freeze protections.
15. Ambiguous or incomplete security state fails closed.

## 3. Protected assets

### 3.1 Authority assets

- owner root authority;
- device-signing authority;
- administrative authority;
- recovery authority;
- device private keys and hardware-backed credentials;
- future update-signing roles and trusted update roots.

### 3.2 Authorization assets

- trust records;
- revocation state;
- credential epochs;
- capability declarations and compatibility state;
- policy rules and decisions;
- approval evidence and obligations;
- authorized operation state and lifetimes.

### 3.3 Protocol and session assets

- authenticated peer identity;
- freshness and replay state;
- protocol-version negotiation;
- logical session identity;
- channel binding;
- control-message integrity;
- data-stream authorization bindings;
- cancellation/shutdown state.

### 3.4 User-data assets

- files and metadata;
- clipboard content;
- notifications;
- camera, microphone, screen, and audio data;
- device status and presence information;
- future backup/sync/compute payloads.

### 3.5 Platform assets

- OS authentication state and credentials;
- privileged-service authority;
- system configuration;
- driver/system-extension authority;
- platform permission grants;
- local IPC endpoints.

### 3.6 Privacy and audit assets

- device identifiers and topology;
- pairing attempts;
- authentication attempts;
- presence/location-adjacent metadata;
- capability use;
- recovery activity;
- audit records.

## 4. Legitimate actors

- **Owner:** highest normal authority in an owner trust domain.
- **Owner-authorized device:** a device with valid owner authorization and non-revoked trust state.
- **Unprivileged Cross-Lab agent:** network-facing user-space runtime coordinating transports, core logic, policy, and nonprivileged adapters.
- **Platform adapter:** capability-specific user-space OS integration.
- **Privileged service/helper:** local elevated component exposing narrowly typed operations.
- **Recovery authority:** cryptographically separate authority for recovery operations.
- **Owner-operated relay/hub:** optional infrastructure that provides connectivity or selected services without becoming ordinary identity/policy authority.

## 5. Threat actors and attacker capabilities

Cross-Lab must consider:

- unauthenticated remote peers;
- on-path attackers able to observe, delay, reorder, replay, inject, redirect, or drop traffic;
- malicious or compromised paired devices;
- malicious local unprivileged processes;
- compromised Cross-Lab UI/plugin processes;
- compromised unprivileged Cross-Lab agent;
- malicious or compromised relay/hub infrastructure;
- attackers controlling a lost/stolen device and its locally available credentials;
- malicious update repositories, mirrors, or distribution paths;
- attackers attempting downgrade, replay, resource exhaustion, confused-deputy, or privilege-escalation attacks.

## 6. Guarantee boundary and assumptions

Cross-Lab does not claim to remain secure against arbitrary control of the local operating-system kernel/hypervisor, compromise of hardware roots of trust, or compromise of the owner's root/recovery private authority. When those roots are compromised, many ordinary guarantees are no longer enforceable.

This does not excuse avoidable privilege concentration. A compromised UI, plugin, network peer, relay, or ordinary user-space process must not automatically obtain privileged or recovery authority.

Physical access is not assumed to be harmless. Lost/stolen-device and local-user scenarios remain in scope to the extent platform security and Cross-Lab credential isolation can mitigate them.

## 7. Trust boundaries

The normative boundaries are defined in `docs/architecture/SECURITY-BOUNDARIES.md`:

- B1 discovery/remote transport input;
- B2 Cross-Lab authentication;
- B3 trust/revocation;
- B4 capability/policy authorization;
- B5 authorized operation lifecycle;
- B6 control/data-plane boundary;
- B7 user-space platform adapters;
- B8 privileged local IPC;
- B9 recovery authority;
- B10 relay/hub infrastructure;
- B11 update trust;
- B12 UI/future plugin authority.

## 8. Threat catalog

### TM-001 — Transport identity substitution

**Boundary/assets:** B1/B2; authenticated identity, trust records.  
**Attacker:** remote peer or on-path attacker.  
**Scenario:** attacker reuses/spoofs an IP, MAC, BLE address, USB identity, QUIC certificate identity, relay identity, Iroh identifier, or libp2p PeerId and is mistaken for a previously trusted Cross-Lab device.  
**Required controls:** Cross-Lab credential authentication independent from transport identity; current trust validation; channel binding.  
**Verification obligation:** authentication tests must prove identical transport metadata cannot substitute for a valid Cross-Lab credential.  
**Residual/deferred:** exact credential and channel-binding format is defined in P0.3/P0.7.

### TM-002 — Unauthorized device enrollment

**Boundary/assets:** pairing; owner/device-signing authority.  
**Attacker:** remote/on-path/local attacker.  
**Scenario:** attacker causes a device to be enrolled without owner intent, substitutes credentials during pairing, or races a legitimate pairing attempt.  
**Required controls:** authenticated bootstrap, freshness, transcript confirmation, explicit owner intent, mutual confirmation, owner-authorized credential issuance, cancellation.  
**Verification obligation:** pairing tests cover substitution, wrong bootstrap context, cancellation, replay, and confirmation mismatch.  
**Residual/deferred:** P0.4 defines the exact transcript/bootstrap design.

### TM-003 — Pairing secret guessing/reuse

**Boundary/assets:** pairing bootstrap.  
**Attacker:** nearby/remote attacker.  
**Scenario:** a low-entropy numeric code is treated as a strong secret or reusable pairing secret.  
**Required controls:** primary bootstrap uses high-entropy one-time material; any future numeric-code flow requires a suitable PAKE/equivalent design, rate limits, and replay resistance.  
**Verification obligation:** Phase 1 must not implement a plain numeric secret comparison as pairing authentication.  
**Residual/deferred:** numeric-code support is optional and remains deferred until separately specified.

### TM-004 — Authentication replay

**Boundary/assets:** B2; peer identity/session freshness.  
**Attacker:** network observer or malicious peer.  
**Scenario:** previously valid authentication material is replayed to establish a fresh session.  
**Required controls:** fresh challenges/nonces/transcript state, session/channel binding, replay cache/sequence semantics where needed.  
**Verification obligation:** Phase 1 negative test replays captured authentication data and expects rejection.  
**Residual/deferred:** exact nonce/transcript scheme is defined in P0.7.

### TM-005 — Protocol downgrade/version confusion

**Boundary/assets:** protocol/session compatibility.  
**Attacker:** on-path or malicious peer.  
**Scenario:** negotiation is modified or replayed so peers select an older/incompatible security behavior or interpret fields differently.  
**Required controls:** authenticated/version-bound negotiation; explicit compatibility ranges; reject unsupported mandatory semantics; canonical security transcript includes negotiated version.  
**Verification obligation:** protocol cross-version and downgrade tests.  
**Residual/deferred:** exact version rules in P0.6.

### TM-006 — Capability advertisement tampering

**Boundary/assets:** capability state, authorization.  
**Attacker:** on-path or malicious peer.  
**Scenario:** capability/version metadata is modified or used to trigger unsafe fallback behavior.  
**Required controls:** capability exchange inside authenticated/integrity-protected session; local runtime capability validation; no authorization derived from advertisement.  
**Verification obligation:** altered capability state cannot grant operations or bypass local support checks.  
**Residual/deferred:** capability identifiers/version model in P0.5/P0.6.

### TM-007 — Trust interpreted as blanket authority

**Boundary/assets:** B3/B4; policy.  
**Attacker:** malicious/compromised trusted device.  
**Scenario:** paired status alone allows screen capture, file access, command execution, or other protected capability.  
**Required controls:** default-deny operation policy; explicit capability/operation/context evaluation.  
**Verification obligation:** trusted-but-unauthorized capability requests are denied.  
**Residual/deferred:** policy model in P0.5.

### TM-008 — Policy context spoofing

**Boundary/assets:** B4; authorization context.  
**Attacker:** remote peer or compromised agent component.  
**Scenario:** peer-supplied claims such as LAN locality, presence, device type, privilege level, or approval state are trusted without local validation.  
**Required controls:** distinguish authenticated peer claims from locally derived context; policy input provenance; fail closed on unverifiable security-sensitive context.  
**Verification obligation:** policy tests reject spoofed/unverified context for restricted operations.  
**Residual/deferred:** exact context types in P0.5.

### TM-009 — Stale authorization after trust/policy change

**Boundary/assets:** B5; operation/session authority.  
**Attacker:** previously authorized peer.  
**Scenario:** an operation token/context remains valid after device revocation, policy change, expiry, or lifecycle transition.  
**Required controls:** bounded lifetimes; revocation generation/epoch binding; explicit invalidation semantics; revalidation for sensitive continuation.  
**Verification obligation:** active/future operation tests after revocation and expiry.  
**Residual/deferred:** exact operation lifecycle in P0.5/P0.7.

### TM-010 — Data-plane authorization bypass

**Boundary/assets:** B6; user data and operations.  
**Attacker:** authenticated or unauthenticated peer.  
**Scenario:** peer opens a bulk/realtime stream directly and bypasses control-plane policy.  
**Required controls:** every protected stream binds to a valid authorized `OperationId`, peer/session, capability, direction, and lifetime.  
**Verification obligation:** missing, expired, revoked, mismatched, reused-single-use, or wrong-peer operation contexts are rejected.  
**Residual/deferred:** stream-open encoding in P0.6/P0.7.

### TM-011 — Confused deputy at platform adapter

**Boundary/assets:** B7; local OS capabilities.  
**Attacker:** malicious peer or compromised core/UI.  
**Scenario:** generic adapter APIs allow a request authorized for one capability to trigger a different local action.  
**Required controls:** typed capability-specific adapter APIs; local argument validation; explicit unsupported/denied results; operation/capability binding.  
**Verification obligation:** platform boundary tests when adapters are introduced.  
**Residual/deferred:** platform-specific APIs are outside Phase 1.

### TM-012 — Privilege escalation through local IPC

**Boundary/assets:** B8; root/admin/SYSTEM authority.  
**Attacker:** compromised agent, UI/plugin, or local process.  
**Scenario:** access to privileged IPC becomes equivalent to arbitrary privileged command execution.  
**Required controls:** OS-authenticated local caller, strict typed requests, allowlisted operations, independent validation, least privilege, no generic shell/RPC escape hatch.  
**Verification obligation:** privilege-boundary tests must prove unauthorized callers and malformed/out-of-scope requests are rejected.  
**Residual/deferred:** OS-specific IPC mechanisms are platform-milestone work.

### TM-013 — Privileged helper becomes network-facing authority

**Boundary/assets:** B8; privileged service.  
**Attacker:** remote peer.  
**Scenario:** privileged helper hosts peer discovery/P2P/network RPC and therefore exposes elevated authority directly to remote input.  
**Required controls:** privileged helper remains local-only; remote peer terminates at unprivileged agent; no general network listener in helper.  
**Verification obligation:** architecture/platform reviews and boundary tests.  
**Residual/deferred:** none; this is a permanent invariant.

### TM-014 — Malicious or compromised trusted peer

**Boundary/assets:** all capability/user-data assets.  
**Attacker:** paired device with valid credentials.  
**Scenario:** trusted device requests operations outside intended scope, floods requests, abuses old permissions, or attempts lateral privilege escalation.  
**Required controls:** per-operation policy, bounded session/operation scope, rate/resource controls, revocation, local platform permissions, audit, high-risk approval obligations.  
**Verification obligation:** simulator tests treat authenticated/trusted peers as potentially malicious inputs.  
**Residual/deferred:** feature-specific abuse constraints added with each capability.

### TM-015 — Relay/hub impersonation or content tampering

**Boundary/assets:** B10; connectivity, metadata, payload integrity.  
**Attacker:** compromised relay/hub or on-path attacker.  
**Scenario:** infrastructure injects/changes traffic, lies about peer location/presence, or attempts to authorize a peer.  
**Required controls:** end-to-end Cross-Lab authentication/integrity independent of relay; relay metadata not sufficient for trust; authorization remains endpoint-owned.  
**Verification obligation:** later relay tests simulate malicious/incorrect relay metadata and modified traffic.  
**Residual/deferred:** remote networking implementation is selected at M9.

### TM-016 — Metadata/privacy leakage through discovery, relay, audit, or logs

**Boundary/assets:** privacy/audit assets.  
**Attacker:** local observer, relay, log reader, network observer.  
**Scenario:** device graph, presence, sensitive capability use, file names, clipboard/media contents, secrets, or authentication material leak through routine telemetry/logs.  
**Required controls:** data minimization, payload redaction, no secret logging, bounded audit fields, privacy-conscious discovery.  
**Verification obligation:** logging/audit tests and review prohibit secret/payload fields.  
**Residual/deferred:** storage/access controls for persistent audit logs are specified when persistence is introduced.

### TM-017 — Parser/resource exhaustion

**Boundary/assets:** availability, memory/CPU/battery.  
**Attacker:** unauthenticated or authenticated peer.  
**Scenario:** oversized frames, huge collection counts, decompression/parse amplification, excessive concurrent sessions/streams, or unbounded task/queue creation exhaust resources.  
**Required controls:** bounded frames/counts/queues/concurrency, backpressure, timeouts, cancellation, cheap rejection before expensive work.  
**Verification obligation:** oversized-frame, queue-saturation, cancellation, and clean-shutdown tests; fuzz externally supplied parsers where useful.  
**Residual/deferred:** exact limits chosen with protocol/performance evidence.

### TM-018 — Reconnect/route change weakens security binding

**Boundary/assets:** B2/B5/B6; session authority.  
**Attacker:** on-path or malicious peer.  
**Scenario:** a reconnect or future route migration reuses stale authentication/authorization or binds a stream to a different peer/path without equivalent security properties.  
**Required controls:** fresh authentication on reconnect; explicit session restoration rules; route eligibility before scoring; peer/channel binding; no automatic authority transfer across unrelated transports.  
**Verification obligation:** reconnect-after-disconnect and wrong-channel-binding tests.  
**Residual/deferred:** seamless cross-transport migration is not required in Phase 1.

### TM-019 — Recovery authority abuse

**Boundary/assets:** B9; recovery authority/device state.  
**Attacker:** malicious normal peer, stolen device, relay, or recovery-credential thief.  
**Scenario:** ordinary credentials are used for recovery; recovery command is replayed; recovery capability becomes covert surveillance; or revoked normal peer regains normal access through recovery.  
**Required controls:** separate recovery credentials/domain; narrow recovery namespace; replay resistance; platform capability checks; visible/auditable actions; no implicit promotion back to ordinary trust.  
**Verification obligation:** recovery tests for authority separation, replay, revoked-device behavior, and unsupported operations.  
**Residual/deferred:** exact recovery command/authentication format in P0.8.

### TM-020 — Update repository compromise, rollback, or freeze

**Boundary/assets:** B11; installed code and privileged components.  
**Attacker:** malicious repository/mirror or compromised signing role.  
**Scenario:** attacker distributes arbitrary artifact, old vulnerable version, or indefinitely freezes clients on stale metadata.  
**Required controls:** TUF-style role separation, signed metadata/artifacts, version/rollback checks, expiry/freeze protection, compromise recovery, trusted-root rotation.  
**Verification obligation:** updater tests when implemented must cover bad signatures, rollback, expired metadata, freeze, and role compromise/recovery cases.  
**Residual/deferred:** role topology and rollback policy in P0.8; `tough` remains candidate implementation.

### TM-021 — UI/plugin inherits core or privileged authority

**Boundary/assets:** B12; owner/device keys, policy, privileged service.  
**Attacker:** compromised UI/plugin.  
**Scenario:** UI/plugin can directly use device private keys, bypass policy, access arbitrary network/filesystem resources, or invoke privileged service operations.  
**Required controls:** UI requests through core APIs; private keys encapsulated; future plugin capability broker/sandbox; privileged IPC not ambient.  
**Verification obligation:** UI/plugin boundary tests when those components exist.  
**Residual/deferred:** plugin runtime is deferred until capability ABI stabilizes.

### TM-022 — Ambiguous failure becomes authorization success

**Boundary/assets:** all protected operations.  
**Attacker:** any actor able to trigger malformed/incomplete state or service failures.  
**Scenario:** timeout, unknown state, unsupported capability, parser failure, missing policy input, unavailable verifier, or partial reconnect is interpreted as permission to continue.  
**Required controls:** explicit typed failures; deny/terminate on incomplete security state; availability degradation must not become authorization success.  
**Verification obligation:** negative tests for malformed, unsupported, expired, cancelled, and verifier-failure paths.  
**Residual/deferred:** none; permanent invariant.

## 9. Compromised paired-device model

A paired device is not equivalent to the owner root. A compromised paired device may possess valid device credentials and still act maliciously.

Therefore the architecture must retain:

- per-capability and per-operation policy;
- narrow authorization contexts;
- explicit expiry and cancellation;
- revocation;
- separate recovery authority;
- separate privileged-service validation;
- local OS permission enforcement;
- auditability for sensitive actions;
- optional owner/biometric/second-approval obligations for high-risk operations.

## 10. Session/revocation requirements derived from threats

Later session specifications and implementation must ensure:

- every new connection is authenticated independently of address continuity;
- reconnect performs fresh authentication appropriate to the protocol;
- revocation prevents creation of new ordinary operations;
- active operations have explicit revocation behavior;
- authorization contexts have bounded scope and lifetime;
- replayed control messages cannot recreate expired/revoked authority;
- channel/peer binding survives multiplexing and is re-established on reconnect;
- route changes cannot silently relax authentication, privacy, or policy constraints.

## 11. Privileged-service requirements derived from threats

The privileged service must remain local-only, narrow, typed, independently validating, and capability-specific. It must not become a general-purpose execution engine.

A compromised unprivileged agent is expected to be able to request privileged operations that the agent itself could legitimately request. The helper boundary must prevent that compromise from expanding into arbitrary privileged authority beyond the narrow exposed operations and their local validation rules.

## 12. Recovery requirements derived from threats

Recovery authority is independent from normal trust and uses a separate command namespace. Recovery must remain useful after ordinary trust is revoked where the platform permits, without granting the lost/revoked device normal ecosystem access.

Every recovery operation must define platform support, owner authorization, replay behavior, audit/visibility, and failure semantics before implementation.

## 13. Relay/hub assumptions

Relays/hubs may be unavailable, compromised, malicious, or metadata-observing. Endpoints therefore own identity authentication and operation authorization. Owner-hosting reduces third-party dependence but does not remove the need for end-to-end controls.

## 14. Update assumptions

Transport security to an update server is insufficient to authorize software. Cross-Lab's future updater must verify signed update metadata/artifacts and enforce rollback/freeze/role rules independently from the distribution path.

## 15. Audit and logging rules

Normal logs must never contain private keys, recovery material, reusable credentials, raw authentication secrets, pairing bootstrap secrets, plaintext files/clipboard/media payloads, or equivalent sensitive content.

Audit records should be structured and minimal. Sensitive audit storage/access policy is specified alongside persistence when introduced.

## 16. Resource-safety rules

External inputs are adversarial. Parsers, queues, streams, session counts, task creation, and buffered payloads require explicit bounds. Cross-Lab uses backpressure and cancellation instead of unbounded accumulation. Resource budgets for unauthenticated peers should be stricter than for established sessions.

## 17. Security verification matrix

| Threats | Primary verification stage |
|---|---|
| TM-001, TM-004 | P0.7 specification; Phase 1 authentication/session tests |
| TM-002, TM-003 | P0.4 specification; Phase 1 pairing tests |
| TM-005, TM-006 | P0.6 specification; protocol compatibility tests |
| TM-007, TM-008, TM-009 | P0.5 specification; Phase 1 policy/revocation tests |
| TM-010 | P0.6/P0.7; Phase 1 authorized-stream tests |
| TM-011 | platform capability milestones |
| TM-012, TM-013 | privileged platform milestones |
| TM-014 | Phase 1 negative tests plus every capability threat model |
| TM-015 | M9 relay/NAT evaluation and later network tests |
| TM-016 | audit/persistence and platform capability tests |
| TM-017 | P0.6/P0.7; parser fuzzing, queue/backpressure/shutdown tests |
| TM-018 | P0.7; Phase 1 reconnect/channel-binding tests |
| TM-019 | P0.8; future recovery tests |
| TM-020 | P0.8; future updater tests |
| TM-021 | desktop/plugin milestones |
| TM-022 | all security-sensitive layers |

## 18. Residual risk and deferred work

This model intentionally does not resolve:

- exact owner/device/recovery credential formats;
- exact key algorithms and key storage APIs;
- exact pairing handshake/transcript;
- final wire serialization and canonical signing encoding;
- exact session key schedule/channel-binding representation;
- OS-specific privileged IPC mechanisms;
- remote NAT/relay implementation;
- persistent audit storage controls;
- plugin runtime;
- capability-specific abuse/privacy controls for features not yet designed.

These items are not omissions from the architecture; they have explicit resolution milestones. A later specification may strengthen this model. Weakening a foundational boundary requires an ADR and architecture approval.

## 19. Review rule

The threat model must be revisited when a change materially affects identity, trust, cryptographic formats, public protocol semantics, privileged boundaries, recovery authority, update trust, transport/session abstraction, mandatory infrastructure, plugin authority, or a new high-impact capability family.
