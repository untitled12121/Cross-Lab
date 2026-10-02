# Current Development State

This file is the durable resume guide for active Cross-Lab development. Git/code/tests are factual state; this file records the intended handoff.

## Current Phase

**M10 Linux + Android implementation is complete in code; physical Linux/Android camera/LAN evidence remains pending. Phase 2 Linux + Android MVP is the active implementation phase. Phase 3 adaptive networking remains explicitly out of scope until Phase 2 is complete.**

M1-M9 are complete. M10 now includes the normal QR → bounded DNS-SD → provisional Quinn → authenticated pairing → reciprocal durable trust product path on Linux + Android, with full CI verification. Owner-hardware evidence remains a separate gate and must not be inferred from CI.

## Canonical Baseline

- Latest merged application-source baseline is PR #78 merge commit `d5f3bce4073d6cd8d37b9fde1e0b461b77f4bda4`. Its verified exact head `fe7be6a2c60c776d993b7dbc98df86a8c043c407` passed full Rust + Android CI `36974940858`; post-merge `main` CI `36975504949` was triggered. PR #77 Rust CI caching cut the measured job from the 16m17s baseline to 11m01s cold cache and 9m14s warm main (logged job durations).
- PR #49 — Linux Add Device QR invitation UI + Android CameraX/ML Kit scanner: merged as `445bbf32178dff94f339fc1ae80447967f5da215`; exact-head CI `35533414673` green on `7d3176ffd4387d3e29982ae5fe641249d6d33445`.
- PR #50 — shared product pairing coordinator + durable reciprocal trust persistence: merged as `3af825e6f78bef4512168f587f452c1b0267b6a7`; exact-head CI `35567548228` and Fuzz Smoke `35567548208` green on `ec59f99f7c09898aa2533d8b40bd4e980fa2b022`.
- PR #51 — ADR-0016 LAN discovery profile + versioned product-pairing wire + provisional Quinn pairing channel: merged as `cf2add350ffa60056d74ac57b0a187a0297d8777`; exact-head CI `35593397861` and Fuzz Smoke `35593397844` green on `2702cc0c926cf044c65aef0376f573aaedae8c6f`.
- PR #52 — bounded DNS-SD discovery adapters: merged as `2881a8cb042207bf55a1ede9f2dc61f30ccd4eb9`; exact-head CI `35598750651` green on `2ef9ab362fb9fa59d7693662ec8de2f65e5031af`.
- PR #53 — product pairing network/platform lifecycle: merged as `9453b0e7f3e67578c79615acb1034c849a82603d`; implementation head `cc099fd7bf3335f51f54d4e09f2d002ab136e4e0` and documentation head `c5570add764ac1e223b08bdb63737abd3b182a25` both passed the full Rust + Android gate (`35641211049` / `35643137494`).
- Master Architecture revision 2.10 governs accepted ADR-0020 resumable single-file transfer v2 in addition to the established pairing, trusted-session, clipboard, and owner-policy-store architecture.
- Remaining real-device evidence lives in `docs/research/M10-platform-evidence.md` and `docs/research/Phase2-file-transfer-device-evidence.md`. The old `m10-task10-real-device-evidence` branch is not present; the current pre-hardware checkpoint is `phase2-device-evidence-checkpoint`.
- M10 evidence protocol: `docs/research/M10-platform-evidence.md`.
- Product pairing implementation plan: `docs/superpowers/plans/2026-09-20-product-add-device-pairing.md`.
- PR #55 — trusted-session platform presence lifecycle: merged as `6abb3984a36be276aa439e9e1dabb42ffaa4a8b3`; exact-head `91a96966a55c52b6b72c7e5f9b8dc686f8cf0211` passed Rust + Android CI `35859170053`.
- PR #56 — per-device permission/capability-control foundation: merged as `d707b330b5d1008e8020267faa3755ad84920591`; exact head `6565efcdaa7ffc533bbeba6d1b16b5ab8c2d7196` passed full CI `35942887247` and Fuzz Smoke `35942887292`.
- PR #57 — clipboard event-driven control preparation: merged as `7e5985f0f8d8ce4546396da8aa4288fb809b508b`; exact head `f8190dfcf70dc70c94c5f15aae86d907ce4b7a15` passed full Rust + Android CI `35972369409`.
- PR #58 — durable owner-policy persistence: merged as `5b9f271a39c43fb06960a7c3597cd20a4ccdb944`; exact head `04035a036f34dd99391d717e9eb0b12a1506537e` passed full Rust + Android CI `36173154192` and Fuzz Smoke `36173154207`.
- PR #59 — persist-before-apply owner permission editing: merged as `f2780f66fd82c430625f534a4109f56c66400d00`; exact head `5efe69bd8b8cda69ed71b6c4b4c0f4188a14d862` passed full Rust + Android CI `36185572618`.
- PR #60 — shared ADR-0018 clipboard v1 runtime: merged as `aade03ad1e09ed97616a70e5eac2431c6651ac11`; exact head `13e7b4faea4f3b8b045249e6c4059c3f8f15dd1e` passed full Rust + Android CI `36239584543`.
- PR #61 — Linux GPUI + Android ClipboardManager product adapters/UI: merged as `61ae275356201245a7dcac95d11bd598d2c3045e`; exact head `11c8b46732facd2c881ee6963f46a9248da0cba6` passed full Rust + Android CI `36244041548`.
- PR #62 — protocol-neutral authorized data-stream runtime foundation: merged as `e338d10911ccdb908bc495150f048503086c0ce2`; exact implementation head `e663909443d208c712ccb7f6e1e74fb8dab26ab8` passed full Rust + Android CI `36258390585`.
- PR #63 — file-transfer foundation checkpoint + revised ADR-0020 review: merged as `225617f4d1dd281d1fedeb6b3e312feea4a4352c`; exact head `4c6116f29beb923e324caf00341965021882b3bb` passed full Rust + Android CI `36267234275`.
- PR #64 — ADR-0020 acceptance + Master Architecture revision 2.10: merged as `464e7caac23f607f9e9d1346d5fe07ebcb6805ba`.
- PR #65 — exact bounded `files.transfer` v2 payload contract and protocol spec: merged as `06d6e70cb200c31b82782a387a44d34837a7dc3f`; exact head `b4845decaee6d8bcdb5b6509a26c9ea8410b6023` passed full CI `36284858974` and Fuzz Smoke `36284858982`.
- PR #66 — event-driven runtime data-stream readiness: merged as `4311722a5c1737ee6fd886c5b7945f777e48b165`; exact head `42e23a6e2128b8d513a3190a2766809388c28b92` passed full Rust + Android CI `36287858429`.
- PR #67 — `files.transfer` v2 control/authorization runtime: merged as `f051ac0060d9086018480cd9009b74246e8e14cf`; exact head `54f7f90eb70eba6409a4409fa1adf24c202337be` passed full Rust + Android CI `36328477626`.
- PR #68 — `files.transfer` v2 data-plane runtime: merged as `d8ef1901253f8363411a713b8955bd045e72dfe8`; exact head `45f14d3ff523ef4eee5268e8d81c9469be8f58de` passed full Rust + Android CI `36340407398`.
- PR #69 — streaming BLAKE3 integrity + bounded retained transfer state: merged as `0a9d9562bfe35f3fcbd4dc9dd717f247ba0da7a1`; exact head `c5fbab50aaa64e86475e20a1fca2527d195fa5ab` passed full Rust + Android CI `36383403790`.
- PR #70 — Linux file-transfer storage/source adapter: merged as `eb32d79c94418ce46d70f8f91e05ec521f3ad504`; exact head `e653883b90ca7af4faddc6a635b9ccabdfb56779` passed full Rust + Android CI `36393985966`.
- PR #71 — Linux receive/publication adapter: merged as `5c8b57c38b1e18771e1df4f872bfa4b5c077f784`; exact head `e1ec4cab51427e47f0318e73fc41d1cc1696ffea` passed full Rust + Android CI `36397884328`.
- PR #72 — Linux authenticated receive service/worker: merged as `c833a4f9827b4e0cc05999e02c0dac621499800f`; exact head `08ea2cf4361dc9c5634743cebb07a05d4e88966e` passed full Rust + Android CI `36472517377`.
- PRs #73, #75 and #76 merged Linux/Android owner-controlled file transfer. PR #77 merged Rust CI caching. **PR #78 is now merged and fully exact-head CI green**; it fixes Android successful-publication ACK interpretation, preserves typed permission/capability/local-storage failure states, and establishes **normal** non-development Linux+arm64 Android product build instructions and a physical-evidence matrix. All owner-hardware results remain **pending**, not implied by CI. The wider Phase 2 roadmap is still open: notifications, owner-facing audit/history, and device-removal/revocation product controls remain unimplemented independent capabilities; hardware findings may also require follow-up fixes. Darkmatter/System-dark awaits an authoritative owner palette. No Phase 3.
- ADR-0018, ADR-0019, and ADR-0020 are accepted. Clipboard and the Linux file-transfer product path are implemented; Android file-transfer platform integration is active.

