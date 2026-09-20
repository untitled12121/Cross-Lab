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

## M10 physical-device build

For the physical Linux + Android evidence build:

```bash
cargo crosslab build --development
cargo crosslab install android --development
```

The existing secret-safe provisioning procedure remains in `docs/research/M10-platform-evidence.md`.
