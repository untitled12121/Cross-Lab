mod apps;
mod cli;
mod environment;
mod process;

use cli::Action;

fn main() {
    if let Err(error) = run(std::env::args().skip(1)) {
        eprintln!("crosslab: {error}");
        std::process::exit(2);
    }
}

fn run(args: impl IntoIterator<Item = String>) -> Result<(), String> {
    match cli::parse(args)? {
        Action::Build(options) => apps::build(options),
        Action::Setup => environment::setup(),
        Action::Doctor => environment::doctor(),
        Action::InstallAndroid { development } => apps::install_android(development),
        Action::RunDesktop { development } => apps::run_desktop(development),
        Action::Help => {
            print_help();
            Ok(())
        }
    }
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