## Implemented M10 Product Path

- Linux/Android production signing-provider and identity-store boundaries.
- Versioned identity-store envelope/currentness validation.
- Product owner/local-device identity snapshots and schema-v2 bounded trusted-peer persistence.
- Five-minute single-use `crosslab:pair:v1:` invitation generation.
- Native GPUI QR presentation with cancel/regenerate and terminal lifecycle states.
- Android CameraX scanner with bundled/offline ML Kit QR recognition and explicit camera permission handling.
- Opaque/redacted Rust mobile bootstrap state; secret-bearing QR material is not retained in Compose state.
- Shared ADR-0003 product pairing coordinator.
- Reciprocal peer credential/trust establishment and atomic Linux/Android persistence.
- Replay, forged-proof, cancellation, persistence failure, restart, stale-writer, rollback/currentness, message-order, and final-ack regression coverage.
- ADR-0016 bounded DNS-SD discovery profile.
- Linux Avahi advertisement and Android bounded `NsdManager` resolution with exact PairingId/TXT filtering.
- Versioned product-pairing wire messages for authority/credential bundles, reciprocal trust, persistence/final acknowledgements, and cancellation.
- Provisional Quinn product-pairing channel with ephemeral invitation TLS material, TLS 1.3 only, no 0-RTT, one connection/one bidirectional stream, bounded frames/timeouts, and graceful final acknowledgement delivery.
- Real product lifecycle wiring: Linux invitation/listener/advertisement → Android scan/discovery/client → ADR-0003 verification → local persistence barriers → reciprocal trust → final completion acknowledgement.
- Linux GPUI and Android Compose pairing states for waiting/finding, connecting, verifying, saving trust, finalizing, paired, expiry, cancellation, and failure.
- Linux Avahi service-type normalization regression coverage so the full architecture service name is mapped correctly to Avahi's type/domain API.

