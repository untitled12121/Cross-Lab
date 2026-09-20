#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum App {
    All,
    Desktop,
    Android,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct BuildOptions {
    pub(crate) app: App,
    pub(crate) development: bool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum Action {
    Build(BuildOptions),
    Setup,
    Doctor,
    InstallAndroid { development: bool },
    RunDesktop { development: bool },
    Help,
}

pub(crate) fn parse(args: impl IntoIterator<Item = String>) -> Result<Action, String> {
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
        other => Err(format!(
            "unknown command `{other}`; run `cargo crosslab help`"
        )),
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
            other => return Err(format!("unknown build argument `{other}`")),
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
            other => return Err(format!("unknown install argument `{other}`")),
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
            other => return Err(format!("unknown run argument `{other}`")),
        }
    }

    Ok(Action::RunDesktop { development })
}

fn expect_no_args(
    mut args: impl Iterator<Item = String>,
    action: Action,
) -> Result<Action, String> {
    if let Some(arg) = args.next() {
        return Err(format!("unexpected argument `{arg}`"));
    }
    Ok(action)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn args<'a>(values: &'a [&'a str]) -> impl Iterator<Item = String> + 'a {
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
