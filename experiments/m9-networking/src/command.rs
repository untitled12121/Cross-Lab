use crate::{config::EvalConfig, error::EvalError};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Command {
    LocalQuinn(EvalConfig),
    LocalIrohDirect(EvalConfig),
    LocalIrohRelay(EvalConfig),
    LocalAll(EvalConfig),
}

impl Command {
    pub fn parse<I, S>(args: I) -> Result<Self, EvalError>
    where
        I: IntoIterator<Item = S>,
        S: AsRef<str>,
    {
        let mut args = args.into_iter();
        let command = args.next().ok_or(EvalError::InvalidCommand)?;
        let config = parse_local_config(&mut args)?;

        match command.as_ref() {
            "local-quinn" => Ok(Self::LocalQuinn(config)),
            "local-iroh-direct" => Ok(Self::LocalIrohDirect(config)),
            "local-iroh-relay" => Ok(Self::LocalIrohRelay(config)),
            "local-all" => Ok(Self::LocalAll(config)),
            _ => Err(EvalError::InvalidCommand),
        }
    }

    pub const fn eval_config(&self) -> Option<EvalConfig> {
        match self {
            Self::LocalQuinn(config)
            | Self::LocalIrohDirect(config)
            | Self::LocalIrohRelay(config)
            | Self::LocalAll(config) => Some(*config),
        }
    }
}

fn parse_local_config<I, S>(args: &mut I) -> Result<EvalConfig, EvalError>
where
    I: Iterator<Item = S>,
    S: AsRef<str>,
{
    let defaults = EvalConfig::default();
    let mut samples = defaults.samples();
    let mut payload_bytes = defaults.bulk_payload_bytes();

    while let Some(argument) = args.next() {
        match argument.as_ref() {
            "--samples" => samples = parse_usize(args.next())?,
            "--payload-bytes" => payload_bytes = parse_usize(args.next())?,
            _ => return Err(EvalError::InvalidArgument),
        }
    }

    EvalConfig::with_limits(samples, payload_bytes)
}

fn parse_usize<S>(value: Option<S>) -> Result<usize, EvalError>
where
    S: AsRef<str>,
{
    value
        .ok_or(EvalError::InvalidArgument)?
        .as_ref()
        .parse()
        .map_err(|_| EvalError::InvalidValue)
}
