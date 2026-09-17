# M10 First Platform Slice Implementation Plan

> **Execution note:** Use the Superpowers executing-plans and test-driven-development workflows for implementation. Complete tasks in order, keep commits small, and update `docs/development/CURRENT.md` at durable checkpoints.

**Goal:** Deliver the first real Cross-Lab Linux desktop + Android vertical slice: native status UIs backed by one shared Rust runtime, real Quinn local/LAN transport, existing Cross-Lab channel-bound authentication/session semantics, clean reconnect/revocation behavior, and the accepted renderer-neutral design-system contract.

**Architecture:** Add one shared unprivileged `crosslab-runtime` coordination crate consumed by the desktop application and the mobile FFI façade. Keep Quinn/TLS and session-bootstrap implementation inside `transports/quic`; keep GPUI and Android APIs outside domain crates; expose Android through one small UniFFI façade. Package one canonical theme document/schema from `design/` into each native application and map it locally into GPUI/Compose types. Do not promote Iroh in M10.

**Verified implementation references (2026-09-17):** GPUI Kit `0.6.1`; UniFFI `0.32.1`; Android Gradle Plugin `9.4.0`; Gradle `9.6.0`; JDK `17`; Kotlin `2.4.20`; Compose BOM `2026.08.00`; Android compile/target API `36`. Re-check exact versions immediately before the commit that first adds each dependency.

**Security boundary:** M10 does not invent a production Quinn certificate-provisioning scheme or bypass TLS verification. ADR-0008 explicitly left production certificate provisioning undecided. The transport API accepts externally provisioned certificate/trust material behind transport-local types; M10 real-device evidence may use explicit development/test provisioning. Release builds must not silently fall back to development certificates or permissive verification. Android persistent production identity keys remain gated on a Keystore/StrongBox adapter.

**Darkmatter gate:** The canonical Darkmatter palette is not present in the verified project sources. Do not infer it. Implement the contract, resolver, adapters, and Ayu Light path; treat Darkmatter visual completion as blocked until the owner supplies the authoritative source.

---

## Task 1: Integrate the approved M10 architecture

**Files**
- Modify: `docs/adr/ADR-0012-cross-platform-design-system.md`
- Modify: `docs/adr/README.md`
- Modify: `docs/architecture/MASTER-ARCHITECTURE.md`
- Modify: `docs/superpowers/specs/2026-09-17-m10-platform-shell-design.md`
- Modify: `docs/development/CURRENT.md`

**Steps**
1. Mark ADR-0012 `Accepted` with the approval date.
2. Register ADR-0012 in the ADR index.
3. Bump the Master Architecture revision and reconcile the desktop/mobile design sections:
   - renderer-neutral semantic theme contract is normative;
   - radius `0` remains the baseline but is a token rather than a hard-coded renderer invariant;
   - Darkmatter/Ayu Light/System identities are recorded;
   - Android remains Kotlin-native and M10 selects Jetpack Compose for its first UI shell;
   - platform renderers may adapt layout/accessibility while preserving Cross-Lab product semantics;
   - theme data is local presentation state and carries no authority.
4. Mark the M10 design `Approved` and update its implementation gate text.
5. Update `CURRENT.md` to record the accepted design/ADR and exact implementation next task.
6. Verify the branch diff contains only the intended architecture/design/plan checkpoint.
7. Open a PR, require exact-head CI, merge with expected head, and verify post-merge `main` CI before production code starts.

---

## Task 2: Create the canonical renderer-neutral theme contract

**Files**
- Create: `design/schema/theme-v1.schema.json`
- Create: `design/themes/README.md`
- Create: `design/themes/ayu-light.json`
- Create: `design/fixtures/theme-v1-valid.json`
- Create: `design/fixtures/theme-v1-invalid.json`
- Modify: `.github/workflows/ci.yml`

**Test first**
1. Add a CI validation step that parses every committed canonical/fixture theme JSON and rejects malformed JSON.
2. Add renderer-side typed parsing tests in Tasks 6 and 8 before feature UI consumes the files.

