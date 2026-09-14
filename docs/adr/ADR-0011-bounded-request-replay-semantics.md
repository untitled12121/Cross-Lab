# ADR-0011: Bounded request replay and retry semantics

**Status:** Accepted  
**Date:** 2026-09-14  
**Accepted:** 2026-09-14

## Context

Protocol v1 currently combines two different concerns around `RequestId`:

1. correlation and retry/result handling for control requests;
2. wording that implies a `NonRetryable` `RequestId` must remain remembered for the entire logical session.

Cross-Lab also requires externally influenced state to remain bounded and requires session replay protection through authenticated `SessionId` plus strictly monotonic directional `message_seq`.

These requirements conflict if duplicate-request protection is interpreted as an unbounded forever-within-session set of every historical `RequestId`. Keeping every completed request ID can permanently exhaust a long-lived session. Evicting completed IDs preserves bounded memory but means an ancient identifier may eventually leave the local duplicate cache.

The current implementation uses bounded completed-request state. That is safer operationally than unbounded retention, but the existing Protocol V1 wording overstates the lifetime guarantee and creates a code/spec mismatch.

## Decision

### 1. Separate wire replay/order integrity from semantic duplicate requests

The authoritative ordinary control-plane replay/order mechanism for Phase 1 remains:

- authenticated logical `SessionId`;
- one ordered reliable control channel per direction;
- directional `message_seq` beginning at zero and increasing exactly by one;
- duplicate/lower sequence rejection;
- gap rejection on the ordered reliable channel;
- fresh `SessionId` and fresh sequence state on reconnect.

An exact previously accepted envelope replayed with its old sequence is rejected by directional sequence state.

If an authenticated malicious peer intentionally constructs a new envelope with the next valid sequence but repeats an earlier operation body or `RequestId`, that is a new authenticated request attempt rather than wire replay. It must still pass current trust, capability, policy, operation, and bounded duplicate-state checks.

`RequestId` is therefore not the primary full-session replay primitive, and retaining it forever cannot be treated as a security boundary against a malicious peer that is already allowed to submit new requests. A peer could otherwise choose a fresh `RequestId` for the same operation anyway.

Operations that require single-use, bounded-use, or idempotent semantics must enforce those semantics through local capability/`AuthorizedOperation` state rather than depend on an unbounded history of request identifiers.

### 2. `RequestId` remains correlation and bounded duplicate/retry state

`RequestId` remains a cryptographically random 128-bit identifier scoped to a logical session.

The receiver maintains bounded session-local request state for:

- requests currently in progress;
- recently completed/cancelled request identifiers;
- capability-authorized idempotent retry/result information when that behavior is implemented.

Duplicate identifiers that are still represented in current bounded state are rejected or handled according to the capability's approved idempotency semantics.

Cross-Lab does not promise permanent retention of every completed `RequestId` for the full lifetime of an arbitrarily long session. Once a completed identifier has legitimately aged out of the bounded recent-history window, a later request using that identifier is evaluated as a new request attempt and must pass current authorization/operation semantics.

### 3. `RetryClass` does not grant authority

Peer-declared `RetryClass::Idempotent` never authorizes duplicate execution by itself.

A duplicate execution/result-reuse path is permitted only when local capability metadata explicitly declares the operation idempotent under the same request semantics.

Generic protocol code must not automatically retry or re-execute `NonRetryable` operations.

For Phase 1, when capability-local idempotent result caching is not implemented, duplicate IDs in the retained recent-history window are rejected rather than re-executed.

`NonRetryable` means the generic sender/protocol layer must not automatically retry that request. It does not create a permanent receiver-side exactly-once guarantee for an arbitrarily long session.

### 4. Bounded-state exhaustion remains fail closed

All request tracking has explicit configured bounds.

If active/in-flight request state reaches its safety capacity, the receiver returns a typed resource-limit failure and follows the existing fatal/nonfatal session policy as specified by Core.

