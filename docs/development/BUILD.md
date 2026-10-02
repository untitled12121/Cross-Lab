# Building Cross-Lab

Cross-Lab has one Rust-native build entry point for the applications that are currently configured.

## Quick start

From the repository root:

```bash
cargo crosslab setup
cargo crosslab
```

`cargo crosslab` is the default all-app build. It builds:

- the desktop application for the **current host operating system**;
- the Android arm64 debug APK when the current host is Linux or macOS.

Project dependencies are resolved by Cargo and Gradle automatically. External platform toolchains such as the Android SDK/JDK remain explicit host prerequisites.

## Commands

Run the built-in reference at any time:

```bash
cargo crosslab --help
```

Common commands:

```bash
cargo crosslab                         # build all configured apps
cargo crosslab build desktop           # build current-host desktop app
cargo crosslab build android           # build Android arm64 debug APK
cargo crosslab build --development     # build all with M10 development provisioning
cargo crosslab install android         # build + install debug APK on an attached device
cargo crosslab install android --dev   # install M10 development-provisioning build
cargo crosslab run desktop             # build + run current-host desktop app
cargo crosslab run desktop --dev       # run desktop with development provisioning enabled
cargo crosslab doctor                  # verify host prerequisites
cargo crosslab setup                   # install Rust Android target and check Android tooling
```

The Android APK is written to:

```text
apps/android/app/build/outputs/apk/debug/app-debug.apk
```

## Platform behavior

The desktop target is native to the machine running the command. A Linux machine builds the Linux desktop app; a Windows machine invokes the same desktop crate for Windows; a macOS machine invokes it for macOS.

Android host builds currently follow the repository's existing shell/NDK integration and are supported from Linux and macOS. On Windows, the default all-app command builds the Windows desktop target and reports Android as skipped rather than pretending that the unconfigured Android host path is verified.

The builder intentionally does **not** cross-compile all desktop operating systems from one host. Release CI should use native runners per operating system when those platform slices are verified.

At the current M10 checkpoint, Linux desktop + Android are the verified real-platform slice. Windows/macOS desktop application behavior remains outside M10's physical evidence gate.

## Android prerequisites

Current Android configuration requires:

- a Java runtime supported by the pinned Gradle wrapper (the Android source/bytecode target remains Java 17);
- Android platform `android-37.0`;
- Android NDK `28.2.13676358`;
- Rust target `aarch64-linux-android`.

Set `ANDROID_SDK_ROOT` or `ANDROID_HOME` when you want Cross-Lab to use a specific SDK location. Otherwise the builder discovers common Android Studio/system locations and its own user-local SDK location.

If `cargo crosslab build` reports that the Android SDK is not ready, run `cargo crosslab setup` once and follow the license prompt, then retry the build.

`cargo crosslab setup` installs the Rust Android target automatically. If no compatible Android SDK is found, it can bootstrap Cross-Lab's pinned command-line SDK into a user-owned data directory and install the required platform, build-tools, platform-tools, and NDK. The Android SDK license is shown/accepted interactively; Cross-Lab does not accept it silently or install privileged operating-system packages.

## Linux prerequisites

On Ubuntu 24.04/Debian-based Linux, the GitHub CI desktop build installs these system packages. Other Linux distributions need equivalent compiler, Wayland/X11, font, TLS and Vulkan runtime/development packages:

```bash
sudo apt-get update
sudo apt-get install -y --no-install-recommends \
  clang gcc g++ libfontconfig-dev libwayland-dev libx11-xcb-dev \
  libxkbcommon-x11-dev libssl-dev libzstd-dev libvulkan1
```

Install a supported Java 17 runtime for the Android Gradle build. Use the repository's pinned Rust toolchain (`rust-toolchain.toml`) and the project SDK/NDK setup rather than mixing SDK versions.

## Normal Linux + Android product build and test

**Use normal builds for QR pairing, LAN discovery, permissions, clipboard and file transfer.** M10 development provisioning is a separate synthetic-credential test mode. It bypasses the Android `ProductPresencePort` path, so it **cannot** test the actual file-transfer product flow.

From a clean checkout of the intended tested `main` commit, on a Linux workstation:

```bash
cargo crosslab setup
cargo crosslab doctor
cargo crosslab build desktop
cargo crosslab build android
cargo crosslab install android
cargo crosslab run desktop
```

The desktop binary is `target/debug/crosslab-desktop`; the normal Android **arm64-v8a** APK is `apps/android/app/build/outputs/apk/debug/app-debug.apk`. The `install android` command requires an attached, authorized arm64 Android device with USB debugging and `adb` available. You may also build with `cargo crosslab` to build both without installing. Do **not** pass `--development`, `--dev`, or `-PcrosslabDevelopmentProvisioning=true` for these product tests.

After normal pairing on both devices, connect over a reachable local network (avoid AP/client isolation), confirm a trusted active session, and explicitly allow the intended peer's `files.transfer / receive` permission. Default deny remains the correct starting state. Test using disposable files rather than private documents.

Fill in the real-device results in [M10 platform evidence](../research/M10-platform-evidence.md) and [Phase 2 transfer evidence](../research/Phase2-file-transfer-device-evidence.md). Do not paste pairing QR payloads, IP addresses, tokens, real file paths or contents into the evidence record. Code/CI success does not replace actual hardware observations.

## Synthetic M10 lifecycle evidence only

For the older development-provisioned lifecycle and explicit revocation test **only**, use:

```bash
cargo crosslab build --development
cargo crosslab install android --development
```

Follow the secret-safe temporary-credential instructions in `docs/research/M10-platform-evidence.md`. These builds are not valid for normal owner-pairing or file-transfer product testing.
