# ADR-0021: Owner-authorized Android notification mirror v3

**Status:** Proposed  
**Date:** 2026-10-02  
**Scope:** Phase 2 Linux + Android only

## Context

The Master Architecture reserves `notifications.read` version 3 without defining a product wire profile. The first Phase 2 use case is explicitly owner-enabled Android-to-Linux mirroring. Android requires separate OS Notification Access through a `NotificationListenerService`. Linux receiver must not treat notification data as authentication, permission or executable instructions. The normal foreground-only Android presence lifecycle cannot promise background delivery.

## Decision (proposed)

- Android is the **source**; Linux is the **consumer**. No Linux-to-Android mirroring, remote actions, or automatic reply in v3.0. No mandatory cloud.
- Exact protected capability: `notifications.read`, version `3.0`; request operation `subscribe` (NonRetryable) evaluated by Android's existing per-peer, exact-rule policy. No rule means deny; `Ask` is not implicit approval. Linux may request only an authenticated, negotiated, currently available subscription.
- Android advertises runtime availability only while its local listener has explicit OS Notification Access, its owner has explicitly enabled Cross-Lab mirroring, and the product transport/runtime is running. Revoking any grant immediately disables delivery and tears down subscriptions.
- An accepted `subscribe` request produces a bounded session-scoped subscription, not a durable capability or independent authority. The request body is empty; an optional cancellation uses the existing request/session cancellation semantics. At most one active subscription per authenticated peer/session. Session replacement, policy revision, revocation, disconnect, listener loss and app background shutdown revoke it.
- Only after acceptance, Android emits authenticated capability events `notifications.posted` and `notifications.removed`. Payload version is exactly 1, encoded using the existing bounded capability event body mechanism, never new unauthenticated transport.
- Posted payload: per-session random opaque notification ID (16 bytes), bounded sanitized app **display label** (0–96 UTF-8 bytes), optional title (0–256 UTF-8 bytes), optional text preview (0–512 UTF-8 bytes), and flags for redacted/missing content. Removed payload: same opaque ID; no content. App package name, original platform notification ID/key, icons, actions, intents, reply tokens and arbitrary extras are never sent. Reject malformed UTF-8, duplicate fields, unknown versions, excessive length and oversized frames.
- Content mirroring (title/preview) is **off by default**; per-device owner consent may enable it. Exclude Android-protected/sensitive notifications where the platform marks content unavailable. Do not claim automatic detection of secrets inside arbitrary user text. Linux presents received data as untrusted plain text only, never markup/shell/URLs to execute.
- Queue is bounded (e.g. 64 pending notifications); terminal/drop behavior is deterministic and does not block security controls. Coalesce a burst by the per-session ID; no uncontrolled retries/polling. Rate-limited state updates and UI-visible skipped-count indication where needed. Receipt is best-effort, not an archival or exactly-once service. A fresh authenticated session begins with a new subscription and identifiers; no replay of historical notifications.
- Linux initially uses in-app owner-visible notifications. Native desktop notification-server forwarding, if added, requires an explicit owner toggle and D-Bus platform adapter. No private payload logging or audit history; minimal metadata may be recorded only under the separately approved audit profile.

## Alternatives considered

- **Bidirectional first:** deferred because Linux's desktop notification observation contract is environment-dependent.
- **Passive push with no subscribe:** rejected; authenticated transport alone does not imply owner authorization.
- **Always-on Android listener/daemon:** deferred; Android's user permission and process constraints must remain authoritative.
- **Mirror all system fields:** rejected for privacy, spoofing and resource reasons.

## Security impact

OS consent and local per-device policy are independent gates. Only session-authenticated, exact-capability, policy-approved subscriptions receive data. Notification text is sensitive and never used as code or an identity/trust signal. Disconnect/revocation/permission loss cancel delivery promptly. Test forged peer, policy transition, revoked listener, payload bounds, reconnect replay, overflow, redaction and user opt-out.

## Compatibility impact

The capability version, operation, events, payload bounds and precise encoding need one reviewed implementation specification and golden tests before acceptance. Existing generic protocol/replay rules remain authoritative. Changes to these v3.0 meanings require version negotiation, not silent reinterpretation.

## Operational impact

Use a narrow Android `NotificationListenerService` adapter, shared Rust bounded protocol/runtime handling, and Linux presentation adapter. No new god daemon and no mandatory background service.

## Consequences

The first mirror is permission-first and explicit rather than pretending background notifications work under every Android lifecycle. **This proposal must be accepted before changes are made to the public protocol.**