Completed-history eviction must not evict active request state and must not weaken directional sequence validation.

The implementation may use count-based bounded retention in Phase 1. A future count+time policy may be added without changing the security contract if it remains locally controlled and bounded.

### 5. No generic exactly-once guarantee

Phase 1 continues to provide no generic durable exactly-once execution guarantee.

After reconnect, request/result state from the old session is not assumed reusable. A capability that needs cross-session idempotency must define a capability-specific stable operation/idempotency design through separate review.

Within one session, capabilities that need stronger duplicate-effect prevention than the recent `RequestId` window must enforce it through their own locally authoritative operation/use state.

### 6. Cancellation semantics

Cancellation does not erase recent duplicate protection immediately. A cancelled request ID remains in the bounded recent-history window so an immediate duplicate cannot recreate work that was just cancelled.

Unknown/already-evicted historical cancellation remains harmless and cannot create authority.

## Alternatives considered

### Retain every `RequestId` until session close

Rejected. This gives simple full-session duplicate-ID memory but creates unbounded growth or a permanent maximum-request-count lifetime for long-lived sessions, conflicting with the architecture's bounded-state requirement. It still would not prevent a malicious authorized peer from submitting the same operation with a fresh identifier.

### Evict IDs while continuing to claim full-session duplicate-ID rejection

Rejected. That preserves bounded implementation state but leaves the specification false and encourages callers to rely on a guarantee the runtime cannot provide.

### Add a new monotonic request ordinal or additional wire replay structure

Rejected for Phase 1. Directional `message_seq` already provides full-session ordered control replay protection. Adding another wire-level sequence for request identity would duplicate state and increase compatibility complexity without a demonstrated requirement.

### Trust peer-declared `Idempotent`

Rejected. Idempotency is an operation semantic controlled by the local capability implementation, not authority granted by an untrusted peer field.

## Security impact

The decision preserves full-session exact-envelope replay/order protection through authenticated session/sequence state while making request-level duplicate memory explicitly bounded.

It does not misrepresent `RequestId` as protection against an authenticated malicious peer intentionally issuing another allowed operation. Authorization, operation-use policy, cancellation, trust/policy revision binding, and capability-specific idempotency remain the authoritative controls for semantic duplicate effects.

It removes pressure to choose between unbounded attacker-influenced memory and silently weakened duplicate semantics.

Capability-specific duplicate execution remains local-authority-controlled rather than peer-controlled.

## Compatibility impact

No protobuf field, enum value, frame shape, or control-envelope encoding changes are required.

This is a semantic clarification of Protocol V1. The existing `RequestId`, `RetryClass`, `SessionId`, and `message_seq` wire representations remain unchanged.

Protocol V1's current full-session duplicate-`RequestId` wording must be revised when this ADR is accepted because that behavioral guarantee changes to an explicitly bounded recent-history contract.

Any future capability-specific cross-session idempotency contract is outside this ADR and may require its own compatibility review.

## Operational impact

Core keeps bounded in-memory request tracking. Long-lived sessions are not forced to terminate solely because a permanent history set has accumulated every completed request identifier.

No persistence, database, new transport primitive, or new dependency is introduced.

## Consequences

- directional `message_seq` is the authoritative Phase 1 exact-envelope replay/order boundary;
- `RequestId` is a bounded correlation/duplicate/retry key rather than a second forever-retained session replay log;
- recent duplicates remain rejected fail-closed;
- active request state is never silently evicted to make room for history;
- a later request whose old ID has aged out is treated as a new request and still requires current authorization/operation validity;
- peer-declared retry class cannot authorize duplicate execution;
- stronger semantic single-use/idempotency requirements live in local capability/operation state;
- Protocol V1 must be updated after this ADR is accepted so its duplicate wording matches the bounded contract;
- capability-specific idempotent result caching can be added later without weakening the generic security boundary.
