# M10 Linux + Android Platform Evidence

This document records evidence for the M10 first real Linux + Android platform slice. CI evidence and real-device evidence are separate. **CI success does not satisfy the real-device lifecycle criterion.**

## Evidence status

- Task 9 integrated baseline: `main` at `f19aa4bb89d744f2ebf0f0fa2b204929fd8495a4`.
- Host CI evidence: **verified** on Task 10 implementation head `fad35fd2f3535d148a64287fdde391d1613b720e`, GitHub Actions `35438748708`.
- Linux real-device evidence: **pending**.
- Android real-device evidence: **pending**.
- Darkmatter/System-dark evidence: **blocked** until an authoritative Darkmatter palette is supplied.
- M10 completion: **open** - mandatory physical Linux + Android lifecycle/security evidence is still pending.

## Safety rules for evidence

Do not record or attach:

- private keys, signing seeds, TLS private keys, credentials, authentication proofs, channel-binding bytes, provisioning-file contents, recovery material, tokens, passwords, or raw sensitive payloads;
- Android serial numbers, MAC addresses, local IP addresses, Wi-Fi SSIDs, usernames, home-directory paths, or other unnecessary device identifiers;
- full logs unless they have been reviewed and redacted.

Safe evidence should use commit/run identifiers, OS/build versions, architecture, application version, coarse device model, observed presentation state, and pass/fail results. When demonstrating reconnect, record only that the new session identifier differs from the old one; do not copy session identifiers into this document.

## Host CI evidence

The Task 10 CI gate must keep the existing Rust gate unchanged:

```text
cargo metadata --locked --no-deps --format-version 1
cargo audit
cargo fmt --check
cargo check --workspace --all-targets --all-features
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-features
```

Additional host-appropriate checks:

- default Linux desktop build;
- Linux desktop build with `development-provisioning`;
- deterministic UniFFI Kotlin generation;
- Android JVM unit tests;
- default Android debug assembly;
- Android debug assembly with `-PcrosslabDevelopmentProvisioning=true`.

These checks prove reproducible host builds only. They do not prove Android process lifecycle, radio/network transitions, or physical-device reconnect behavior.

Task 10 host verification: GitHub Actions `35438748708` on `fad35fd2f3535d148a64287fdde391d1613b720e` passed the preserved Rust gate, default Linux desktop build, Linux development-provisioning desktop build, deterministic UniFFI Kotlin generation, Android unit tests, default Android debug assembly, and Android development-provisioning debug assembly.

## Real-device setup

Use only the throwaway generator below. **Never place a real owner-root key, real Device Signing key, or production device key in the M10 development provisioning files.**

On the Linux host, choose an unused UDP port and the Linux machine's LAN address that the Android device can reach. Do not copy that address into this evidence document.

```bash
PROVISION_DIR="/tmp/crosslab-m10-evidence"
cargo run -p crosslab-transport-quic \
  --example m10_provision \
  --features development-provisioning \
  -- \
  --server-remote "<LINUX_LAN_IP>:45777" \
  --out-dir "$PROVISION_DIR"
```

The generator creates `desktop.json` and `android.json` with synthetic credentials, restricts them to private files on Unix, refuses to overwrite existing files, and does not print secret contents. Delete both files after evidence collection.

Build and install the Android development slice on an arm64 physical device:

```bash
cd apps/android
./gradlew :app:installDebug \
  -PcrosslabDevelopmentProvisioning=true \
  --no-daemon
cd ../..
```

Copy the Android provisioning document directly into the debuggable app's private storage through `run-as`, without staging it in shared device storage:

```bash
adb shell 'run-as dev.crosslab.android sh -c "umask 077; cat > files/crosslab-development-provisioning.json"' \
  < "$PROVISION_DIR/android.json"
adb shell 'run-as dev.crosslab.android chmod 600 files/crosslab-development-provisioning.json'
```

Do not paste the output of provisioning files or `adb` device identifiers into evidence.

Build and start the Linux peer directly so its PID is the Cross-Lab desktop process rather than a `cargo run` parent:

```bash
cargo build -p crosslab-desktop --features development-provisioning
CROSSLAB_DEVELOPMENT_PROVISIONING="$PROVISION_DIR/desktop.json" \
  target/debug/crosslab-desktop &
DESKTOP_PID=$!
```

If a host firewall is active, allow only the selected development UDP port on the local/LAN interface for the duration of the test, then remove that temporary rule afterward.

Start the Android app after the Linux peer is listening. For foreground/background evidence, use normal device navigation rather than process-kill commands so `ProcessLifecycleOwner` receives real lifecycle transitions.

