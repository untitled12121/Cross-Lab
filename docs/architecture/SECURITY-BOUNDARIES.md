# Cross-Lab Security Boundaries

**Status:** Normative Phase 0 security boundary reference  
**Architecture baseline:** `docs/architecture/MASTER-ARCHITECTURE.md`, Revision 2.0  
**Applies from:** P0.2 — Threat Model and Trust Boundaries

## 1. Purpose

This document defines where Cross-Lab changes trust level and what validation is required before data, identity, authorization, or privileged authority crosses each boundary. It is intentionally shorter and more operational than the full threat model.

The governing rule is:

> Untrusted input may become authenticated identity; authenticated identity may become trusted membership; trusted membership may receive operation-specific authorization; authorized operations may reach ordinary or privileged implementations only through their defined boundary checks.

No later layer may infer an earlier decision that was not explicitly established.

## 2. Trust zones

Cross-Lab uses the following conceptual trust zones:

```text
External / Untrusted
    remote peer
    discovery metadata
    network path
    relay / signalling infrastructure
          |
          v
Network-facing User Agent
    bounded transport input
    protocol parser
    Cross-Lab authentication
    trust validation
    policy evaluation
    logical session / operation lifecycle
          |
          +----> user-space platform adapters
          |
          v
Authenticated Local Privilege IPC
          |
          v
Narrow Privileged Service / Helper
          |
          v
OS privileged APIs / drivers / system extensions
```

Recovery uses a separate authority path:

```text
ordinary device trust ----X----> revoked/lost device ordinary access

recovery authority ------------> recovery verifier
                                   |
                                   v
                             permitted recovery action
```

Optional relays and hubs are connectivity infrastructure. They are not owner identity, device trust, policy, or privileged-operation authorities.

## 3. Boundary B1 — Discovery and remote transport input

### Inputs

- IP/QUIC/TCP packets;
- BLE, USB, local-network, relay, or future transport metadata;
- discovery advertisements;
- peer-provided transport identifiers and addresses.

### Required treatment

All inputs are untrusted until parsed within explicit resource limits and bound to an authenticated Cross-Lab identity.

Transport identifiers such as IP addresses, MAC addresses, Bluetooth addresses, USB identities, QUIC certificates, Iroh identifiers, libp2p PeerIds, relay identities, or OS account names must not establish Cross-Lab trust by themselves.

### Required checks

- message/frame size limits;
- bounded collection counts;
- bounded concurrent unauthenticated work;
- protocol/version sanity before expensive processing;
- explicit cancellation and timeouts where work can block;
- authentication before sensitive operations.

### Prohibited implicit authority

Discovery success, network proximity, LAN membership, possession of a transport address, or successful transport encryption must not imply device membership or operation permission.

## 4. Boundary B2 — Cross-Lab authentication

Authentication establishes which Cross-Lab principal is participating in the current exchange. It does not authorize capabilities.

### Required checks

- credential validity;
- signer/authority role validity;
- owner/domain relationship where required;
- credential epoch/currentness;
- freshness and anti-replay state required by the active protocol;
- channel binding to the current protected transport/session context;
- explicit protocol/domain separation for signed material.

Authentication failure must fail closed.

A new connection or reconnect cannot inherit identity solely because a previous connection from the same address or transport library identifier was authenticated.

## 5. Boundary B3 — Trust and revocation

Trust answers whether an authenticated principal belongs in the relevant Cross-Lab trust relationship.

### Required checks

- current trust record;
- current revocation state;
- credential/device lifecycle restrictions;
- recovery-only or quarantined restrictions where applicable.

Trust does not imply permission. A trusted device still requires policy authorization for protected operations.

Revocation must invalidate the ability to create new ordinary authorized operations. Session and operation specifications must define how active work reacts to revocation; they may not leave the behavior implicit.

## 6. Boundary B4 — Capability and policy authorization

Capability describes what the local/remote implementation can support. Policy decides whether a specific request may proceed.

### Required inputs to authorization

Where relevant:

- authenticated source identity;
- destination identity;
- requested `CapabilityId` and operation;
- negotiated capability/version compatibility;
- trust and revocation state;
- risk/lifecycle/recovery state;
- network classification and route-security properties;
- requested privilege level;
- owner/user approval evidence;
- time/expiry context;
- policy rules and obligations.

### Decision model

Authorization uses a primary effect such as `Allow`, `Deny`, or `Ask` plus explicit constraints/obligations. Context signals such as presence, LAN locality, or device integration level may constrain policy but must not bypass authentication or required owner approval.

Incomplete, ambiguous, unsupported, expired, or inconsistent authorization state is denied.

## 7. Boundary B5 — Authorized operation lifecycle

A successful policy decision creates a bounded operation context rather than ambient authority.

An authorized operation must carry sufficient state to determine:

- who requested it;
- which destination is acting;
- which capability/operation is authorized;
- applicable constraints;
- lifetime/expiry;
- session binding where required;
- whether it remains valid after policy, trust, or revocation changes.

Operation identifiers must be unguessable or otherwise non-authorizing by themselves. Possession of an `OperationId` is never a substitute for the authenticated/session context required by its specification.

## 8. Boundary B6 — Control plane to data plane

Bulk/realtime streams must not create authority independently.

The normative sequence is:

```text
control request
    -> authenticate / trust / policy
    -> authorized operation
    -> OperationId + constraints
    -> data-stream open bound to that operation
    -> revalidate active/session/peer constraints
    -> accept or reject
```

