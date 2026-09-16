use std::{env, process::ExitCode};

use crosslab_m9_networking::{Command, baseline, error::EvalError, netprobe};

#[tokio::main]
async fn main() -> ExitCode {
    match run().await {
        Ok(Some(output)) => {
            print!("{output}");
            ExitCode::SUCCESS
        }
        Ok(None) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("{error}");
            ExitCode::from(2)
        }
    }
}

async fn run() -> Result<Option<String>, EvalError> {
    match Command::parse(env::args().skip(1))? {
        command @ (Command::LocalQuinn(_)
        | Command::LocalIrohDirect(_)
        | Command::LocalIrohRelay(_)
        | Command::LocalAll(_)) => {
            let report = baseline::run_local(command).await?;
            Ok(Some(report.to_tsv()))
        }
        Command::NetprobeRelay(args) => {
            netprobe::run_relay(args).await?;
            Ok(None)
        }
        Command::NetprobeServer(args) => {
            netprobe::run_server(args).await?;
            Ok(None)
        }
        Command::NetprobeClient(args) => {
            netprobe::run_client_streaming(args).await?;
            Ok(None)
        }
    }
}
