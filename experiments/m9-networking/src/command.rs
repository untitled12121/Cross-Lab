use std::{net::SocketAddr, path::PathBuf};

use iroh::RelayUrl;

use crate::{
    config::EvalConfig,
    error::EvalError,
    netprobe::{NetprobePeerArgs, NetprobeRelayArgs},
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Command {
    LocalQuinn(EvalConfig),
    LocalIrohDirect(EvalConfig),
    LocalIrohRelay(EvalConfig),
    LocalAll(EvalConfig),
    NetprobeRelay(NetprobeRelayArgs),
    NetprobeServer(NetprobePeerArgs),
    NetprobeClient(NetprobePeerArgs),
}

impl Command {
    pub fn parse<I, S>(args: I) -> Result<Self, EvalError>
    where
        I: IntoIterator<Item = S>,
        S: AsRef<str>,
    {
        let mut args = args.into_iter();
        let command = args.next().ok_or(EvalError::InvalidCommand)?;

        match command.as_ref() {
            "local-quinn" => Ok(Self::LocalQuinn(parse_local_config(&mut args)?)),
            "local-iroh-direct" => Ok(Self::LocalIrohDirect(parse_local_config(&mut args)?)),
            "local-iroh-relay" => Ok(Self::LocalIrohRelay(parse_local_config(&mut args)?)),
            "local-all" => Ok(Self::LocalAll(parse_local_config(&mut args)?)),
            "netprobe-relay" => Ok(Self::NetprobeRelay(parse_netprobe_relay(&mut args)?)),
            "netprobe-server" => Ok(Self::NetprobeServer(parse_netprobe_peer(&mut args)?)),
            "netprobe-client" => Ok(Self::NetprobeClient(parse_netprobe_peer(&mut args)?)),
            _ => Err(EvalError::InvalidCommand),
        }
    }

    pub const fn eval_config(&self) -> Option<EvalConfig> {
        match self {
            Self::LocalQuinn(config)
            | Self::LocalIrohDirect(config)
            | Self::LocalIrohRelay(config)
            | Self::LocalAll(config) => Some(*config),
            Self::NetprobeRelay(_) | Self::NetprobeServer(_) | Self::NetprobeClient(_) => None,
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

fn parse_netprobe_relay<I, S>(args: &mut I) -> Result<NetprobeRelayArgs, EvalError>
where
    I: Iterator<Item = S>,
    S: AsRef<str>,
{
    expect_flag(args.next(), "--bind")?;
    let bind = parse_value::<SocketAddr, _>(args.next())?;
    reject_extra(args)?;
    Ok(NetprobeRelayArgs::new(bind))
}

fn parse_netprobe_peer<I, S>(args: &mut I) -> Result<NetprobePeerArgs, EvalError>
where
    I: Iterator<Item = S>,
    S: AsRef<str>,
{
    expect_flag(args.next(), "--rendezvous")?;
    let rendezvous = parse_path(args.next())?;
    expect_flag(args.next(), "--relay-url")?;
    let relay_url = parse_value::<RelayUrl, _>(args.next())?;
    reject_extra(args)?;
    Ok(NetprobePeerArgs::new(rendezvous, relay_url))
}

fn expect_flag<S>(value: Option<S>, expected: &str) -> Result<(), EvalError>
where
    S: AsRef<str>,
{
    match value {
        Some(value) if value.as_ref() == expected => Ok(()),
        _ => Err(EvalError::InvalidArgument),
    }
}

fn parse_path<S>(value: Option<S>) -> Result<PathBuf, EvalError>
where
    S: AsRef<str>,
{
    let value = value.ok_or(EvalError::InvalidArgument)?;
    if value.as_ref().is_empty() {
        return Err(EvalError::InvalidValue);
    }
    Ok(PathBuf::from(value.as_ref()))
}

fn parse_value<T, S>(value: Option<S>) -> Result<T, EvalError>
where
    T: std::str::FromStr,
    S: AsRef<str>,
{
    value
        .ok_or(EvalError::InvalidArgument)?
        .as_ref()
        .parse()
        .map_err(|_| EvalError::InvalidValue)
}

fn parse_usize<S>(value: Option<S>) -> Result<usize, EvalError>
where
    S: AsRef<str>,
{
    parse_value(value)
}

fn reject_extra<I, S>(args: &mut I) -> Result<(), EvalError>
where
    I: Iterator<Item = S>,
{
    if args.next().is_some() {
        Err(EvalError::InvalidArgument)
    } else {
        Ok(())
    }
}
