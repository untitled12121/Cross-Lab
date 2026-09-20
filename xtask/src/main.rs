use std::{
    env,
    path::{Path, PathBuf},
    process::{Command, ExitStatus},
};

const ANDROID_RUST_TARGET: &str = "aarch64-linux-android";
const ANDROID_NDK_VERSION: &str = "28.2.13676358";
const ANDROID_PLATFORM: &str = "android-37.0";

fn main() {
    if let Err(error) = run(env::args().skip(1)) {
        eprintln!("crosslab: ${error}");
        std::process::exit(2);
    }
}

fn run(args: impl IntoIterator<Item = String>) -> Result<(), String> {
    match parse(args)? {
        Action::Build(options) => build(options),
        Action::Setup => setup(),
        Action::Doctor => doctor(),
        Action::InstallAndroid { development } => install_android(development),
        Action::RunDesktop { development } => run_desktop(development),
        Action::Help => {
            print_help();
            Ok(())
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum App {
    All,
    Desktop,
    Android,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct BuildOptions {
    app: App,
    development: bool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Action {
    Build(BuildOptions),
    Setup,
    Doctor,
    InstallAndroid { development: bool },
    RunDesktop { development: bool },
    Help,
}

fn parse(args: impl IntoIterator<Item = String>) -> Result<Action, String> {
    let mut args = args.into_iter();
    let Some(command) = args.next() else {
        return Ok(Action::Build(BuildOptions {
            app: App::All,
            development: false,
        }));
    };

    match command.as_str() {
        "build" => parse_build(args),
        "setup" => expect_no_args(args, Action::Setup),
        "doctor" => expect_no_args(args, Action::Doctor),
        "install" => parse_install(args),
        "run" => parse_run(args),
        "help" | "--help" | "-h" => Ok(Action::Help),
        other => Err(format!("unknown command `${other}`; run `cargo crosslab help`")),
    }
}

fn parse_build(args: impl Iterator<Item = String>) -> Result<Action, String> {
    let mut app = App::All;
    let mut development = false;

    for arg in args {
        match arg.as_str() {
            "all" => app = App::All,
            "desktop" => app = App::Desktop,
            "android" => app = App::Android,
            "--development" | "--dev" => development = true,
            other => return Err(format!("unknown build argument `${other}`")),
        }
    }

    Ok(Action::Build(BuildOptions { app, development }))
}

fn parse_install(mut args: impl Iterator<Item = String>) -> Result<Action, String> {
    let Some(app) = args.next() else {
        return Err("install requires `android`".into());
    };
    if app != "android" {
        return Err("only Android device installation is configured".into());
    }

    let mut development = false;
    for arg in args {
        match arg.as_str() {
            "--development" | "--dev" => development = true,
            other => return Err(format!("unknown install argument `${other}`")),
        }
    }

    Ok(Action::InstallAndroid { development })
}

fn parse_run(mut args: impl Iterator<Item = String>) -> Result<Action, String> {
    let Some(app) = args.next() else {
        return Err("run requires `desktop`".into());
    };
    if app != "desktop" {
        return Err("only the current-host desktop app can be run directly".into());
    }

    let mut development = false;
    for arg in args {
        match arg.as_str() {
            "--development" | "--dev" => development = true,
            other => return Err(format!("unknown run argument `${other}`")),
        }
    }

    Ok(Action::RunDesktop { development })
}

fn expect_no_args(
    mut args: impl Iterator<Item = String>,
    action: Action,
) -> Result<Action, String> {
    if let Some(arg) = args.next() {
        return Err(format!("unexpected argument `${arg}`"));
    }
    Ok(action)
}

fn build(options: BuildOptions) -> Result<(), String> {
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

fn install_android(development: bool) -> Result<(), String> {
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

fn run_desktop(development: bool) -> Result<(), String> {
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

fn setup() -> Result<(), String> {
    banner("setup");
    ensure_command("rustup", "--version")?;
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

fn doctor() -> Result<(), String> {
    banner("doctor");
    let mut failed = false;

    failed |= report_command("cargo", "--version", "Cargo");
    failed |= report_command("rustc", "--version", "Rust");
    failed |= report_command("java", "-version", "Java 17");

    match android_sdk_root() {
        Some(sdk) => match verify_android_sdk(&sdk) {
            Ok(()) => println!("crosslab: [ok] Android SDK {}", sdk.display()),
            Err(error) => {
                failed = true;
                println!("crosslab: [missing] ${error}");
            }
        },
        None => {
            failed = true;
            println!("crosslab: [missing] ANDROID_SDK_ROOT / ANDROID_HOME");
        }
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

fn ensure_android_environment() -> Result<(), String> {
    ensure_command("java", "-version")
        .map_err(|_| "Java 17 is required for Android builds".to_owned())?;
    let sdk = android_sdk_root().ok_or_else(|| {
        "Android SDK not found; install Android Studio/SDK and set ANDROID_SDK_ROOT".to_owned()
    })?;
    verify_android_sdk(&sdk)
}

fn verify_android_sdk(sdk: &Path) -> Result<(), String> {
    let ndk = sdk.join("ndk").join(ANDROID_NDK_VERSION);
    if !ndk.is_dir() {
        return Err(format!(
            "Android NDK ${ANDROID_NDK_VERSION} is missing under {}",
            sdk.display()
        ));
    }

    let platform = sdk.join("platforms").join(ANDROID_PLATFORM).join("android.jar");
    if !platform.is_file() {
        return Err(format!(
            "Android platform ${ANDROID_PLATFORM} is missing under {}",
            sdk.display()
        ));
    }

    Ok(())
}

fn ensure_android_rust_target() -> Result<(), String> {
    let mut command = Command::new("rustup");
    command.args(["target", "add", ANDROID_RUST_TARGET]);
    run_command(&mut command)
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

fn gradle_command() -> Command {
    if cfg!(windows) {
        Command::new("gradlew.bat")
    } else {
        Command::new("./gradlew")
    }
}

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("xtask must live directly under the repository root")
        .to_path_buf()
}

fn ensure_command(program: &str, version_arg: &str) -> Result<(), String> {
    if command_available(program, version_arg) {
        Ok(())
    } else {
        Err(format!("required command `${program}` is not available"))
    }
}

fn command_available(program: &str, version_arg: &str) -> bool {
    Command::new(program)
        .arg(version_arg)
        .status()
        .is_ok_and(|status| status.success())
}

fn report_command(program: &str, version_arg: &str, label: &str) -> bool {
    if command_available(program, version_arg) {
        println!("crosslab: [ok] ${label}");
        false
    } else {
        println!("crosslab: [missing] ${label}");
        true
    }
}

fn run_command(command: &mut Command) -> Result<(), String> {
    println!("crosslab: > ${command:?}");
    let status = command
        .status()
        .map_err(|error| format!("failed to start command: ${error}"))?;
    require_success(status)
}

fn require_success(status: ExitStatus) -> Result<(), String> {
    if status.success() {
        Ok(())
    } else {
        Err(format!("command failed with ${status}"))
    }
}

fn banner(name: &str) {
    println!();
    println!("== Cross-Lab ${name} ==");
}

fn print_help() {
    println!(
        "\
Cross-Lab build orchestrator

USAGE:
  cargo crosslab
  cargo crosslab build [all|desktop|android] [--development]
  cargo crosslab setup
  cargo crosslab doctor
  cargo crosslab install android [--development]
  cargo crosslab run desktop [--development]

DEFAULT:
  `cargo crosslab` builds every currently configured application for this host:
  the native desktop app plus the Android arm64 debug APK.

NOTES:
  Cargo and Gradle fetch project dependencies automatically.
  `setup` installs the Rust Android target and checks external Android tooling.
  Desktop builds are native to the current OS; this command does not cross-compile
  Linux, Windows, or macOS desktop binaries from another host."
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    fn args(values: &[&str]) -> impl Iterator<Item = String> + '_ {
        values.iter().map(|value| (*value).to_owned())
    }

    #[test]
    fn no_args_builds_all_apps() {
        assert_eq!(
            parse(args(&[])).unwrap(),
            Action::Build(BuildOptions {
                app: App::All,
                development: false,
            })
        );
    }

    #[test]
    fn parses_android_development_build() {
        assert_eq!(
            parse(args(&["build", "android", "--development"])).unwrap(),
            Action::Build(BuildOptions {
                app: App::Android,
                development: true,
            })
        );
    }

    #[test]
    fn rejects_unknown_build_argument() {
        assert!(parse(args(&["build", "ios"])).is_err());
    }

    #[test]
    fn parses_android_install() {
        assert_eq!(
            parse(args(&["install", "android", "--dev"])).unwrap(),
            Action::InstallAndroid { development: true }
        );
    }
}