## M10 Pending Evidence Only

- Real Linux + Android QR camera scan.
- Real LAN DNS-SD resolution across owner hardware/network.
- Real provisional Quinn pairing exchange between physical devices.
- Resource/lifecycle evidence required by `docs/research/M10-platform-evidence.md`.

These are owner-hardware evidence tasks, not missing implementation tasks.

## Phase 2 Linux + Android MVP — Active

Trusted-session foundation is implemented on PR #54:

- ordinary session proofs use the production SigningProvider boundary;
- authenticated Quinn can resolve the presented peer against bounded durable trusted-peer records;
- ProductIdentityState projects persisted peer evidence into verified TrustRecord values;
- ADR-0017 defines privacy-conscious normal-session DNS-SD with ephemeral random instances and bounded candidate/retry behavior;
- production trusted-session Quinn endpoints use TLS 1.3, no 0-RTT, non-authoritative ephemeral TLS identity, ADR-0008 channel binding, and fresh SessionId per connection;
- provider-backed endpoint/reconnect tests verify fresh session authority and unknown-peer rejection;
- PR #54 merged as `69759d8887a98392519dd72151404399adb9c5dc`; implementation head `feb0fe427045f1f067d2b4b42af4cf4618308b96` passed CI `35685303240`, and documentation head `7178e9b8d4a29a16527b3ecff96018d4a7388d80` passed CI `35686509887`.

PR #55 implements the trusted-session platform lifecycle:

- Linux Avahi and Android NsdManager advertise/browse the ADR-0017 normal-session profile using ephemeral routing-only instances, exact TXT validation, self filtering, and the shared bounded candidate limit;
- the narrow Rust `crosslab-agent` presence coordinator loads durable `ProductIdentityState`, consumes the platform `SigningProvider`, and owns automatic trusted-session connect/accept/reconnect lifecycle above Quinn;
- deterministic dial-role handling and bounded 1/2/4/8/15-second reconnect backoff avoid duplicate connection races and unbounded retry churn;
- network loss, discovery failure, explicit Disconnect/Reconnect, and discovery-instance rotation are wired on Linux and Android;
- authenticated presence is surfaced in Linux GPUI and Android Compose as discovering, connecting, online, reconnecting, paused, or failed without treating discovery/TLS metadata as identity authority;
- physical Linux/Android LAN evidence remains separately pending; PR #55 merged after exact-head Rust + Android CI `35859170053` passed.

PR #56 implements the per-device permission/capability-control foundation:

- `PolicyState` exposes exact typed rule lookup/effect update/removal while preserving constraints/obligations, monotonic revisioning, idempotent no-ops, and default deny when no exact rule exists;
- runtime policy replacement rejects stale revisions and clears request/subscription state tied to the previous policy revision;
- `TrustedPresenceAgent` owns the current in-memory policy, propagates changes into the active runtime, and carries the latest policy into fresh authenticated reconnects;
- Linux GPUI and Android Compose receive presentation-safe negotiated capability IDs plus exact per-peer permission posture, showing unruled capability operations as default deny;
- the slice deliberately does not invent wildcard authority, new product operation identifiers, or a persistent policy-store format; concrete capability features register exact operations and persistence is reviewed when the first product editor requires it.

PR #57 clipboard preparation implements:

- Quinn signals inbound control readiness only after a frame enters its existing bounded queue;
- `RuntimeActor` drains inbound control without polling and publishes bounded non-status `NodeEvent` values;
- the trusted presence coordinator consumes the runtime event stream so an unobserved event queue cannot stall a session;
- actor send wrappers expose the existing capability advertisement/request/response paths without defining new wire semantics;
- ADR-0018 now fixes the first product clipboard profile: explicit text-only `clipboard.read/get` and `clipboard.write/set`, no automatic background clipboard event in v1;
- ADR-0019 now fixes a separate rollback/currentness-aware owner-policy store with load-before-session and persist-before-apply ordering.

Owner approval on 2026-09-25 unblocked implementation. PR #58 merged the shared `crosslab-policy-store` deterministic bounded snapshot/currentness/CAS core and exact `PolicyState` snapshot restoration, plus Linux policy-state/Secret Service currentness and Android AtomicFile/Keystore currentness adapters. Both product presence paths load validated durable policy before constructing the trusted-session agent. Exact-head Rust + Android CI and Fuzz Smoke passed, including fail-closed rejection of ambiguous multi-anchor currentness state.

PR #59 completed the persist-before-apply permission-edit boundary. PR #60 then added the shared ADR-0018 clipboard runtime with bounded UTF-8 payloads, capability negotiation, session-local request correlation, and cancellation across policy/reconnect/disconnect/shutdown. PR #61 completed the Linux GPUI and Android ClipboardManager adapters plus explicit Send/Fetch product controls without background clipboard monitoring or plaintext history.