**Contract**
Use schema version `1` and require typed semantic sections for:
- identity: stable theme id, display name, appearance (`light` or `dark`), schema version;
- semantic OKLCH colors: background, foreground, surface, surface-foreground, primary, primary-foreground, secondary, secondary-foreground, muted, muted-foreground, accent, accent-foreground, destructive, destructive-foreground, border, input, ring, selection;
- typography: sans/mono families plus named text scales with size, line height, weight;
- radius: none/sm/md/lg/xl/full;
- spacing: xxs/xs/sm/md/lg/xl/xxl;
- density/control metrics: border width, compact/default control height, row height, icon size/stroke;
- elevation/shadow and motion values actually consumed by M10 components.

Represent OKLCH structurally (`l`, `c`, `h`, optional `alpha`) rather than as CSS strings so native parsers do not implement CSS.

**Ayu Light**
Use GPUI Kit's maintained Ayu material only as a reference. Translate only the semantic values M10 consumes. Record the exact upstream revision/version and Apache-2.0 source reference in `design/themes/README.md`. Do not copy component-specific GPUI Kit schema into Cross-Lab.

**Darkmatter**
Do not create `darkmatter.json` until the authoritative owner source is supplied. Tests for resolver behavior may use synthetic in-test theme objects; do not commit invented Darkmatter values as product data.

**Verify**
- JSON syntax validation passes.
- Schema has no renderer-specific (`gpui`, `compose`, `swiftui`, `css`) field names.
- No security/policy/network fields are present.

---

## Task 3: Extract shared application runtime coordination from the simulator

**Files**
- Create: `crates/runtime/Cargo.toml`
- Create: `crates/runtime/src/lib.rs`
- Create: `crates/runtime/src/node.rs`
- Create: `crates/runtime/src/status.rs`
- Create: `crates/runtime/src/tests.rs`
- Modify: `apps/sim/Cargo.toml`
- Modify: `apps/sim/src/node.rs`
- Modify: `apps/sim/src/lib.rs`
- Modify: root `Cargo.toml`

**Test first**
1. Move/copy the meaningful existing `SimNode` behavior tests into `crosslab-runtime` and make them fail against the initial empty API.
2. Add tests for:
   - constructing only from an active `LogicalSession`;
   - bounded request/event state inherited from `ControlDispatcher`;
   - transport loss closes session authority;
   - peer revocation clears dispatcher state and closes the transport;
   - owner/device-signing authority replacement fails closed;
   - status snapshots never expose credentials, signing keys, channel-binding bytes, or payloads.

**Implementation**
Introduce a small generic `RuntimeNode<T: TransportConnection>` (or the narrowest equivalent ownership form supported by the current trait) containing the reusable logic currently living in `SimNode`: logical session, dispatcher, transport, policy, local capabilities, and network class.

Add typed UI-facing snapshot structs containing only safe summaries needed by M10, for example:
- local/peer `DeviceId`;
- `SessionId` only while active;
- orthogonal trust/connectivity/session state;
- negotiated protocol/features/capability ids;
- transport metadata already redacted/normalized for presentation.

Do not add platform names, GPUI types, Kotlin types, UniFFI derives, or Quinn types to `crosslab-runtime`.

Refactor `apps/sim` to consume `crosslab-runtime` instead of retaining a second coordination implementation. Avoid a compatibility wrapper unless it materially reduces churn; prefer re-export/type alias only where useful.

**Verify**
- `cargo test -p crosslab-runtime`
- `cargo test -p crosslab-sim`
- workspace check/clippy/tests.

---

## Task 4: Productize Quinn endpoint/bootstrap mechanics without changing identity semantics

**Files**
- Create: `transports/quic/src/endpoint.rs`
- Create: `transports/quic/src/session.rs`
- Create: `transports/quic/src/session/tests.rs`
- Modify: `transports/quic/src/lib.rs`
- Refactor: `transports/quic/src/session_auth_tests.rs`
- Refactor if justified: `transports/quic/src/record.rs`

**Test first**
Extract the existing successful session-auth test flow into public transport-local APIs while preserving all negative tests. Add failing tests for:
- client/server authenticated session success;
- wrong exporter binding rejected before active;
- proof replay on a fresh connection rejected;
- unknown/untrusted peer rejected;
- revoked peer rejected;
- malformed/oversized bootstrap frame rejected;
- bootstrap timeout/cancellation closes the connection;
- fresh reconnect produces a distinct session id;
- public endpoint API cannot return a usable `RuntimeNode` before session authentication succeeds.

**Implementation**
Create transport-local input types for externally provisioned TLS material using DER bytes, socket/server-name configuration, and bounded timeouts. Keep rustls/quinn concrete types private where possible.

