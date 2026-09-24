# ADR-0019: Platform owner-policy store boundary

**Status:** Proposed
**Date:** 2026-09-24

## Context

PR #56 introduced exact per-device policy mutation and runtime replacement, but intentionally kept policy in memory because no durable policy-store format or currentness boundary had been reviewed.

The Phase 2 clipboard slice is the first product feature that needs owner-edited permission rules to survive restart. Persisting those rules inside identity state would couple two security domains with different lifecycles, while storing an unauthenticated preferences file would permit rollback or silent policy reset.

Cross-Lab therefore needs a narrow owner-policy persistence boundary before product permission controls become writable.

## Decision

If accepted, Cross-Lab adds a separate platform owner-policy store. It is independent from the ADR-0015 identity store and persists only local authorization policy.

### Stored state

The shared snapshot contains:

- schema version;
- exact PolicyState revision;
- bounded exact rules;
- each rule's RuleId;
- source DeviceId;
- CapabilityId;
- OperationName;
- RuleEffect;
- constraints;
- obligations.

It does not contain:

- identity private keys or signing-provider handles;
- credentials or trust records;
- clipboard/file/notification payloads;
- audit history;
- discovery/session routing data.

The encoded snapshot must be deterministic, bounded, versioned, and reject malformed/duplicate rule keys.

### Revision and currentness

PolicyState revision is the authoritative policy revision and is persisted unchanged.

The store must reject:

- a snapshot whose encoded revision does not match its decoded PolicyState revision;
- stale or mixed snapshot/currentness state;
- rollback to an older valid snapshot after a newer revision was committed;
- concurrent writes based on a stale expected revision.

Absence of any policy store on a first-run installation represents the valid empty PolicyState at revision 0.

Once durable policy state has been committed, missing/corrupt snapshot or missing/mismatched protected currentness material is not treated as a new empty policy. The product fails closed for protected capability operations and surfaces policy storage as unavailable.

### Load ordering

Product startup loads and validates policy before constructing TrustedPresenceAgent or any runtime session that can authorize protected operations.

The runtime therefore receives one locally current PolicyState from the start rather than connecting with revision 0 and applying persisted owner rules later.

### Commit ordering

An owner permission edit uses persist-before-apply ordering:

1. derive the next PolicyState in memory;
2. atomically commit it through the platform policy store using the expected current revision;
3. only after durable commit succeeds, replace the active agent/runtime policy;
4. if runtime replacement unexpectedly fails after a successful commit, fail the affected runtime closed and restart from the durable policy state.

A failed durable commit leaves the active policy unchanged.

### Shared/platform split

A shared Rust policy-store crate owns:

- deterministic snapshot encode/decode;
- schema and bounds;
- revision/currentness envelope validation;
- compare-and-swap preparation and validation.

Platform adapters own protected currentness material and crash-safe file replacement.

Linux uses a separate Cross-Lab policy-state file plus a Secret Service currentness anchor under policy-specific labels. It does not reuse identity-store anchor labels.

Android uses a separate AtomicFile plus Android Keystore protected currentness material under policy-specific aliases. It does not reuse identity-store aliases.

Future Windows/macOS/iOS adapters implement the same shared contract using their native protected storage.

### Bounds

The initial product store remains intentionally small:

- at most 1,024 exact policy rules;
- existing CapabilityId and OperationName validators remain authoritative;
- constraints and obligations remain limited to the currently defined bounded policy enums;
- total encoded snapshot size is capped at 1 MiB.

These are local storage bounds, not network protocol limits.

## Alternatives considered

### Persist policy inside ProductIdentityState

Rejected. Owner authorization policy is not identity/trust evidence and changes more frequently. Coupling them would widen identity-store writes and mix independent rollback/currentness domains.

### Plain preferences/JSON without protected currentness

Rejected. Integrity-readable preferences do not prevent rollback to an older valid allow/deny configuration or silent reset.

### Treat any missing store as empty default-deny

Rejected after the first committed policy. Default deny is secure for operation execution, but silently discarding durable owner intent breaks currentness and owner-control guarantees. Missing committed state must be surfaced as storage failure.

### Apply runtime policy before persistence

Rejected. A crash could expose a policy that never became durable or revert owner intent after restart.

## Security impact

The design keeps trust separate from permission, gives owner policy a rollback/currentness boundary, and ensures protected operations never run from stale or partially loaded persisted policy.

Policy metadata is security-sensitive even though it is not secret key material. Platform files use private permissions and protected currentness material; logs must not dump complete policy snapshots unnecessarily.

## Compatibility impact

The local policy snapshot schema and protected-currentness contract become persistent compatibility surfaces once accepted and implemented.

No network wire, identity transcript, trust, session-authentication, or capability payload format changes.

## Operational impact

A damaged or unavailable committed policy store disables protected capability authorization until repaired/recovered rather than silently resetting permissions.

Platform storage adapters may surface native secure-storage availability errors outside UI rendering paths.

## Consequences

Cross-Lab gains a durable owner-controlled permission boundary suitable for clipboard and later capabilities without overloading the identity store.

The first implementation can reuse proven identity-store crash-safety patterns while keeping separate files, labels, aliases, schemas, and domain types.