Clipboard is complete for the Phase 2 MVP software slice. PR #62 completed the protocol-neutral authorized data-stream runtime foundation, PR #65 implemented the accepted v2 offer/accept/result payload contract, PR #66 completed event-driven data-stream draining through `RuntimeActor`, PR #67 completed exact v2 control/authorization runtime, PR #68 completed the actor-owned source/destination data plane, and PR #69 completed streaming BLAKE3 integrity plus bounded retained partial/checkpoint/tombstone state. PR #70 completed the first Linux platform-adapter slice with private crash-safe retained-state persistence, opaque local path locators, and bounded source hashing/resume readers. PR #71 completed private receive partials, durable 1 MiB checkpoint ordering, restart truncation, exact final integrity verification, completion tombstones, crash-safe publication recovery, and atomic non-clobber publication. PR #72 merged authenticated source binding plus the bounded Linux receive worker/service. PR #73 completed explicit native source/save selection, bounded sender hashing/streaming, stable retry/resume identity, send/receive progress, cancellation, retry/resume controls, production `files.transfer` advertisement only when the complete Linux worker/UI path starts successfully, and bounded age-based retained-state cleanup. The current PR #73 checkpoint closes both pre-stream and session-lifecycle cancellation races: owner cancellation removes the pending source offer immediately, propagates request cancellation to the destination product worker, consumes any already-issued single-stream authority if acceptance won the race, drops stale save prompts by exact request identity, and allows immediate retry with the same `TransferId`. Session/policy/disconnect/operation-expiry cleanup now also releases accepted destination platform receivers while retaining only resumable partial state, so stale session-local authority cannot leave a transfer permanently `AlreadyActive`. The Linux receive UI now exposes an explicit owner `Decline` action for a pending offer; decline returns the existing typed `Cancelled` control failure, creates no stream authority or partial file, and permits immediate same-`TransferId` retry. Dismissing the native Save As picker remains non-destructive so the owner can choose a destination again. Accepted `Ready` receives are now cancellable before the data stream opens, during transfer, and after stream finish while publication is still pending: cancellation is keyed by `TransferId`, revokes exact operation/stream authority, best-effort emits terminal `Cancelled`, releases the platform receiver while retaining resumable partial state, and suppresses already-buffered events for the exact cancelled stream so stale data cannot overwrite the owner's cancellation state. If the source observes transport close before the terminal event, it enters the existing bounded terminal-wait state instead of issuing a second finish operation. Platform integrity/storage aborts remain a separate atomic path so they preserve the correct `IntegrityFailed` or `StorageFailed` terminal outcome. Stale destination prompts are never requeued after session loss. Sender retry tokens now bind the generated `TransferId` to the first prepared exact offer identity (display basename, exact size, and BLAKE3 digest); if the local source changes, retry fails locally instead of reusing the same `TransferId` with different metadata. Path-bearing send handles are dropped at terminal state, successful transfers discard the retry token, and source-change failures require a fresh owner file selection/new transfer identity.

Continue in small verified vertical slices:

- per-device permissions/capability-control foundation — implemented on PR #56;
- clipboard — implemented on PR #60 + PR #61;
- resumable file transfer — protocol-neutral stream foundation on PR #62; ADR-0020 accepted on PR #64; v2 payload contract on PR #65; event-driven actor streams on PR #66; control/authorization runtime on PR #67; data-plane runtime on PR #68; integrity/checkpoint retained state on PR #69; Linux storage/source adapter on PR #70; Linux receive/publication adapter on PR #71; Linux product service on PR #72; Linux native product UI + retention cleanup merged on PR #73; Android SAF/ContentResolver adapter merged on PR #75; Android Compose product flow active on `phase2-android-file-transfer-ui`;
- notifications;
- privacy-conscious audit/history;
- revocation/device removal;
- complete professional Linux GPUI and Android Compose control-center UX for those features.

Phase 3 adaptive networking does not begin until Phase 2 is complete.

## Security Invariants

- QR possession is temporary bootstrap authentication, not durable trust.
- ADR-0003 transcript confirmation, owner-authorized credential validation, and device proof-of-possession remain mandatory.
- Discovery addresses, DNS-SD metadata, TLS certificates, and QUIC connection identity are routing/channel metadata only.
- Successful pairing is product-visible only after required credential/trust state is committed through ADR-0015.
- Currentness/persistence failure fails closed.
- Secrets, private keys, signer material, credentials, and sensitive payload contents are never logged.

## Exact Next Task

1. Owner checks out the latest `main` on Linux and builds **normal product mode** (without `--development`): `cargo crosslab setup`, `cargo crosslab doctor`, `cargo crosslab build desktop`, `cargo crosslab build android`; optionally `cargo crosslab install android` and `cargo crosslab run desktop`. Follow `docs/development/BUILD.md`.
2. Owner collects actual Linux + Android QR/camera/LAN pairing and trust evidence in `docs/research/M10-platform-evidence.md`, plus clipboard, default-deny policy, bidirectional file-transfer/reconnect/resume/cancel, destination collision/provider semantics, and privacy evidence in `docs/research/Phase2-file-transfer-device-evidence.md`. Do not mark pending rows passed from CI. Share only redacted failures/evidence for targeted fixes.
3. Before declaring the **entire Phase 2 MVP** complete, approve and implement the independent roadmap gaps (notifications, owner-facing audit/history and device-removal/revocation product controls). Do not invent a new notifications protocol or silently revise architecture without approval.
4. Darkmatter/System-dark remains blocked without the supplied authoritative palette; do not substitute invented values. Stop before Phase 3.