A stream request is rejected when the operation is missing, expired, revoked, already consumed when single-use, bound to another peer/session, incompatible with the requested stream type, or otherwise invalid.

This rule applies even when control and data share one QUIC connection.

## 9. Boundary B7 — User-space platform adapters

Platform adapters expose capability-specific OS behavior to the unprivileged agent/core.

They must:

- report actual platform/runtime support truthfully;
- respect local OS permission state;
- validate capability-specific inputs;
- return explicit unsupported/denied/error states;
- avoid silently escalating to privileged helpers.

Platform capability availability is not proof that a remote peer is authorized to use it.

## 10. Boundary B8 — Privileged local IPC

The privileged service is a separate authority boundary. Authorization performed by the network-facing agent is necessary but not sufficient for privileged execution.

### Required checks

- local-only IPC endpoint;
- OS-appropriate caller authentication;
- strict typed request decoding;
- request/capability allowlisting;
- validation of operation scope and authorization evidence where required;
- local platform privilege/permission checks;
- bounded input/resource use;
- explicit denial of unsupported or ambiguous requests.

### Prohibited designs

The privileged service must not provide:

- a general P2P listener;
- peer discovery or relay logic;
- generic shell execution as an escape hatch;
- unrestricted generic RPC forwarding;
- plugin runtime;
- desktop UI;
- unrelated sync/network responsibilities.

A compromised user-space agent must not automatically obtain arbitrary root/admin/SYSTEM capability merely because it can reach the IPC endpoint.

## 11. Boundary B9 — Recovery authority

Recovery is cryptographically and semantically separate from ordinary device trust.

Recovery commands require:

- valid recovery authority;
- recovery-specific domain/operation separation;
- freshness/replay protection;
- narrow recovery command scope;
- platform capability validation;
- visibility/audit appropriate to the action.

Ordinary device credentials cannot invoke recovery operations unless an explicit recovery specification grants that authority. Revoking ordinary trust must not silently revoke or grant recovery authority.

Recovery functionality must not be repurposed into covert surveillance.

## 12. Boundary B10 — Optional relay and hub infrastructure

A relay/hub may assist presence, signalling, encrypted relay, cache, update mirror, or future owner-controlled services. It must be treated as potentially unavailable, malicious, or compromised unless a specific feature explicitly gives it additional authority.

By default it must not possess:

- owner/device private keys;
- recovery private keys;
- plaintext capability payload authority;
- permission to authorize operations;
- privileged-service authority.

Cross-Lab must not require a Cross-Lab-operated trusted cloud account for ordinary operation.

## 13. Boundary B11 — Update trust

Update distribution is not equivalent to update authorization.

Mirrors, repositories, CDNs, owner hubs, and network paths may deliver update material but cannot authorize arbitrary artifacts merely by serving them.

The update design must independently validate trusted metadata/signing roles, artifact integrity, version/rollback rules, freeze resistance, and compromise-recovery policy before privileged installation.

Exact TUF role definitions are specified in P0.8.

## 14. Boundary B12 — UI and future plugins

The desktop/mobile UI is an interaction surface, not an authority root. It may request actions through defined controllers/core APIs but must not receive private signing keys or privileged-process authority by default.

Future plugins are less trusted than the core. Plugins must use explicit capability APIs and policy mediation, and must not inherit unrestricted filesystem, network, device-key, recovery, or privileged-service access.

## 15. Canonical authorization path

For a normal remote request:

```text
untrusted bytes
  -> bounded parse
  -> peer authentication
  -> trust/revocation validation
  -> capability/version validation
  -> policy evaluation
  -> bounded operation context
  -> ordinary adapter OR authorized data-plane binding
```

For a privileged remote request:

```text
untrusted bytes
  -> bounded parse
  -> peer authentication
  -> trust/revocation validation
  -> capability/version validation
  -> policy evaluation
  -> bounded operation context
  -> authenticated local IPC
  -> local caller validation
  -> privileged request validation
  -> narrow OS operation
```

No optimization may remove the logical checks represented by these paths.

## 16. Fail-closed conditions

Cross-Lab denies or terminates the relevant operation/session when security state is incomplete or invalid, including:

- unknown or invalid identity;
- invalid owner/device authorization;
- revoked trust;
- invalid credential epoch;
- replay/freshness failure;
- channel-binding failure;
- unsupported/incompatible capability;
- missing or denied policy decision;
- expired/mismatched operation context;
- malformed or oversized input;
- failed privileged caller validation;
- unsupported recovery authority;
- update metadata/integrity/version failure.

Availability failures may degrade capability availability, but they must not be converted into authorization success.

## 17. Logging and audit boundary

Audit records are sensitive metadata. Normal logs must not contain private keys, reusable credentials, authentication secrets, raw pairing bootstrap secrets, recovery secrets, file/clipboard/media payload contents, or equivalent private data.

Security-relevant audit events should identify the source/destination authority, capability/operation, decision/result, time, and transport class only to the extent required for accountability and diagnosis.

## 18. Resource-safety boundary

Network-facing parsing, queues, streams, and task creation must be bounded. Cross-Lab must use explicit cancellation, shutdown, backpressure, and concurrency/resource limits appropriate to each layer. Unauthenticated peers receive the smallest resource budget practical.

Exact numeric limits are implementation decisions unless interoperability requires them to be protocol constants.

## 19. Change control

A change that weakens, merges, bypasses, or materially relocates one of these authority boundaries requires an ADR and explicit architecture approval. Feature-specific threat models may strengthen these rules but must not silently weaken them.
