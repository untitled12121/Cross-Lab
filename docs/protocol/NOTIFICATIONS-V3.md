# Notification mirror payload v3.0 — bounded capability event body

**Status:** Implemented protocol/agent/platform integration under accepted ADR-0021, pending exact-head format/build/security CI and Linux–Android hardware evidence. Implemented code is not yet a verified delivered product capability.

Capability: `notifications.read`, version `3.0`. Protected subscribe operation: `subscribe`, non-retryable, **empty request body**. Empty-body validation never constitutes authorization: owner opt-in, Android Notification Access, active authenticated session, explicit exact per-peer rule and capability negotiation are separate mandatory gates.

Events (authenticated session only): `notifications.posted` and `notifications.removed`. Both carry **version 1** binary bodies, not new transport frames. Big-endian lengths; reject any trailing bytes, invalid UTF-8, control characters, unknown flags/version/type or over-limit data. Maximum payload **896 bytes**.

| Field | Posted | Removed |
|---|---|---|
| profile version | u8 = 1 | u8 = 1 |
| kind | u8 = 1 | u8 = 2 |
| opaque id | 16 random per-session bytes | 16 bytes from posted |
| flags | u8 (bit0 redacted; bit1 title; bit2 preview) | absent |
| app label | u8 byte length + UTF-8 bytes (0–96) | absent |
| optional title | when bit1, u16 byte length + UTF-8 (0–256) | absent |
| optional text | when bit2, u16 byte length + UTF-8 (0–512) | absent |

Redacted means **neither title nor preview may be present**. Content is disabled by default even for allowed mirroring. Package names, original Android notification IDs/keys, actions, icons, intents and other extras are never included. Render as plain untrusted text. A removed record never contains text.

Golden removed example: `01 02` followed by 16 bytes `7a`; resulting length exactly 18 bytes. This profile is not sufficient to authorize `subscribe` or emit events: the sender/receiver runtime must enforce the separate accepted ADR-0021 state machine and bounded queues, lifecycle shutdown and revocation cancellation.

Code/test home: `crates/protocol/src/notification.rs`.

The Linux GPUI owner action initiates an exact negotiated v3 non-retryable, empty-body `subscribe` request over the existing authenticated control channel. Its bounded in-memory `NotificationInbox` registers posted/removed event subscriptions but rejects incoming events until a **matching request/session** receives an accepted empty success result. Session replacement, closure, policy changes and malformed events clear private inbox state. The reader shows plain untrusted text, no notification persistence, and a bounded skipped count.

On Android, availability is independently gated by Notification Access, connected OS listener, owner opt-in, foreground-only runtime, a *trusted authenticated* session, exact local capability version 3.0 and an explicit durable per-peer Allow rule for `notifications.read/subscribe`. Missing or Ask rules fail closed; the Android Owner screen provides explicit allow/deny operations only for the currently authenticated full peer ID. Permission mutation checks exact session/peer again within the protected adapter to prevent session switch races.

The OS `NotificationListenerService` checks for a live source subscription **before reading** app label or contents and ignores system, call, secret, ongoing and service notifications. App label, optional title and preview are sanitized as bounded plain UTF-8 and then sent through UniFFI to the shared Rust agent. Content is off by default even if mirroring is enabled; platform-private notifications are always flagged redacted. Original Android keys remain in a bounded, process-only per-session map (64 entries), never in the wire events, logs or audit history. The mirror uses a fresh 16-byte random opaque identifier, a coalesced bounded queue and authenticated actor events.

Owner disable, OS listener loss, background runtime stop, policy change, session revocation/disconnect and failed sender operation must close in-memory subscriptions and purge transient payloads. An explicit cancellation must match the exact originally accepted notification subscription request ID; cancelling an unrelated capability operation cannot revoke that subscription. The Linux reader applies a bounded, monotonic ten-second response timeout, sends a cancellation for that timed-out request, and rejects late responses. The matching request/session response must arrive **strictly before** the deadline even if its runtime event is selected before the timeout callback; the deadline comparison never grants authority or inserts private data. An expired pending request remains scheduled for cancellation/status delivery until the timer processes it; timeout does not imply approval or grant authority. No Android notification listener background delivery guarantee is implied. End-to-end pairing, authorization deny, relaunch, reconnect, OS opt-out, removal and resource/performance hardware evidence remains an **open Phase 2 release gate**. Integration source: `crates/agent/src/notification/`, `crates/agent/src/presence/runner.rs`, Android `features/notifications`, and Linux `control_center`.