## Efficient Verification Workflow

During implementation, prefer long coherent coding sessions over short update/push loops. Carry related code, tests, refactors, UI integration, and documentation through a meaningful milestone before creating a Git checkpoint. Run only focused checks for touched crates/features plus formatting while iterating; do not repeatedly run the full workspace/Android matrix for every small edit. Push once at the milestone boundary (or earlier only when interruption/context risk makes a durable checkpoint necessary), then let CI run without polling. If CI fails, inspect the completed failure once, fix only the verified issue, run the smallest relevant local check, and push one repair batch. The full Rust + Android exact-head gate remains mandatory before a PR is marked ready/merged.

## Resume Procedure

1. verify `main`, active feature/evidence branches, open PRs, exact-head CI, and recent commits;
2. read Master Architecture revision 2.10, this file, active plans, and relevant ADRs/specifications;
3. inspect existing code and relevant uploaded research before adding adapters/dependencies;
4. reuse existing pairing/session/policy/currentness boundaries rather than duplicating security semantics;
5. work in coherent verifiable milestones, use focused checks while iterating, batch related edits into one push, and update this file;
6. require the full exact-head Rust + Android gate before merge, but do not poll CI continuously after a push;
7. keep physical-device evidence separate from CI claims.

## 2026-10-02 Owner Hardware Bring-up — Open Fix

- Physical Linux product UI reports `product pairing QUIC bind failed`; QR is absent, so real Android scanning/pairing cannot yet complete. Cause isolated to the Linux pairing service attempting Quinn bind on the GPUI caller before entering a Tokio runtime. The feature-branch fix moves QUIC binding and Avahi advertisement startup into the worker runtime and reports startup success before rendering the QR.
- User-local Android SDK bootstrap also failed due missing extracted executable bits and outdated `platforms;android-37` package selection. The feature branch restores CLI owner-execute bits, uses `android sdk install platforms/android-37.0` (as in the pinned CI configuration), and adds offline regression checks.
- Branch: `fix/phase2-linux-pairing-sdk-bootstrap`. Desktop pairing runtime regression and SDK bootstrap tests were added; CI and physical retest results are **not yet verified**.
- Required next evidence: build normal product mode on Linux and Android, regenerate/scan real QR, verify Avahi advertisement and discovery, establish reciprocal durable trust, and validate authenticated presence/clipboard/files against the existing evidence matrices. Do not expose raw QR payloads, peer keys, credentials, or private file metadata in logs/screenshots.
- Phase 2 remains **open**, independently requiring notifications, privacy-conscious owner audit/history, production device revocation/removal and corresponding Android/Linux UX and physical verification. The notification protocol needs a reviewed/approved ADR before implementation. Darkmatter/System-dark palette remains owner-blocked. Phase 3 remains out of scope.

## Batch: paired-device inventory (2026-10-02)

Following successful CI on PR #79 (`36986048412`), the owner requested a larger Phase 2 implementation batch. This branch now adds read-only paired-device inventory sourced exclusively from validated durable `ProductIdentityState`, with presentation-shortened IDs on both Linux GPUI and Android Compose. It refreshes on normal pairing completion and does not confer removal/revocation authority. Android loads identity off the Compose UI thread. Unit check added for inventory surviving transient runtime status loss. **New CI/hardware verification pending**; never confuse inventory display with production revocation. Notification/secure audit/revocation contracts still require scoped ADR review, and physical pairing/reconnect/file tests still need owner hardware.

## 2026-10-02 Durable Phase 2 batch checkpoint

- Working branch: `fix/phase2-linux-pairing-sdk-bootstrap`; source commit: `22719421bbd021b4ce14208c795f7303a903ed63`. Its parent is PR #79's prior exact-head pairing/SDK fix `b2b119d965be4f4230e74defa76eff324a8e2d1d`, whose Rust + Android CI run `36986048412` passed.
- Added Rust/UniFFI product trusted-peer inventory and Linux Owner GPUI + Android Compose presentations. These derive only from validated persisted identity, use truncated presentation IDs, refresh after normal pairing completion, and do **not** claim revocation. Android reads from a background worker. GPUI ignores stale pairing invitation generations; Android shutdown/NSD cleanup races have narrower handling.
- Product pairing currently binds IPv4; Avahi pairing advertisements now advertise IPv4 only and Android resolves an IPv4 host explicitly from the platform address list when available. Android reports an actionable missing-route outcome instead of attempting unusable IPv6. Android pairing profile regression plus Rust/mobile identity privacy-redaction regression included. IPv6-only pairing remains **unsupported by this provisional IPv4 product listener**, pending future review; this is routing only, not identity authority.
- Proposed, **not approved**: ADR-0021 notifications v3 (Android→Linux), ADR-0022 persistent terminal device revocation schema v3, ADR-0023 owner-only bounded audit persistence. Existing Master Architecture and accepted ADRs remain normative; proposed formats MUST NOT be silently implemented before review.
- Current verification: new branch batch not yet run through CI or physical hardware; Rust/Android `cargo fmt`, Clippy, Gradle integration and real-device outcomes are **pending**. Re-run consolidated CI after pushing this complete batch. Do not infer real-device correctness from prior PR #79 CI. The remote execution environment provided no working local Cargo/container runner for this batch.
- Next exact task: inspect CI at the new PR #79 head, repair any compile/lint/test failures in one grouped correction. Then obtain owner Linux/Android non-development QR pairing, reconnect, permission, clipboard and file-transfer evidence. After explicit ADR review/acceptance, implement production revocation, notifications, and private audit in coherent vertical slices. Do not mark Phase 2 complete, and do not start Phase 3.

