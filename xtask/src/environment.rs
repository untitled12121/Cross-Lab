use std::{
    env,
    path::{Path, PathBuf},
    process::Command,
};

use crate::process::{banner, command_available, run_command};

const ANDROID_RUST_TARGET: &str = "aarch64-linux-android";
const ANDROID_NDK_VERSION: &str = "28.2.13676358";
const ANDROID_PLATFORM: &str = "android-37.0";

pub(crate) fn setup() -> Result<(), String> {
    banner("setup");
    ensure_command("rustup", "--version")?;

    if !android_host_supported() {
        println!(
            "crosslab: Android host builds are not configured on {}; desktop setup is ready",
            env::consts::OS
        );
        return Ok(());
    }

    ensure_android_rust_target()?;

    if let Some(sdk) = android_sdk_root() {
        println!("crosslab: Android SDK -> {}", sdk.display());
        verify_android_sdk(&sdk)?;
    } else {
        println!(
            "crosslab: Android SDK not found; install Android Studio/SDK and set ANDROID_SDK_ROOT"
        );
    }

    if command_available("java", "-version") {
        println!("crosslab: Java detected");
    } else {
        println!("crosslab: Java 17 is required for Android builds");
    }

    println!("crosslab: Rust/Gradle project dependencies are fetched automatically during build");
    println!("crosslab: run `cargo crosslab doctor` to verify the host");
    Ok(())
}

pub(crate) fn doctor() -> Result<(), String> {
    banner("doctor");
    let mut failed = false;

    failed |= report_command("cargo", "--version", "Cargo");
    failed |= report_command("rustc", "--version", "Rust");

    if android_host_supported() {
        failed |= report_command("java", "-version", "Java 17");

        match android_sdk_root() {
        Some(sdk) => match verify_android_sdk(&sdk) {
            Ok(()) => println!("crosslab: [ok] Android SDK {}", sdk.display()),
            Err(error) => {
                failed = true;
                println!("crosslab: [missing] {error}");
            }
        },
            None => {
                failed = true;
                println!("crosslab: [missing] ANDROID_SDK_ROOT / ANDROID_HOME");
            }
        }
    } else {
        println!(
            "crosslab: [skip] Android host build is not configured on {}",
            env::consts::OS
        );
    }

    println!(
        "crosslab: desktop target -> {} ({})",
        env::consts::OS,
        env::consts::ARCH
    );

    if failed {
        Err("host prerequisites are incomplete".into())
    } else {
        println!("crosslab: host prerequisites look ready");
        Ok(())
    }
}

pub(crate) fn android_host_supported() -> bool {
    matches!(env::consts::OS, "linux" | "macos")
}

pub(crate) fn ensure_android_environment() -> Result<(), String> {
    ensure_command("java", "-version")
        .map_err(|_| "Java 17 is required for Android builds".to_owned())?;
    let sdk = android_sdk_root().ok_or_else(|| {
        "Android SDK not found; install Android Studio/SDK and set ANDROID_SDK_ROOT".to_owned()
    })?;
    verify_android_sdk(&sdk)
}

pub(crate) fn ensure_android_rust_target() -> Result<(), String> {
    let mut command = Command::new("rustup");
    command.args(["target", "add", ANDROID_RUST_TARGET]);
    run_command(&mut command)
}

fn verify_android_sdk(sdk: &Path) -> Result<(), String> {
    let ndk = sdk.join("ndk").join(ANDROID_NDK_VERSION);
    if !ndk.is_dir() {
        return Err(format!(
            "Android NDK {ANDROID_NDK_VERSION} is missing under {}",
            sdk.display()
        ));
    }

    let platform = sdk
        .join("platforms")
        .join(ANDROID_PLATFORM)
        .join("android.jar");
    if !platform.is_file() {
        return Err(format!(
            "Android platform {ANDROID_PLATFORM} is missing under {}",
            sdk.display()
        ));
    }

    Ok(())
}

fn android_sdk_root() -> Option<PathBuf> {
    env::var_os("ANDROID_SDK_ROOT")
        .or_else(|| env::var_os("ANDROID_HOME"))
        .map(PathBuf::from)
        .filter(|path| path.is_dir())
        .or_else(default_android_sdk_root)
}

fn default_android_sdk_root() -> Option<PathBuf> {
    let home = env::var_os("HOME").map(PathBuf::from);
    match env::consts::OS {
        "linux" => home.map(|path| path.join("Android/Sdk")),
        "macos" => home.map(|path| path.join("Library/Android/sdk")),
        "windows" => env::var_os("LOCALAPPDATA")
            .map(PathBuf::from)
            .map(|path| path.join("Android/Sdk")),
        _ => None,
    }
    .filter(|path| path.is_dir())
}

fn ensure_command(program: &str, version_arg: &str) -> Result<(), String> {
    if command_available(program, version_arg) {
        Ok(())
    } else {
        Err(format!("required command `{program}` is not available"))
    }
}

fn report_command(program: &str, version_arg: &str, label: &str) -> bool {
    if command_available(program, version_arg) {
        println!("crosslab: [ok] {label}");
        false
    } else {
        println!("crosslab: [missing] {label}");
        true
    }
}
