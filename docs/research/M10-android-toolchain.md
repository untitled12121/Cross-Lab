# M10 Android toolchain

Verified on 2026-09-18 for M10 Task 8.

## Selected build set

- Android Gradle Plugin: \`9.3.1\`
- Gradle: \`9.5.0\`
- Kotlin Gradle plugin/runtime used by AGP built-in Kotlin: \`2.4.20\`
- Compose compiler plugin: \`2.4.20\`
- Compose BOM: \`2026.09.00\`
- Activity Compose: \`1.13.0\`
- Lifecycle: \`2.11.0\`
- JDK: \`17\`
- compileSdk: \`37\`
- targetSdk: \`36\`
- minSdk: \`23\`
- NDK: \`28.2.13676358\`
- UniFFI JVM bridge: JNA \`5.19.1\`
- M10 native ABI: \`arm64-v8a\`

## Rationale

AGP 9.4.0 is current, but Kotlin 2.4.20 documents full AGP compatibility only through 9.3.1. AGP 9 supports built-in Kotlin and permits upgrading its KGP runtime. Using AGP 9.3.1 with KGP 2.4.20 therefore keeps the Android build inside the documented compatibility window while retaining current Kotlin.

Compose 1.12+ requires compileSdk 37 and AGP 9. The current Compose setup documentation lists BOM 2026.09.00. Google Play requires new apps and updates to target API 36 or higher from 2026-08-31, so targetSdk 36 is retained for this development slice.

The minimum is API 23 because current AndroidX releases have moved their default minimum from API 21 to API 23. The Rust Android targets and the NetworkCallback API can operate lower, while StrongBox begins at API 28 and remains an optional future secure-storage capability rather than a reason to raise the whole application's minimum today.

M10 builds only arm64-v8a because no Android emulator is executed in CI. This is a development evidence ABI, not a final store ABI policy. Add another ABI only when a concrete M10 device/emulator requires it.

## Sources

- Android Gradle Plugin 9.3 release notes: https://developer.android.com/build/releases/agp-9-3-0-release-notes
- Kotlin Gradle compatibility: https://kotlinlang.org/docs/gradle-configure-project.html
- AGP built-in Kotlin: https://developer.android.com/build/migrate-to-built-in-kotlin
- Compose setup/compiler: https://developer.android.com/develop/ui/compose/setup-compose-dependencies-and-compiler
- AndroidX versions: https://developer.android.com/jetpack/androidx/versions
- Google Play target API requirements: https://developer.android.com/google/play/requirements/target-sdk
- Rust Android platform support: https://doc.rust-lang.org/rustc/platform-support/android.html
- Android Keystore: https://developer.android.com/privacy-and-security/keystore