Refactor the current test-only bootstrap sequence into reusable initiator/responder functions that:
1. await the full QUIC/TLS handshake;
2. derive ADR-0008 `quic-tls-exporter-v1` exactly;
3. exchange bounded `SessionAuthHello` and proof records;
4. build the existing `SessionAuthTranscriptV1` and role-separated proofs;
5. call existing `LogicalSession::authenticate` with local authoritative trust/owner state;
6. create `QuicTransportConnection` only after both local verification conditions for the side are satisfied.

The API must not treat TLS certificate identity as Cross-Lab `DeviceId`, trust, or permission. No 0-RTT authority.

**Provisioning constraint**
Do not add accept-all certificate verification or an implicit self-signed certificate fallback. Development/test certificates are supplied explicitly by the caller. Production certificate issuance/pinning lifecycle remains a separate decision if/when a release consumer needs it.

**Verify**
- `cargo test -p crosslab-transport-quic`
- existing M8/M9-compatible session-auth tests remain green.

---

## Task 5: Add an application runtime actor for lifecycle-safe consumers

**Files**
- Create: `crates/runtime/src/actor.rs`
- Create: `crates/runtime/src/command.rs`
- Modify: `crates/runtime/src/lib.rs`
- Modify: `crates/runtime/src/tests.rs`

**Test first**
Add tests proving:
- one start produces one runtime task;
- repeated start is idempotent or explicitly rejected;
- stop cancels tasks and closes active transport/session state;
- network loss transitions connectivity without preserving stale session authority;
- reconnect requires a fresh authenticated session;
- command and state/event queues are bounded;
- slow subscribers cannot create an unbounded queue;
- dropping the public handle results in deterministic shutdown semantics.

**Implementation**
Use Tokio bounded `mpsc`/`watch` primitives already present in the workspace. The actor owns the mutable runtime/session state; UI/FFI consumers interact through typed commands and immutable snapshots. Avoid polling loops: drive work from commands, transport events, and lifecycle signals.

Do not add global mutable state or a process-wide singleton in Rust. Android application scoping and desktop app ownership belong to their platform shells.

---

## Task 6: Implement the Linux desktop shell and local design adapter

**Dependency check before editing Cargo**
Re-verify latest stable GPUI Kit release. Expected reviewed version: `0.6.1` (Apache-2.0) using the maintained GPUI pre-release package family it pins.

**Files**
- Create: `apps/desktop/Cargo.toml`
- Create: `apps/desktop/src/main.rs`
- Create: `apps/desktop/src/lib.rs`
- Create: `apps/desktop/src/pages/mod.rs`
- Create: `apps/desktop/src/pages/devices/page.rs`
- Create: `apps/desktop/src/pages/devices/layout.rs`
- Create: `apps/desktop/src/pages/devices/_components/mod.rs`
- Create: `apps/desktop/src/components/ui/mod.rs`
- Create only consumed primitives under `apps/desktop/src/components/ui/`
- Create: `apps/desktop/src/features/devices/mod.rs`
- Create: `apps/desktop/src/features/appearance/mod.rs`
- Create: `apps/desktop/src/features/appearance/theme.rs`
- Create: `apps/desktop/src/features/appearance/tests.rs`
- Modify: root `Cargo.toml`

**Test first**
Add parser/mapper tests using `design/fixtures/theme-v1-valid.json` and invalid/missing-field fixtures. Test `System` resolution as pure logic from an injected appearance enum; do not make tests depend on host desktop appearance.

Add devices feature tests that map `crosslab-runtime` status snapshots into display state without exposing secret/session-binding bytes.

**Implementation**
- GPUI `page.rs` composes only.
- `layout.rs` owns shell/navigation layout.
- `features/devices` owns state/controller interaction with the runtime handle.
- `features/appearance` owns local theme selection and mapping into GPUI/GPUI Kit theme values.
- `components/ui` contains only primitives needed by the device/status screen.
- Use sharp radius-0 baseline, compact spacing, thin separators, keyboard focus, accessible labels, and restrained status color use.

Do not add networking/crypto calls inside GPUI components. Do not create a full speculative component library.

**Darkmatter behavior**
The appearance resolver may know the `Darkmatter` theme id, but production UI must not substitute guessed values. Until the canonical Darkmatter file exists, a dark-theme load must return a typed `ThemeUnavailable`/configuration error rather than silently using a fabricated palette. Keep the shell usable in Ayu Light for development.

