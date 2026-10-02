# Owner Audit History — Initial Typed Snapshot v1

**Scope:** accepted ADR-0023, metadata-only shared model. Platform-protected storage, owner authentication, CAS currentness, and product history UI remain pending.

A `crosslab-core::AuditRecord` contains only:

- `action`: closed enum of pairing/trust/session/permission/transfer/notification-subscription lifecycle transitions.
- `outcome`: closed enum (succeeded/failed/denied/cancelled).
- `occurred_hour`: coarse local Unix hour, not raw request timestamps.
- `revision`: relevant monotonic local revision when known (0 otherwise).

No raw device IDs, IPs, notification app/title/content, file names/paths, clipboard data, QR/bootstrap or cryptographic material. This initial profile omits peer reference entirely to minimize tracking. Owner-facing CSV export contains only these four typed fields.

Local snapshot: 8-byte `CLAUD01\\0` magic, `u16_be` version 1, `u64_be` dropped count, `u32_be` event count; then exactly N fixed 18-byte records (`u8` action + `u8` outcome + `u64_be` hour + `u64_be` revision). All schemas and outcome codes fail closed if unknown. Count max 1,024, encoded size max 256 KiB. Retain at most 30 days (720 hours); reject newly submitted out-of-window events and prune expired entries on load. Overflow evicts oldest and increments the visible dropped count. No unbounded strings or payloads.

This codec is **not** a persistent protected store by itself. Platform adapters must use a separate owner-only currentness-protected store and atomic writes, not the identity/policy envelope, with corruption/read/write failure surfaced. Clearance only clears audit records, never signed trust tombstones. Authorization must never be upgraded because an audit write failed.
