use std::process::Command;

use crate::{
    cli::{App, BuildOptions},
    environment::{ensure_android_environment, ensure_android_rust_target},
    process::{banner, gradle_command, repo_root, run_command},
};

pub(crate) fn build(options: BuildOptions) -> Result<(), String> {
    match options.app {
        App::All => {
            build_desktop(options.development)?;
            build_android(options.development)
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
    banner("android");
    ensure_android_environment()?;
    ensure_android_rust_target()?;

    let mut command = gradle_command();
    command
        .current_dir(repo_root().join("apps/android"))
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
    banner("android install");
    ensure_android_environment()?;
    ensure_android_rust_target()?;

    let mut command = gradle_command();
    command
        .current_dir(repo_root().join("apps/android"))
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
