# Notification mirror payload v3.0 — bounded capability event body

**Status:** Initial implementation profile under accepted ADR-0021. Android OS Notification Access listener, independently stored owner/content consent and permission screen are staged. No cross-device notification delivery is yet integrated or hardware-verified.

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

Android `NotificationListenerService` explicitly discards all callbacks until an authenticated, policy-approved session subscription is wired through the agent runtime. The OS permission and owner/content toggles are separate requirements; neither a manifest registration nor granting Notification Access alone enables delivery or retention. The first receiver is Linux in-app plaintext; the request/event bridge, queue integration, lifecycle testing and receiver UI are still pending.
