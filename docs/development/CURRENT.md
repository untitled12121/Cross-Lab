# Current Development State

This file is the durable resume guide for active Cross-Lab development. Git/code/tests are factual state; this file records the intended handoff.

## Current Phase

**M10 first real Linux + Android platform slice remains open only for physical-device evidence. Product pairing implementation continues in parallel.**

M1-M9 are complete. The M10 host/runtime/UI slice is integrated on `main`; its mandatory physical Linux + Android lifecycle/security/resource evidence is still pending and must not be inferred from CI.

The production pairing foundation has now advanced through the Add Device / QR surface:

- PR #45: platform signing-provider boundary;
- PR #46: product pairing bootstrap envelope;
- PR #47: platform identity-store architecture;
- PR #48: Android/Linux production identity-store adapters;
- PR #49: Linux Add Device QR invitation UI plus Android CameraX/ML Kit QR scanner.

## Canonical Baseline

- Current `main` after PR #49: `445bbf32178dff94f339fc1ae80447967f5da215`.
- PR #48 — production identity-store implementation: merged as `6aae001a2911f4752c29dfc16696fabb20695aa5`; exact-head CI `35520246921` fully green on `6f958c98618ae7af44dce6def92efdf8fec487c5`.
- PR #49 — product Add Device / QR flow: merged as `445bbf32178dff94f339fc1ae80447967f5da215`; exact-head CI `35533414673` fully green on `7d3176ffd4387d3e29982ae5fe641249d6d33445`.
- The final PR #49 CI blocker was fixed rather than suppressed: the redacted `MobilePairingBootstrap` debug implementation was moved before the test module to satisfy `clippy::items-after-test-module`.
- Physical-evidence continuation branch: `m10-task10-real-device-evidence`.
- M10 evidence protocol: `docs/research/M10-platform-evidence.md`.
- Active product pairing plan: `docs/superpowers/plans/2026-09-20-product-add-device-pairing.md`.
- M10 platform plan: `docs/superpowers/plans/2026-09-17-m10-first-platform-slice.md`.
- Master Architecture revision 2.6 plus ADR-0003, ADR-0008, ADR-0009, ADR-0010, ADR-0013, ADR-0014, and ADR-0015 are the governing baseline for the next pairing work.

## Product Pairing Status

### Implemented and merged

- Android production identity store with Android Keystore protected signing material/currentness handling.
- Linux production identity store with Secret Service protected signing material/currentness handling.
- Shared versioned identity-store envelope/currentness contract.
- Product owner/local-device identity snapshot with provider binding validation.
- Five-minute single-use `crosslab:pair:v1:` invitation generation.
- Native GPUI QR presentation with cancel/regenerate behavior.
- Android CameraX lifecycle-bound scanner.
- Bundled/offline ML Kit QR recognition.
- Explicit Android camera permission handling.
- Shared Rust bootstrap validation.
- Scanned pairing secret remains inside an opaque Rust UniFFI object rather than Compose presentation state.
- QR/bootstrap debug output is redacted.

### Not yet implemented

- Shared product pairing coordinator.
- Atomic durable peer credential + verified trust persistence for both sides of a successful pairing.
- Product-level failure/replay/persistence/rollback coordinator tests.
- Bounded LAN discovery profile and its ADR.
- Provisional Quinn product-pairing channel.
- Real Linux ↔ Android product pairing over LAN.

## Security Finding for the Next Milestone

Normal authenticated sessions require each side to possess current validated trust for its peer. The existing core pairing state machine establishes inviter-side trust after the joiner's final proof, but the product persistence snapshot does not yet durably reconstruct the reciprocal peer credential/trust state required after restart.

The next implementation must therefore preserve these rules:

- scanning a QR proves possession of a temporary bootstrap secret; it is **not** durable trust;
- ADR-0003 transcript confirmation, owner-authorized credential validation, and device proof-of-possession remain mandatory;
- successful pairing becomes product-visible only after the required credential/trust state is committed atomically through the ADR-0015 identity-store boundary;
- persistence/currentness failure fails closed and does not leave durable `Trusted` state;
- discovery addresses and Quinn/TLS identity remain routing/channel metadata, never Cross-Lab identity authority;
- pairing secrets, private keys, protected signer material, and sensitive payloads are never logged.

## Exact Next Task

Continue `docs/superpowers/plans/2026-09-20-product-add-device-pairing.md` in this order:

1. implement the shared product pairing coordinator using the existing ADR-0003 pairing flow rather than a second protocol;
2. persist peer credential + verified trust as one logical security commit and make restart reconstruction fail closed;
3. add success, cancellation, replay, malformed bootstrap, proof failure, persistence failure, and rollback/currentness tests;
4. record the bounded LAN discovery + provisional Quinn pairing profile in an ADR before promotion;
5. implement Linux ↔ Android real product pairing;
6. collect physical camera/LAN evidence on actual devices.

M10 physical hardware evidence remains a separate track and does not block product implementation. Do not claim physical behavior verified until it is tested on the real Linux + Android pair.

## M10 Physical Evidence Still Pending

The following remain physically unverified:

- Android foreground → background → foreground;
- local network loss → restoration;
- Linux/Android disconnect → fresh authenticated reconnect;
- revocation → reconnect denial on the physical pair;
- clean explicit shutdown;
- no duplicate runtime/session after lifecycle churn;
- bounded idle/reconnect behavior on real devices;
- Ayu Light rendering/selection on the physical pair;
- the new Android QR scanner on a physical camera;
- real LAN product pairing once implemented.

Darkmatter/System-dark remains blocked until an authoritative Darkmatter palette is supplied.

## Development Constraints

- UI code does not own networking, cryptography, persistence internals, or privileged operations.
- Shared pairing state stays in Rust; UI/FFI receives presentation-safe state/events only.
- Product pairing reuses the existing pairing transcript/HMAC and credential proof-of-possession semantics.
- Cross-Lab identity/trust remain independent from IP, hostname, BLE identifiers, TLS certificates, Quinn connection identity, or discovery metadata.
- Production identity load/commit is fail-closed when provider/currentness state is missing, stale, rolled back, or inconsistent.
- Development provisioning remains explicit, non-default, private, and separate from production pairing.
- Do not promote Iroh in M10; preserve ADR-0009.
- No architectural trust/protocol/transport/persistent-format change is made silently; use an ADR when the Master Architecture requires one.

## Resume Procedure

1. verify `main`, active feature/evidence branches, PR state, exact-head CI, and recent commits before editing;
2. read Master Architecture revision 2.6, this file, the active product pairing plan, and relevant ADRs/specifications;
3. inspect the existing core pairing/session/policy code and platform identity-store adapters before introducing new APIs;
4. reuse the existing pairing state machine and currentness boundaries rather than duplicating security semantics;
5. keep changes in small verifiable milestones, run the relevant full Rust/Android gates, commit/push completed work, and update this file with the exact next task;
6. keep physical-device evidence separate from CI claims.