## 2026-10-02 — Approved Phase 2 trust milestone (verification pending)

- Owner explicitly approved ADR-0021 Android-to-Linux permission-gated notification mirror, ADR-0022 durable signed terminal revocation, and ADR-0023 private bounded owner audit/history. Marked the ADRs Accepted and updated the Master Architecture to revision 2.11. **Approval is not evidence of implementation** for notifications or audit, which remain unfinished.
- The first follow-on code batch implements signed `TrustTransition` revocation/tombstones in `ProductIdentityState` schema v3 with backward-compatible v1/v2 loading; snapshots with no revocation remain v2. Revoked trust survives restart, cannot be re-paired under the same DeviceId, and is excluded from active-peer counts.
- Both Linux GPUI and Android Compose Owner interfaces now offer double-confirmed revocation of a precise typed/full validated peer ID (display remains abbreviated); platform signing authority is required, with persist-before-apply and fail-closed runtime invalidation/restart. Platform identity stores check expected previous snapshots. Linux identity commits acquire an OS file lock to serialize writers. Regression tests cover wrong signer, tamper, revocation reload/CAS, mobile summary, and Android runtime restart failure.
- Prior pushed commit `54d7612d4c598441cd7bd4e6845e940f47ee1248` passed Rust CI but **failed Android CI** run `36988646080` because Kotlin called `release()` on a nullable multicast lock. The null-safe correction is included in this batch.
- **New verification is pending.** Local command execution tools are unavailable (failed to start), so local format/lint/build/tests were *not run*. Run the consolidated Rust+Android CI on the next pushed exact head; fix any format/compile/test failures in grouped review. Actual physical-device revocation/reconnect and normal pairing/network/file/clipboard evidence remains owner-pending.
- **Remaining Phase 2:** secure notification codec/runtime/Android listener/Linux UI with consent and exact policy; protected bounded local audit/history storage+owner surface; remaining production revocation/platform evidence and UX polish. Phase 2 is not complete, and Phase 3 remains out of scope.

## 2026-10-02 — Notification/audit domain batch pending verification

- After previous PR #79 exact head `31fc355b4d5d671743fca247480a0f3c9225c839`, Fuzz Smoke run `37010549008` passed, but Rust CI run `37010548976` failed at `cargo fmt --check` and the Android job failed compiling Rust UniFFI: `ProductIdentityError::PeerNotFound` and `Revocation(_)` were missing from an exhaustive mobile error mapping. This batch applies all exact Rustfmt changes reported by that completed CI run and completes the mapping, without triggering intermediate CI.
- Implemented shared `notifications.read` v3.0 **bounded version-1 event payload contract** (`docs/protocol/NOTIFICATIONS-V3.md`) with closed event types, strict binary/UTF-8/size/redaction tests, and a shared agent `NotificationMirror` consent+exact-policy+trusted-session-context admission gate, bounded/coalescing queue and opt-out/policy-revision/session-invalidation tests. **Not a live platform notification pipeline yet**: Android OS listener, runtime capability advertisement/request/event bridging and Linux reader UI are still outstanding.
- Added `crosslab-core::AuditHistory` typed privacy-safe, content-free, bounded 30-day/1,024-event snapshot and redacted CSV export; `docs/protocol/OWNER-AUDIT-V1.md` records precise local codec and golden fixture. **No production history persistence yet**: independent protected Android/Linux storage/currentness, authenticated owner view and durable event wiring remain outstanding. No extra private data is logged.
- Security review closed a potential wrong-device UI race: Android revocation confirmation captures the device inventory generation and checks it again before resolving a full immutable peer ID. Linux existing confirmation already binds a full typed DeviceId.
- GitHub commit/branch head for this batch must be recorded from the next pushed checkpoint. Local Cargo/Gradle commands could not be executed due failed container environment; exact-head full Rust+Android CI, hardware pairing and revocation evidence are pending. Continue using consolidated review/corrections. **Phase 2 is still open.**

### Verified source checkpoint for the pending integration gates

