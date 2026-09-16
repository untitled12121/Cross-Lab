use std::{env, process::ExitCode};

use crosslab_m9_networking::{Command, baseline, error::EvalError};

#[tokio::main]
async fn main() -> ExitCode {
    match run().await {
        Ok(output) => {
            print!("{output}");
            ExitCode::SUCCESS
        }
        Err(error) => {
            eprintln!("{error}");
            ExitCode::from(2)
        }
    }
}

async fn run() -> Result<String, EvalError> {
    let command = Command::parse(env::args().skip(1))?;
    let report = baseline::run_local(command).await?;
    Ok(report.to_tsv())
}