**Verify**
- desktop crate unit tests;
- desktop build on Linux CI;
- workspace fmt/check/clippy/tests.

---

## Task 7: Add the narrow UniFFI mobile façade

**Dependency check before editing Cargo**
Re-verify UniFFI. Expected reviewed version: `0.32.1`, MPL-2.0. Depend on the crate; do not copy/modify UniFFI source into Cross-Lab. Record the dependency license in the repository's third-party/dependency documentation if such a manifest exists; otherwise add a focused M10 dependency note under the plan/research docs.

**Files**
- Create: `crates/mobile-ffi/Cargo.toml`
- Create: `crates/mobile-ffi/src/lib.rs`
- Create: `crates/mobile-ffi/src/runtime.rs`
- Create: `crates/mobile-ffi/src/dto.rs`
- Create: `crates/mobile-ffi/src/error.rs`
- Create: `crates/mobile-ffi/src/tests.rs`
- Modify: root `Cargo.toml`

**Test first**
Add Rust tests for the exact exported contract:
- start/stop lifecycle;
- duplicate start handling;
- snapshot conversion/redaction;
- connection/session status conversion;
- bounded event subscription behavior;
- error mapping contains no sensitive payloads;
- no exported function returns private keys, credentials, channel-binding bytes, raw Quinn types, or internal domain references.

**Implementation**
Expose only M10-consumed UniFFI-safe records/enums and a lifecycle object/handle. Keep DTOs intentionally small. Use callbacks or an explicit bounded snapshot/event mechanism supported cleanly by UniFFI; do not export every internal crate/type.

Do not expose the theme contract through FFI. Android loads presentation data locally.

Add deterministic binding-generation instructions/tooling only now that mobile binding generation is a real requirement. Prefer a small script/Gradle integration over a broad `xtask` unless more cross-platform automation is immediately justified.

---

## Task 8: Scaffold the Android Kotlin/Compose application

**Verified toolchain baseline**
- AGP `9.4.0` (requires/defaults Gradle `9.6.0`, JDK `17`; supports through API 37)
- Kotlin `2.4.20`
- Compose BOM `2026.08.00`
- compileSdk/targetSdk `36` for Android 16 compatibility/current Play requirement
- NDK `28.2.13676358` unless the Rust/UniFFI build proves a concrete reason to override it

Re-check these values immediately before committing the Gradle files.

**Files**
- Create: `apps/android/settings.gradle.kts`
- Create: `apps/android/build.gradle.kts`
- Create: `apps/android/gradle.properties`
- Create: `apps/android/gradle/libs.versions.toml`
- Create: `apps/android/app/build.gradle.kts`
- Create: `apps/android/app/src/main/AndroidManifest.xml`
- Create: `apps/android/app/src/main/kotlin/dev/crosslab/android/CrossLabApplication.kt`
- Create: `apps/android/app/src/main/kotlin/dev/crosslab/android/MainActivity.kt`
- Create: `apps/android/app/src/main/kotlin/dev/crosslab/android/features/devices/...`
- Create: `apps/android/app/src/main/kotlin/dev/crosslab/android/features/appearance/...`
- Create: `apps/android/app/src/main/kotlin/dev/crosslab/android/components/ui/...` only as consumed
- Create: Android unit tests alongside feature packages
- Modify: `.github/workflows/ci.yml`

`dev.crosslab.android` is an M10 development namespace, not a release/store identity commitment; document any future production application-id decision before signing/publication.

Do not choose a minimum Android OS version by memory. Before setting `minSdk`, verify it against Cross-Lab's desired device support, UniFFI/Rust native requirements, and the Android APIs actually used by this slice. Record the chosen minimum and rationale in this plan/CURRENT before first release build.

**Test first**
- Theme parser/mapping tests against the same canonical `design/` fixture files.
- Theme selection resolver tests for System/light/dark ids.
- RuntimeController tests for foreground/background, duplicate start/stop, network lost/available, and shutdown.
- Devices feature state tests for connected/disconnected/trusted/revoked display states.

