# ADR-0001: Repository license

**Status:** Proposed  
**Date:** 2026-09-11

## Context

Cross-Lab is intended to be fully open source and Rust-first, with optional reuse of maintained permissive Rust libraries and study of research repositories under a mixture of permissive, copyleft, and other licenses. The Master Architecture requires the project license to be selected before third-party source adaptation and prohibits treating public repository visibility as a license grant.

## Decision

Adopt the common Rust ecosystem dual-license model:

```text
MIT OR Apache-2.0
```

Repository contributions would be accepted under either license at the recipient's option. Individual third-party components retain their own licenses and must still pass compatibility review before source adaptation.

This proposal does not permit copying GPL/AGPL/noncommercial research code into Cross-Lab. Those repositories remain study-only unless a later, explicit compatibility/legal decision permits otherwise.

## Alternatives considered

### Apache-2.0 only

Provides an explicit patent grant and clear terms, but is less flexible for downstream Rust users accustomed to the dual-license pattern.

### MIT only

Very simple and permissive, but lacks Apache-2.0's explicit patent-license language.

### GPL/AGPL family

Would require downstream derivative distributions to follow stronger copyleft obligations. This can protect openness but would materially restrict integration into some operating-system, mobile, and commercial environments and would not automatically make all research code compatible.

## Security impact

No direct runtime security impact. Clear licensing improves dependency/source provenance and reduces pressure to copy code from incompatible research repositories.

## Compatibility impact

MIT OR Apache-2.0 is broadly compatible with the Rust ecosystem and the permissively licensed libraries currently selected/candidate for the foundation. Copyleft or otherwise restrictive source still requires separate review.

## Operational impact

If accepted, the repository must add both license texts and use `license = "MIT OR Apache-2.0"` in publishable Cargo packages unless a package has an explicitly documented exception.

## Consequences

- Contributors and downstream users receive a permissive dual-license choice.
- Cross-Lab must maintain third-party license review rather than assuming the project license makes all source reusable.
- GPL/AGPL/noncommercial research repositories remain reference material by default.
- This ADR requires explicit owner approval before its status changes to Accepted.
