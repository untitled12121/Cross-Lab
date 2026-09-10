# ADR-0004: Protocol encoding and canonical signing profile v1

**Status:** Accepted  
**Date:** 2026-09-11  
**Accepted:** 2026-09-11

## Context

Cross-Lab requires a compact, evolvable, cross-language wire format suitable for Rust, Kotlin, and Swift while ensuring that security-sensitive signatures are stable across serializers and language implementations. The Master Architecture lists Protocol Buffers as a strong candidate but explicitly prohibits signing arbitrary serializer output.

## Decision

Use **Protocol Buffers** as the v1 control-plane wire schema/serialization format.

Security-sensitive signed/MACed objects do **not** sign protobuf bytes. They are converted to a Cross-Lab canonical transcript v1 defined in `docs/protocol/PROTOCOL-V1.md`. The canonical transcript uses fixed byte framing, fixed-width big-endian integer encoding, spec-defined field tags/order, explicit domain/profile identifiers, and BLAKE3-256 transcript digests.

Protocol major/minor compatibility is negotiated before ordinary session traffic. Unknown mandatory semantics, unsupported major versions, malformed oneof bodies, or invalid security transcript/profile identifiers fail closed.

## Alternatives considered

### CBOR as both wire and canonical format

Canonical CBOR can support deterministic encoding, but using one encoding for both wire evolution and signing increases the chance that ordinary serializer/schema behavior accidentally becomes security-critical. Cross-Lab keeps the signed transcript deliberately narrower.

### JSON

Human-readable but larger, slower to validate, less strongly typed, and more normalization-sensitive for a security-sensitive cross-device protocol.

### Custom binary wire protocol

Could minimize overhead but would require Cross-Lab to design and maintain schema tooling, compatibility rules, and language bindings unnecessarily.

### Sign deterministic protobuf serialization

Rejected. Deterministic serialization settings are an implementation property and do not provide the explicit long-term canonical security contract Cross-Lab needs across languages/versions.

## Security impact

Separating wire serialization from canonical signing prevents serializer behavior, unknown-field retention, field ordering, or language implementation differences from changing what a security signature means. Explicit bounds and version negotiation reduce parser/resource and downgrade risk.

## Compatibility impact

`.proto` schemas become the long-term public wire contract. Additive minor-version evolution is allowed only within explicit compatibility rules; security-semantic changes require a new transcript/profile and potentially protocol major version.

## Operational impact

Phase 1 requires maintained protobuf/prost tooling. Exact crates/tooling and versions are verified at implementation time. Build tooling must remain reproducible and must not require untracked generated files as the only source of protocol truth.

## Consequences

- Cross-language protocol evolution has a standard schema format.
- Security signatures remain independent of protobuf byte serialization.
- The repository maintains golden vectors for canonical transcripts and protocol compatibility.
- This ADR is part of the accepted Phase 0 architecture baseline.
