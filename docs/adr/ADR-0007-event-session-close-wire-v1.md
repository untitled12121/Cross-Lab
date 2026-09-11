# ADR-0007: Event and session-close wire contract v1

**Status:** Accepted  
**Date:** 2026-09-11  
**Accepted:** 2026-09-11

## Context

`docs/protocol/PROTOCOL-V1.md` requires authenticated control-plane `event` and `session_close` envelope bodies but does not assign enough stable v1 wire detail for their event-type representation, system-event namespace, or session-close reason registry. These values are public protocol compatibility decisions and cannot be invented implicitly in implementation.

## Decision

### Events

`EventV1` contains:

```text
event_id:      16 bytes
capability_id: optional canonical CapabilityId
event_type:    canonical namespaced ASCII EventType
body:          opaque bytes
```

`EventType` is limited to 128 ASCII bytes and uses lowercase dot-separated segments. Each segment begins with `a-z`; remaining characters are `a-z`, `0-9`, or `-`, and a segment cannot end in `-`. At least two segments are required.

Capability events carry `capability_id` and may not use the reserved system prefix. System events omit `capability_id` and must use an `event_type` beginning with:

```text
crosslab.system.
```

Envelope field tag `8` is assigned to `EventV1`. Event bodies remain capability/system-family payloads interpreted above the generic protocol layer and are bounded by the normal control-frame limit.

### Session close

`SessionCloseReasonV1` uses the stable numeric registry:

```text
0  Unspecified          invalid on decode
1  Normal
2  LocalRequest
3  ProtocolError
4  AuthenticationLost
5  TrustRevoked
6  Shutdown
```

Unknown values fail closed. `SessionCloseV1` carries the typed reason plus an optional safe diagnostic capped by the existing 512 UTF-8 byte protocol-diagnostic limit. Envelope field tag `11` is assigned to `SessionCloseV1`.

The tracked `.proto` schema is the public wire representation and the Rust domain layer validates all event identifiers, scopes, close reasons, and diagnostics during wire conversion.

## Alternatives considered

### Global numeric event registry

Rejected for v1. Capability and system event families are expected to evolve independently, and a single global numeric registry would create unnecessary central coordination while still requiring capability scoping.

### Arbitrary unvalidated event strings

Rejected. Unbounded or noncanonical strings weaken compatibility checks, permit ambiguous representations, and increase parser/resource risk.

### Separate free-form system namespace field

Rejected. A reserved canonical `crosslab.system.*` event-type namespace keeps the wire shape small while making system events distinguishable without granting them capability identity.

### Free-form session-close reason text

Rejected. Close reasons affect lifecycle/security behavior and require a stable fail-closed registry; optional diagnostic text remains informational only.

## Security impact

System events cannot masquerade as capability events or vice versa because scope and the reserved prefix are validated together. Unknown session-close reasons do not map to a successful/default lifecycle action. Event IDs retain exact 128-bit validation, and diagnostic text remains bounded and non-authoritative.

## Compatibility impact

Envelope tags `8` and `11`, the `EventV1` field layout, canonical `EventType` grammar, reserved system prefix, and session-close numeric values are part of protocol v1 and must not be reinterpreted. Future additions use additive protobuf evolution or a new protocol/profile when semantics are breaking.

## Operational impact

No new runtime service, transport, persistence, or platform dependency is introduced. Existing control-envelope parser fuzzing covers the new envelope variants, and fixed protobuf/framing vectors preserve the assigned v1 bytes.

## Consequences

- M4 can implement the two previously underspecified control-envelope bodies without crossing into M5 session orchestration.
- Capability event identity and system-event identity remain explicit and fail closed.
- Session-close handling has stable cross-language reason values.
- `.proto`, strict domain conversion, compatibility tests, golden vectors, and this ADR preserve the v1 contract.
