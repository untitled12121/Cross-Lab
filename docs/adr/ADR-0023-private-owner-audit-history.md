# ADR-0023: Privacy-bounded owner audit/history store

**Status:** Accepted
**Accepted:** 2026-10-02 (owner approval)  
**Date:** 2026-10-02  
**Scope:** Phase 2 Linux and Android

## Context

`AUDIT-PRIVACY.md` defines audit classes and prohibited sensitive contents, but only in-memory simulator evidence exists. Owner-visible history requires a durability, integrity/currentness, deletion, and failure contract separate from identity/policy/transfer state.

## Decision

- A dedicated shared typed audit event model and narrow platform store service; never embed audit bytes in the identity, owner-policy or clipboard/transfer state. Version each event and snapshot.
- First product event types: pairing started/failed/completed/cancelled, device trust added/revoked, session authenticated/disconnected, owner permission changed, transfer terminal result, notification subscription enabled/disabled. Use public action/capability IDs, coarse timestamp, revision, redacted peer-display reference, and an explicit outcome code only when useful.
- Never record QR content/secrets, signing material, cryptographic proofs, raw identifiers that aren't needed, notification title/body/app labels, clipboard text, file contents/paths/names, packet/IP/MAC address, unredacted device topology, or raw error stack traces.
- Enforce a strict maximum retained count (proposed 1,024) and capped encoded snapshot size (proposed 256 KiB); store oldest-to-newest bounded ordering with loss/count indication. Proposed default 30-day local retention, owner-visible delete and export of redacted records. No cloud sync. Export is explicit, not a background uploader.
- Use private platform-protected storage and separate currentness protection, with crash-safe replacement; no silent fallback to anonymous writable files or reset when a previously committed store is unavailable. Owner is the only audit reader/exporter. Corruption surfaces an actionable safe status.
- Security-related audit failure must never create authorization. Operations that explicitly require durable audit must fail closed *before* effects when audit unavailable; normal best-effort status audit must not silently grant permissions and must surface dropped-event count. Avoid recursive auditing of audit errors.
- Event dispatch uses bounded asynchronous queues with backpressure/rate limits; no polling, unbounded growth or full-payload logging. Originating services emit minimal typed audit intents, not arbitrary strings.
- Device and notification history is owner-only, off by default where content could be sensitive. Owner can clear/export local history, but signed trust revocation records cannot be deleted via an audit-history control.

## Alternatives considered

- **Unbounded debug logs:** rejected for privacy and DoS.
- **Append-only plaintext text file:** rejected without owner access, currentness, rotation or retention controls.
- **Reuse identity-state envelope:** rejected due different lifetime and security domain.
- **Mirrored or remote audit service:** deferred, contradicts local-first minimal MVP.

## Security impact

Bounded writes, owner-only access and safe redaction are mandatory. Test metadata leaks through Debug/display/errors, rollback, concurrent writes, corruption, deletion, flood/coalescing and fail-closed policy operations.

## Compatibility impact

Proposed event/snapshot schemas and platform key labels are local persistent formats; their exact bytes and migration/currentness semantics require a reviewed follow-on test-vector specification. No new network wire event is created by audit history.

## Operational impact

Keep separate Linux/Android platform adapters; use audited, tested local file replacement/currentness patterns rather than custom database/crypto unless evidence requires it.

## Consequences

Provides a small owner-visible record without becoming a second store of personal data. The owner-approved architecture requires an exact bounded persistent codec, platform-currentness schema, and regression tests before shipping the store.