- Branch: `fix/phase2-linux-pairing-sdk-bootstrap`; implementation source commit `bd38119fb326ede827ac5beea924fa2c561b845e`; review PR: [#79](https://github.com/untitled12121/Cross-Lab/pull/79). This checkpoint follows failing CI head `31fc355b4d5d671743fca247480a0f3c9225c839` and contains its grouped fixes.
- Implementation includes `crates/protocol/src/notification.rs` v3.0 typed binary profile with strict parser, `crates/agent/src/notification.rs` explicit OS+owner+runtime+per-peer policy/auth-context gate and bounded queue, and `crates/core/src/audit.rs` redacted bounded in-memory history/codec. Android revocation generation replay/race check has a separate Kotlin regression.
- **Exact next action:** examine completed Rust/Android/Fuzz CI at the newest PR #79 head. Group and fix all verified compiler, formatting, security and test failures in one repair batch. Once source green, implement the *remaining* actual Android Notification Access listener and owner opt-in, authenticated notification runtime event bridge, Linux reader UI, and independently protected platform audit stores plus owner read/export/delete in cohesive vertical slices. Real-device LAN pairing and revocation/transfer evidence is still owner-required; no Phase 3 or completion claim.
- **Verification:** New Rust, Android, fuzz and physical evidence have **not** passed at this source commit. This session's shell/container command runner failed to start; no local `cargo fmt`, `cargo check`, `cargo clippy`, `cargo test` or Gradle test was performed. CI must run on the pushed exact head.

## 2026-10-02 — Consolidated owner audit/notification consent batch

- Exact source commit for this unpushed batch: `25f314aff778d7334d5b4a4fbbb58089ec4b3d40` (parent `214ca23d6882c75ae5347e9004b693693343a43d`), branch `fix/phase2-linux-pairing-sdk-bootstrap`, review PR #79.
- Previous CI at `214ca23d6882c75ae5347e9004b693693343a43d`: Fuzz Smoke passed (run 37013243141), Rust failed `cargo fmt --check`, Android failed compile because `MobileProductIdentity` omitted `trusted_peer_device_ids` and `revoked_peer_ids`. Exact Rustfmt hunks were applied and mobile FFI full-ID/active/revoked fields reconstructed with tests. No build claims until new exact-head CI.
- Implemented separate native protected **owner audit/history** on both platforms. Rust typed codec and mobile FFI produce no sensitive payload or raw device identifiers. Android AndroidKeyStore/HMAC + `AtomicFile` currentness checks and Linux Secret Service + private XDG file lock/atomic state both require intact single current anchor; missing/corrupted/rollback state fails closed. Owner views show bounded recent records and dropped count; clear is confirmed and preserves trust tombstones; CSV export is explicit (Android SAF file, Linux clipboard). Successful owner-device revocation is the first event source. Other event sources remain to integrate.
- Added Android `NotificationListenerService`, OS Notification Access settings, private owner opt-in and distinct content-consent toggles. Defaults are disabled and the listener intentionally does not read/retain notification payloads until a verified authenticated subscription/transport bridge is implemented. Notification v3 live transmission, Android→Linux event routing, Linux in-app receiver and physical consent/reconnect checks are **not complete**.
- Security review: after committing signed revocation, Android tears down/reloads trusted runtime **before** optional audit writes; Linux cancels owner clear/revoke confirmation when navigating away. Added store/consent/selection/test cases. These new changes are not local-tested: command runners are unavailable.
- CI efficiency: a cheap Rust `format` prerequisite now gates heavy Rust and Android compilation, reducing repeated expensive CI waste. Full Rust+Android gates remain required when source formatting passes.
- **Next exact task:** Review newly pushed PR #79 exact-head *format* CI and Rust+Android/Fuzz results; group fixes once. Then implement live notification listener-to-agent subscription, negotiated authenticated request/events with strict policy, Linux reader UI, remaining audit event sources, and real Linux+Android pairing/clipboard/file/transfer/revoke/notification tests. Update docs with actual verification and no Phase 2 completion claim until software+hardware evidence. Do not begin Phase 3.

## 2026-10-02 — Consolidated format gate + on-disk retention correction

- Prior branch head: `906460f9748ba3a38ddc183f524d637d4f3f6201`. New cheap format-first workflow run `37017420509` stopped at Rustfmt and correctly **skipped** the expensive Rust and Android jobs. The twenty exact formatter hunks across `apps/desktop/src/features/audit/mod.rs`, `apps/desktop/src/features/mod.rs`, `apps/desktop/src/pages/control_center/page.rs`, `crates/core/src/audit.rs`, and `crates/mobile-ffi/src/audit.rs` were applied together, without speculative full CI reruns.
- Reviewed retention semantics: history decoding pruned expired records in memory but would otherwise leave expired records on protected storage until the next owner edit. Both Android Keystore and Linux Secret Service audit stores now atomically persist the pruned snapshot at an advanced protected revision only when its bytes change. The signed device-trust store is untouched.
- This unpushed correction source is `c8afce469d9580512492d4526510d20adcfca8b6`, parent `906460f9748ba3a38ddc183f524d637d4f3f6201`. Final pushed head/CI result must be checked separately; no local Rust/Android toolchain was available and these fixes remain **unverified**.
- Exact next task: after push inspect new format gate, then consolidated Rust and Android CI/fuzz once finished. Group any compile/lint/security/test errors into one correction batch. Complete actual Android authenticated notification subscription/event delivery and Linux reader plus the remaining audit event producer coverage and Linux+Android owner-hardware tests. **Phase 2 remains unfinished.**

## 2026-10-02 — Authenticated Android-to-Linux notification integration (new verification pending)

- Previous pushed PR #79 head: `26d15f9f42e1c1c76b213f84533cbbf183b9421e`. **Verified evidence:** format, Android and Fuzz Smoke succeeded; Rust CI failed `cargo check` on a moved `NotificationPayload::Removed` value in the protocol test. Fixed the exact Rust ownership test error without cloning; no local toolchain available. This new implementation source review checkpoint is `e8308b5b7054da2c09e6dbe83b829df1c12812ca` (detached until push).
- Notification v3 controller/transport slice added using the existing authenticated control channel and `RuntimeActor`. Linux owner explicitly initiates `notifications.read/subscribe`, waits for a matching request ID/session + exact v3 empty success before showing bounded plain-text posted/removed events. Unknown/malformed/replayed, unapproved, session-switched events fail closed. The new agent inbox is transient RAM only, maximum 64 events.
- Android source emits only after `NotificationListenerService` OS access, owner mirroring permission, foreground runtime, v3 capability negotiation, trusted session, exact durable per-peer **Allow** (no implicit Ask approval) and accepted subscription. Android Owner includes exact connected-peer Allow/Deny, with runtime and protected adapter comparing the full peer ID again at commit time to avoid wrong-peer changes on reconnection. Source runtime policy/session/OS consent changes invalidate subscriptions and queued events.
- Android OS callback checks source subscription before reading any event fields; protected/system/secret/ongoing categories are excluded or redacted, optional content disabled by default, bounded `CharSequence` truncation avoids copying large notifications. Raw Android keys stay solely in bounded process memory and map to random opaque event IDs; no file, icon, actions, package name, clipboard or content history is sent/logged. Android data enters through narrow UniFFI methods, not privileged UI inheritance.
- Added negative/regression tests for malformed subscribe, wrong version/type, mismatched session/approval, bounded queue, raw-platform-key non-disclosure, protected redaction, Android bounded display and peer-switched policy edit rejection. **Tests authored but not run locally.** Consolidated GitHub CI is required at pushed exact source head; no source is claimed software-verified or hardware-verified by these notes.
- **Unfinished Phase 2:** investigate exact Rust/Android/format/Fuzz CI results and consolidate fixes; run Linux/Android physical QR pair/reconnect/revoke, notification delivery/denial/opt-out/foreground recovery, keyboard/accessibility and resource measurements. Remaining audit producers (authentication, transfer terminal, pairing, subscription enable/disable, etc.) and complete protected audit security tests remain. Do not mark Phase 2 done or begin Phase 3. Keep work in one coherent CI batch.

## 2026-10-02 — One-pass notification integration formatting correction

- New integrated Android→Linux source head `e1f6014a5e41a6d563a605cb5a4be85b57f25131`: cheap format-first CI run `37024984704` failed its **Source format** job and consequently **skipped** heavy Rust and Android jobs. This is expected verification failure, not a successful feature test; Fuzz Smoke is tracked separately.
- Applied every one of the **33 reported Rustfmt hunks across nine files** from the exact failed format log in one correction tree, without manually guessing at formatting or rerunning heavy CI. The consolidated correction must be tested on the newly pushed exact head; previous software results do not establish this batch.
- **Next exact task:** inspect the new source-format prerequisite once; if it passes, await Rust/Android/security CI and Fuzz. Group and resolve verified compiler/security failures, then hardware-test Android Notification Access + owner/content opt-in, exact connected-peer Allow/Deny, Linux authenticated subscription, posted/removed/redaction and reconnect/revocation/OS withdrawal. Audit event coverage and physical Phase 2 gates remain unfinished.

## 2026-10-02 — Review of consolidated notification CI and Linux revocation integration

- Exact PR #79 head at review: `3385c7b29f46c1023dddd598d5b99519035fb3e8`. CI run `37025221654` completed: **Source format passed, Android passed, Rust failed at `cargo check`**, and Fuzz Smoke run `37025221641` **passed**. Rust reported seven missing Linux pairing API/method errors: no `revoke_product_peer` export, and no `trusted_peer_device_ids` / `revoked_peer_ids` accessors on `ProductIdentityPresentation` and `DesktopPairingInvitation`. No broader Rust test/Clippy/build pass can be inferred from this failed job.
- Staged one focused fix: desktop pairing now exposes a typed full `DeviceId` inventory alongside shortened display-only IDs and signed revoked tombstone display rows, shared by both identity presentation and invitation. Restored `revoke_product_peer` using existing delegated device-signing authority, verified identity provider state and protected identity-store `commit_payload_if_current` before the UI tears down sessions and records a non-payload audit event. Added regression test for full-ID precision and decoded signed terminal revocation; did not substitute truncated UI IDs for secure selectors.
- Also corrected a discovered cross-capability lifecycle bug in the notification mirror: `RequestCancelled` only clears a subscription if its **own exact subscribing request ID** matches; a clipboard/file cancellation can no longer tear down notifications. Added matching and unrelated cancellation tests, with no change to approved ADR-0021 wire format or policy model.
- These new changes are **source-staged and not locally compiled** (GitHub connector supports file/commit operations; command runner/toolchain unavailable). **Next exact task:** push the combined correction, inspect source format once, then review exact-head Rust check/tests/Clippy, Android and Fuzz. Group any verified residual issues in a new batch. Complete remaining audit event producers, physical Linux+Android pairing/revocation/notification permission/denial/redaction/reconnect tests, native polish and hardware evidence. **Phase 2 still open; do not merge or start Phase 3 without required verification.**
