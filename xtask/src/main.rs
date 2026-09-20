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
Cross-Lab developer builder

USAGE:
  cargo crosslab [COMMAND]

COMMANDS:
  build [all|desktop|android] [--development]
      Build applications configured for this host.
      With no target, `all` is used.

  setup
      Prepare/check local build tooling and install the Rust Android target.
      Cargo and Gradle fetch project dependencies during builds.

  doctor
      Report missing local prerequisites without changing the machine.

  install android [--development]
      Build and install the Android debug app on an attached device.

  run desktop [--development]
      Build and run the native desktop app for the current host.

  help
      Show this help.

FLAGS:
  --development, --dev
      Enable the existing M10 development-provisioning build path.

  -h, --help
      Show this help.

DEFAULT:
  `cargo crosslab` is equivalent to `cargo crosslab build all`.

HOST BEHAVIOR:
  Linux/macOS  Native desktop + Android arm64 builds are configured.
  Windows      Native desktop build is configured; Android host build is skipped.

EXAMPLES:
  cargo crosslab
  cargo crosslab build desktop
  cargo crosslab build android
  cargo crosslab build --development
  cargo crosslab doctor
  cargo crosslab setup
  cargo crosslab install android --development
  cargo crosslab run desktop --development

NOTES:
  Desktop builds are native to the current OS. The builder does not cross-compile
  Linux, Windows, and macOS desktop binaries from a single host."
    );
}
