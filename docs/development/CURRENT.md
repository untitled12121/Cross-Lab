# Current Development State

This file is the durable resume guide for active Cross-Lab development. Git/code/tests are factual state; this file records the intended handoff.

## Current Phase

**M10 Linux + Android implementation is complete in code; physical Linux/Android camera/LAN evidence remains pending. Phase 2 Linux + Android MVP is the active implementation phase. Phase 3 adaptive networking remains explicitly out of scope until Phase 2 is complete.**

M1-M9 are complete. M10 now includes the normal QR → bounded DNS-SD → provisional Quinn → authenticated pairing → reciprocal durable trust product path on Linux + Android, with full CI verification. Owner-hardware evidence remains a separate gate and must not be inferred from CI.

## Canonical Baseline

- Current `main` after PR #75: `f048b13e4e93298dc1ad2fa1bb12a64ffcbc3aa2`. PR #75 exact head `b6683540891cfa20d409e565f8b9dfb85d25836b` passed Rust + Android CI `36912713661` before merge.
- PR #49 — Linux Add Device QR invitation UI + Android CameraX/ML Kit scanner: merged as `445bbf32178dff94f339fc1ae80447967f5da215`; exact-head CI `35533414673` green on `7d3176ffd4387d3e29982ae5fe641249d6d33445`.
- PR #50 — shared product pairing coordinator + durable reciprocal trust persistence: merged as `3af825e6f78bef4512168f587f452c1b0267b6a7`; exact-head CI `35567548228` and Fuzz Smoke `35567548208` green on `ec59f99f7c09898aa2533d8b40bd4e980fa2b022`.
- PR #51 — ADR-0016 LAN discovery profile + versioned product-pairing wire + provisional Quinn pairing channel: merged as `cf2add350ffa60056d74ac57b0a187a0297d8777`; exact-head CI `35593397861` and Fuzz Smoke `35593397844` green on `2702cc0c926cf044c65aef0376f573aaedae8c6f`.
- PR #52 — bounded DNS-SD discovery adapters: merged as `2881a8cb042207bf55a1ede9f2dc61f30ccd4eb9`; exact-head CI `35598750651` green on `2ef9ab362fb9fa59d7693662ec8de2f65e5031af`.
- PR #53 — product pairing network/platform lifecycle: merged as `9453b0e7f3e67578c79615acb1034c849a82603d`; implementation head `cc099fd7bf3335f51f54d4e09f2d002ab136e4e0` and documentation head `c5570add764ac1e223b08bdb63737abd3b182a25` both passed the full Rust + Android gate (`35641211049` / `35643137494`).
- Master Architecture revision 2.10 governs accepted ADR-0020 resumable single-file transfer v2 in addition to the established pairing, trusted-session, clipboard, and owner-policy-store architecture.
- Physical-evidence continuation branch: `m10-task10-real-device-evidence`.
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
- PR #73 — Linux file-transfer product flow merged as `881a5dc0f50943d2cc312a5396afecc07c6a1073`. PR #75 — Android SAF/ContentResolver adapter, narrow UniFFI transfer operations and shared streaming verification/retention — merged as `f048b13e4e93298dc1ad2fa1bb12a64ffcbc3aa2`; exact-head CI `36912713661` passed both Rust and Android. Active Phase 2 branch `phase2-android-file-transfer-ui` implements Task 6 owner-selected Send/CreateDocument, request-correlated Save As, receive/stream/progress/cancel/retry service, prepublication cancellation gate, and native Compose panel. This code is awaiting exact-head verification and physical Linux/Android evidence remains separate.
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

1. verify Task 6 branch `phase2-android-file-transfer-ui` with full exact-head Rust + Android CI; repair only verified failures in coherent batches, and preserve significant code checkpoints;
2. review Linux↔Android v2 cancellation and prepublication authority, request-correlated destination picker, retry/reconnect, partial recovery, content URI privacy, bounded streams and no broad storage access;
3. once fully green, mark Task 6 PR ready, merge via merge commit, then checkpoint main;
4. keep M10 physical QR/camera/LAN and file-transfer evidence as separately pending owner-hardware verification; stop before Phase 3.

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
