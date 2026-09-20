use std::{
    env,
    path::{Path, PathBuf},
    process::Command,
};

use crate::process::{banner, command_available, repo_root, run_command};

const ANDROID_RUST_TARGET: &str = "aarch64-linux-android";
const ANDROID_NDK_VERSION: &str = "28.2.13676358";
const ANDROID_BUILD_TOOLS_VERSION: &str = "36.0.0";
const ANDROID_PLATFORM: &str = "android-37";

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

    ensure_command("java", "-version")
        .map_err(|_| "Java is required for Android builds".to_owned())?;
    ensure_android_rust_target()?;

    if android_sdk_root().is_none() {
        let root = android_sdk_install_root()?;
        bootstrap_android_sdk(&root)?;
    }

    let sdk = android_sdk_root().ok_or_else(|| {
        "Android SDK setup did not produce the required pinned toolchain".to_owned()
    })?;
    verify_android_sdk(&sdk)?;

    println!("crosslab: Android SDK is ready");
    println!("crosslab: Rust/Gradle project dependencies are fetched automatically during build");
    println!("crosslab: setup complete");
    Ok(())
}

pub(crate) fn doctor() -> Result<(), String> {
    banner("doctor");
    let mut failed = false;

    failed |= report_command("cargo", "--version", "Cargo");
    failed |= report_command("rustc", "--version", "Rust");

    if android_host_supported() {
        failed |= report_command("java", "-version", "Java");

        if android_sdk_root().is_some() {
            println!("crosslab: [ok] Android SDK");
        } else {
            failed = true;
            println!("crosslab: [missing] Android SDK toolchain");
            println!("crosslab:           run `cargo crosslab setup` to bootstrap it");
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

pub(crate) fn ensure_android_environment() -> Result<PathBuf, String> {
    ensure_command("java", "-version")
        .map_err(|_| "Java is required for Android builds".to_owned())?;

    android_sdk_root().ok_or_else(|| {
        "Android SDK is not ready; run `cargo crosslab setup` once, then retry".to_owned()
    })
}

pub(crate) fn ensure_android_rust_target() -> Result<(), String> {
    let mut command = Command::new("rustup");
    command.args(["target", "add", ANDROID_RUST_TARGET]);
    run_command(&mut command)
}

fn bootstrap_android_sdk(root: &Path) -> Result<(), String> {
    ensure_command("python3", "--version").map_err(|_| {
        "Python 3 is required to bootstrap the user-local Android SDK".to_owned()
    })?;

    println!("crosslab: Android SDK is missing; starting user-local SDK setup");
    let mut command = Command::new("python3");
    command
        .current_dir(repo_root())
        .arg(repo_root().join("scripts/setup-android-sdk.py"))
        .arg(root);
    run_command(&mut command)
}

fn verify_android_sdk(sdk: &Path) -> Result<(), String> {
    let ndk = sdk.join("ndk").join(ANDROID_NDK_VERSION);
    if !ndk.is_dir() {
        return Err(format!(
            "Android NDK {ANDROID_NDK_VERSION} is missing from the configured SDK"
        ));
    }

    let build_tools = sdk.join("build-tools").join(ANDROID_BUILD_TOOLS_VERSION);
    if !build_tools.is_dir() {
        return Err(format!(
            "Android build-tools {ANDROID_BUILD_TOOLS_VERSION} are missing from the configured SDK"
        ));
    }

    let platform = ["android-37", "android-37.0"]
        .into_iter()
        .map(|version| sdk.join("platforms").join(version).join("android.jar"))
        .find(|path| path.is_file());
    if platform.is_none() {
        return Err(format!(
            "Android platform {ANDROID_PLATFORM} is missing from the configured SDK"
        ));
    }

    Ok(())
}

fn android_sdk_root() -> Option<PathBuf> {
    if let Some(configured) = configured_android_sdk_root() {
        return verify_android_sdk(&configured)
            .is_ok()
            .then_some(configured);
    }

    default_android_sdk_roots()
        .into_iter()
        .find(|path| verify_android_sdk(path).is_ok())
}

fn configured_android_sdk_root() -> Option<PathBuf> {
    env::var_os("ANDROID_SDK_ROOT")
        .or_else(|| env::var_os("ANDROID_HOME"))
        .map(PathBuf::from)
}

fn android_sdk_install_root() -> Result<PathBuf, String> {
    if let Some(configured) = configured_android_sdk_root() {
        return Ok(configured);
    }

    managed_android_sdk_root()
        .ok_or_else(|| "cannot determine a user-local Android SDK directory".to_owned())
}

fn default_android_sdk_roots() -> Vec<PathBuf> {
    let mut roots = Vec::new();

    if let Some(home) = env::var_os("HOME").map(PathBuf::from) {
        match env::consts::OS {
            "linux" => {
                roots.push(home.join("Android/Sdk"));
                roots.push(home.join("Android/sdk"));
            }
            "macos" => roots.push(home.join("Library/Android/sdk")),
            _ => {}
        }
    }

    if env::consts::OS == "linux" {
        roots.push(PathBuf::from("/opt/android-sdk"));
        roots.push(PathBuf::from("/usr/lib/android-sdk"));
    }

    if let Some(managed) = managed_android_sdk_root() {
        roots.push(managed);
    }

    roots
}

fn managed_android_sdk_root() -> Option<PathBuf> {
    let home = env::var_os("HOME").map(PathBuf::from)?;

    match env::consts::OS {
        "linux" => {
            let data = env::var_os("XDG_DATA_HOME")
                .map(PathBuf::from)
                .unwrap_or_else(|| home.join(".local/share"));
            Some(data.join("crosslab/android-sdk"))
        }
        "macos" => Some(home.join("Library/Application Support/Cross-Lab/android-sdk")),
        _ => None,
    }
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn managed_linux_sdk_uses_xdg_data_home_shape() {
        if env::consts::OS != "linux" {
            return;
        }

        let root = managed_android_sdk_root().expect("Linux should have a managed SDK root");
        assert!(root.ends_with("crosslab/android-sdk"));
    }

    #[test]
    fn platform_verification_accepts_standard_android_37_directory() {
        let root = std::env::temp_dir().join(format!(
            "crosslab-xtask-android-sdk-{}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(root.join(format!("ndk/{ANDROID_NDK_VERSION}"))).unwrap();
        std::fs::create_dir_all(root.join(format!(
            "build-tools/{ANDROID_BUILD_TOOLS_VERSION}"
        )))
        .unwrap();
        let platform = root.join("platforms/android-37");
        std::fs::create_dir_all(&platform).unwrap();
        std::fs::write(platform.join("android.jar"), b"fixture").unwrap();

        assert!(verify_android_sdk(&root).is_ok());
        std::fs::remove_dir_all(root).unwrap();
    }
}
