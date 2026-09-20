use std::process::Command;

use crate::{
    cli::{App, BuildOptions},
    environment::{
        android_host_supported, ensure_android_environment, ensure_android_rust_target,
    },
    process::{banner, gradle_command, repo_root, run_command},
};

pub(crate) fn build(options: BuildOptions) -> Result<(), String> {
    match options.app {
        App::All => {
            build_desktop(options.development)?;
            if android_host_supported() {
                build_android(options.development)
            } else {
                println!(
                    "crosslab: Android host builds are not configured on {}; skipped",
                    std::env::consts::OS
                );
                Ok(())
            }
        }
        App::Desktop => build_desktop(options.development),
        App::Android => build_android(options.development),
    }
}

fn build_desktop(development: bool) -> Result<(), String> {
    banner("desktop");
    let mut command = Command::new("cargo");
    command
        .current_dir(repo_root())
        .args(["build", "--locked", "-p", "crosslab-desktop"]);
    if development {
        command.args(["--features", "development-provisioning"]);
    }
    run_command(&mut command)
}

fn build_android(development: bool) -> Result<(), String> {
    require_android_host()?;
    banner("android");
    let sdk = ensure_android_environment()?;
    ensure_android_rust_target()?;

    let mut command = gradle_command();
    command
        .current_dir(repo_root().join("apps/android"))
        .env("ANDROID_SDK_ROOT", sdk)
        .args([":app:assembleDebug", "--no-daemon"]);
    if development {
        command.arg("-PcrosslabDevelopmentProvisioning=true");
    }
    run_command(&mut command)?;

    println!(
        "crosslab: Android APK -> {}",
        repo_root()
            .join("apps/android/app/build/outputs/apk/debug/app-debug.apk")
            .display()
    );
    Ok(())
}

pub(crate) fn install_android(development: bool) -> Result<(), String> {
    require_android_host()?;
    banner("android install");
    let sdk = ensure_android_environment()?;
    ensure_android_rust_target()?;

    let mut command = gradle_command();
    command
        .current_dir(repo_root().join("apps/android"))
        .env("ANDROID_SDK_ROOT", sdk)
        .args([":app:installDebug", "--no-daemon"]);
    if development {
        command.arg("-PcrosslabDevelopmentProvisioning=true");
    }
    run_command(&mut command)
}

pub(crate) fn run_desktop(development: bool) -> Result<(), String> {
    banner("desktop run");
    let mut command = Command::new("cargo");
    command
        .current_dir(repo_root())
        .args(["run", "--locked", "-p", "crosslab-desktop"]);
    if development {
        command.args(["--features", "development-provisioning"]);
    }
    run_command(&mut command)
}

fn require_android_host() -> Result<(), String> {
    if android_host_supported() {
        Ok(())
    } else {
        Err(format!(
            "Android host builds are not configured on {}; use a Linux/macOS host or CI",
            std::env::consts::OS
        ))
    }
}
