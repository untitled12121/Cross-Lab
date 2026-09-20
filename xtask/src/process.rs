use std::{
    path::{Path, PathBuf},
    process::{Command, ExitStatus},
};

pub(crate) fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("xtask must live directly under the repository root")
        .to_path_buf()
}

pub(crate) fn gradle_command() -> Command {
    let wrapper = repo_root().join("apps/android").join(if cfg!(windows) {
        "gradlew.bat"
    } else {
        "gradlew"
    });

    if cfg!(windows) {
        Command::new(wrapper)
    } else {
        let mut command = Command::new("bash");
        command.arg(wrapper);
        command
    }
}

pub(crate) fn command_available(program: &str, version_arg: &str) -> bool {
    Command::new(program)
        .arg(version_arg)
        .status()
        .is_ok_and(|status| status.success())
}

pub(crate) fn run_command(command: &mut Command) -> Result<(), String> {
    println!("crosslab: > {command:?}");
    let status = command
        .status()
        .map_err(|error| format!("failed to start command: {error}"))?;
    require_success(status)
}

fn require_success(status: ExitStatus) -> Result<(), String> {
    if status.success() {
        Ok(())
    } else {
        Err(format!("command failed with {status}"))
    }
}

pub(crate) fn banner(name: &str) {
    println!();
    println!("== Cross-Lab {name} ==");
}
