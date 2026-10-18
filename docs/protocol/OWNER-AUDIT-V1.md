# Owner Audit History — Initial Typed Snapshot v1

**Scope:** accepted ADR-0023, metadata-only shared model. Android and Linux owner history store/UI adapters are implemented in the active Phase 2 PR, with exact-head CI, privacy security review and physical test evidence still pending.

A `crosslab-core::AuditRecord` contains only:

- `action`: closed enum of pairing/trust/session/permission/transfer/notification-subscription lifecycle transitions.
- `outcome`: closed enum (succeeded/failed/denied/cancelled).
- `occurred_hour`: coarse local Unix hour, not raw request timestamps.
- `revision`: relevant monotonic local revision when known (0 otherwise).

No raw device IDs, IPs, notification app/title/content, file names/paths, clipboard data, QR/bootstrap or cryptographic material. This initial profile omits peer reference entirely to minimize tracking. Owner-facing CSV export contains only these four typed fields.

Local snapshot: 8-byte `CLAUD01\\0` magic, `u16_be` version 1, `u64_be` dropped count, `u32_be` event count; then exactly N fixed 18-byte records (`u8` action + `u8` outcome + `u64_be` hour + `u64_be` revision). All schemas and outcome codes fail closed if unknown. Count max 1,024, encoded size max 256 KiB. Retain at most 30 days (720 hours); reject newly submitted out-of-window events and prune expired entries on load. Overflow evicts oldest and increments the visible dropped count. No unbounded strings or payloads.

This codec is **not** a persistent protected store by itself. Phase 2 wraps it with two independent app-private, integrity/currentness-protected platform stores (not embedded in identity or owner policy):

- Linux: `$XDG_STATE_HOME/crosslab/audit/history-v1.bin` (or `~/.local/state`), private `0700` directory/`0600` file, an atomically replaced `CLAU\\x01` bundle of generic versioned envelope + anchor, Linux Secret Service item kind `owner-audit-currentness` with one exact current revision. Missing/extra anchors, rollback, corrupt bytes and unavailable protected keyring fail closed. The store serializes writers under a separate file lock.
- Android: `noBackupFilesDir/crosslab/audit/history-v1.bin`, Android `AtomicFile` and independent hardware/OS-backed Keystore HMAC-SHA256 keys `crosslab.audit.anchor.v1.<revision>`. Bundle bytes are `43 4c 41 48 01` magic, `u64_be` revision, `u32_be` payload length, bounded Rust-validated history payload and exactly 32 HMAC bytes. MAC covers magic, revision, length and payload. Exactly one current Keystore alias is required; missing, orphan, stale and mismatched anchors fail closed.

Explicit owner read, history clear and redacted CSV export are wired in both owner views. Linux export copies redacted CSV to clipboard on click; Android export uses an explicit system document destination. Clear persists an empty bounded history at a newer currentness revision and never deletes signed device revocation evidence. Event sources in the current feature branch cover pairing start/failure/completion/cancellation, durable trust addition/revocation, authenticated/disconnected session state, owner policy revision change, transfer terminal success/deny/failure/cancellation (including a direct failed or denied attempt, with Android send and receive directions tracked separately), and notification subscription enable/disable (Linux also tracks denied/timed-out subscription responses). Linux watches the existing trusted presence/capability state and records through a **single bounded 64-intent writer**; Android observes product pairing/runtime/transfer/notification transitions and uses a **single bounded 64-intent protected writer**, even when the owner view is closed. Every persisted record still contains only four typed metadata fields, never source snapshot objects or private identifiers.

Initial history load, event writes, signed-revocation history, explicit clear and owner CSV export are serialized through the same platform recorder, so an earlier pending write/export cannot race a just-cleared history. A rejected enqueue or protected-write failure is surfaced to the owner; Linux tracks dropped queue intents separately from older evicted history events, and Android tracks a separate dropped-events count. Auditing is best effort for normal status events; a write failure does **not** upgrade trust, permission or capability state. The signed trust revocation store remains independent and terminal. The event integration and physical behavior require exact-head CI/security and device verification before release.

## Retention/currentness behavior

On owner read, both platform adapters decode, validate and prune the typed history. If pruning removed expired entries, the reduced payload is atomically recommitted under the next protected revision; clean reads do not generate background writes. This is necessary to enforce the 30-day bound **on disk**, not only in the presentation layer. Failure to verify or advance currentness never triggers a fallback to a blank/unprotected store.