For the physical revocation scenario, while both peers are connected send the development-only Linux trigger:

```bash
kill -USR1 "$DESKTOP_PID"
```

The Linux runtime must transition to revoked/disconnected. Then background and foreground the Android app (or otherwise perform an explicit reconnect attempt) and verify that the desktop rejects the fresh connection. The `SIGUSR1` path exists only behind `development-provisioning`; it does not exist in the default desktop build.

After evidence collection:

```bash
kill "$DESKTOP_PID" 2>/dev/null || true
adb shell am force-stop dev.crosslab.android
rm -rf "$PROVISION_DIR"
```

Also remove the app-private development provisioning file or uninstall the debug app before using the device for unrelated testing.

## Real-device environment record

Fill this section only when physical-device evidence is collected.

| Field | Linux peer | Android peer |
| --- | --- | --- |
| Evidence ID | pending | pending |
| Cross-Lab commit | pending | pending |
| Build/application version | pending | pending |
| OS/version | pending | pending |
| Kernel/API level | pending | pending |
| Architecture/ABI | pending | pending |
| Desktop session / Android model | pending | pending |
| Development provisioning enabled | pending | pending |
| Evidence collector/date | pending | pending |

Use a coarse Android model name only. Do not record a device serial.

## Real-device procedure and results

Each scenario must be executed against the same Task 10 commit unless a row explicitly records a different commit. Record observable presentation state rather than secrets or raw protocol frames.

| Scenario | Required observation | Result | Evidence ID / notes |
| --- | --- | --- | --- |
| Initial authenticated connection | Both peers show trusted/connected/active state only after Cross-Lab authentication completes. | pending | pending |
| Android foreground -> background -> foreground | Background stops the mobile runtime; foreground creates one runtime and reconnects using fresh authentication. | pending | pending |
| Local network loss -> restoration | Both peers become disconnected without stale active-session authority; restoration yields a fresh authenticated session. | pending | pending |
| Desktop/Android disconnect -> reconnect | Reconnect succeeds with a new `SessionId`; record only `changed: yes`. | pending | pending |
| Revocation -> reconnect denial | Revocation closes/invalidates authority and a subsequent reconnect attempt is rejected. | pending | pending |
| Clean explicit shutdown | Runtime and transport shut down without a duplicate runtime or stale connected state. | pending | pending |
| Lifecycle churn | Repeated foreground/background and reconnect cycles do not create duplicate runtimes or duplicate active sessions. | pending | pending |
| Idle resource behavior | No busy polling; CPU/network activity remains bounded while idle. | pending | pending |
| Reconnect resource behavior | Reconnect attempts remain bounded and settle after success/failure without runaway background work. | pending | pending |
| Ayu Light selection | Canonical Ayu Light tokens render and remain selected as expected. | pending | pending |
| Darkmatter/System-dark | Run only after the authoritative Darkmatter palette is integrated. | blocked | authoritative palette absent |

## Suggested non-secret capture

For Linux, record only the output needed to establish the environment, for example distro/version, `uname -r`, and `uname -m`. For Android, record Android version/API level, ABI, application version, and coarse model. Review command output before copying it here.

For lifecycle/network scenarios, screenshots of the Cross-Lab status UI are preferred when they contain no secrets. If logs are needed, capture only presentation-safe state transitions and redact unrelated system/application data before attaching or summarizing them.

For idle/reconnect behavior, record the observation window and a coarse conclusion such as `idle CPU remained low; no repeated reconnect loop observed`. Do not claim numeric performance guarantees unless a repeatable measurement method and raw non-sensitive evidence are retained.

## Evidence acceptance criteria

Real-device evidence is acceptable only when:

1. the exact tested Cross-Lab commit is recorded;
2. both Linux and Android environment identifiers are present without sensitive identifiers;
3. every required lifecycle/network/security scenario above is pass/fail with an evidence reference;
4. reconnect evidence confirms a fresh session without publishing session values;
5. revocation evidence shows reconnect denial rather than only a generic network failure;
6. lifecycle churn shows no duplicate active runtime/session;
7. idle/reconnect behavior is bounded and does not depend on polling;
8. evidence has been reviewed for secrets;
9. Ayu Light is verified, while Darkmatter/System-dark remains explicitly open until its authoritative palette exists.

## Current blockers

Physical Linux + Android device execution is required to fill the pending rows. The repository/CI environment cannot manufacture this evidence. Darkmatter/System-dark also remains blocked by the missing authoritative palette.