**Implementation**
- Use Compose UI/Foundation primitives and Cross-Lab tokens; Material may provide behavior/accessibility utilities where useful but must not define product styling.
- `CrossLabApplication`/a dedicated lifecycle owner owns the mobile façade handle; composables observe state only.
- Network callbacks/lifecycle events send typed commands to the Rust façade; they do not mutate Rust internals directly.
- Keep production background/foreground-service behavior disabled unless the slice establishes a concrete always-on requirement and implements the OS-visible contract.

**Native build**
Add deterministic Rust Android ABI builds for the exact ABIs needed by M10 evidence. Start with the real test device ABI plus one common emulator/CI ABI only if CI executes it; do not build every ABI by convention. Package generated UniFFI Kotlin sources and native library without hand-copying generated files into multiple divergent locations.

**Verify**
- `./gradlew :app:testDebugUnitTest`
- `./gradlew :app:assembleDebug`
- binding generation/native library task is reproducible from a clean checkout.

---

## Task 9: Wire desktop/mobile consumers to the shared runtime and Quinn development provisioning

**Files**
- Modify: `apps/desktop/src/features/devices/*`
- Modify: `apps/android/.../features/devices/*`
- Modify: `crates/mobile-ffi/src/runtime.rs`
- Add narrowly scoped development provisioning/config modules under each app only if required
- Add integration tests under `tests/` or the consuming crate with the smallest existing pattern

**Test first**
Build a two-peer integration harness using the product Quinn/session APIs, not private test helpers. Provision real owner/device credentials using the existing identity APIs and explicit development transport certificates.

Prove:
1. Linux-side peer listens/connects using explicit local/LAN Quinn configuration.
2. Android-side façade reaches `Connected/Authenticated` only after existing Cross-Lab proofs activate the session.
3. both status snapshots agree on peer ids/session id and report trusted/active state;
4. disconnect causes session closure;
5. reconnect yields a fresh `SessionId`;
6. after revocation, reconnect is rejected and UI-facing state becomes revoked/disconnected without stale authority;
7. no secret bytes enter status/log output.

**Implementation**
Connect each UI controller to the same runtime command/snapshot model. Keep development credential/certificate provisioning clearly separated and unavailable in release configuration by default.

Do not introduce mDNS/BLE discovery in this task. Endpoint selection may be explicit development configuration for the first real-device evidence.

---

## Task 10: Add CI gates and collect real-device evidence

**Files**
- Modify: `.github/workflows/ci.yml`
- Create: `docs/research/M10-platform-evidence.md`
- Modify: `docs/development/CURRENT.md`

**CI**
Preserve the existing Rust gate exactly:
```text
cargo metadata --locked --no-deps --format-version 1
cargo audit
cargo fmt --check
cargo check --workspace --all-targets --all-features
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-features
```

Add only reproducible host-appropriate mobile/desktop checks. Do not make Linux CI pretend it performed real Android-device lifecycle evidence.

**Manual/real-device evidence**
Record device/OS/build identifiers without secrets and verify:
- Android foreground -> background -> foreground;
- local network loss -> restoration;
- desktop/Android disconnect -> fresh authenticated reconnect;
- revocation -> reconnect denial;
- clean explicit shutdown;
- no duplicate runtime after lifecycle churn;
- bounded resource behavior at idle and during reconnect;
- theme selection behavior for Ayu Light and, only after authoritative source is supplied, Darkmatter/System-dark.

M10 cannot be called complete without this evidence. CI success alone is insufficient for the real-device lifecycle criterion.

---

## Task 11: Final M10 integration

**Files**
- Modify: `docs/development/CURRENT.md`
- Update architecture/research dependency notes only if implementation evidence changed an approved design assumption.

**Steps**
1. Run the full Rust gate fresh.
2. Run Android unit/build/binding-generation gates fresh.
3. Run desktop Linux build/tests fresh.
4. Verify real-device evidence is recorded and contains no secrets.
5. Verify `Darkmatter` completion only if the authoritative source has been integrated; otherwise keep that criterion explicitly open and do not claim full M10 visual completion.
6. Update `CURRENT.md` with exact commit/run/evidence identifiers and the next vertical slice.
7. Open the final PR, require exact-head CI, merge with expected head, verify post-merge `main`, and checkpoint `CURRENT.md` if the merged file cannot yet contain post-merge identifiers.

## Completion definition

M10 is complete only when the approved design's success criteria are all evidenced. If the authoritative Darkmatter source or required real Android device evidence remains unavailable, land verified intermediate milestones but keep M10 explicitly in progress rather than weakening or silently dropping the gate.
