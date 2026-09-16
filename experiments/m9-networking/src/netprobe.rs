use std::{
    net::SocketAddr,
    path::{Path, PathBuf},
};

use iroh::{EndpointAddr, EndpointId, RelayUrl};

use crate::error::EvalError;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NetprobeRelayArgs {
    bind: SocketAddr,
}

impl NetprobeRelayArgs {
    pub const fn new(bind: SocketAddr) -> Self {
        Self { bind }
    }

    pub const fn bind(&self) -> SocketAddr {
        self.bind
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NetprobePeerArgs {
    rendezvous: PathBuf,
    relay_url: RelayUrl,
}

impl NetprobePeerArgs {
    pub fn new(rendezvous: PathBuf, relay_url: RelayUrl) -> Self {
        Self {
            rendezvous,
            relay_url,
        }
    }

    pub fn rendezvous(&self) -> &Path {
        &self.rendezvous
    }

    pub const fn relay_url(&self) -> &RelayUrl {
        &self.relay_url
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Rendezvous {
    endpoint_id: EndpointId,
    relay_url: Option<RelayUrl>,
    ip: Option<SocketAddr>,
}

impl Rendezvous {
    pub fn new(endpoint_id: EndpointId, relay_url: Option<RelayUrl>, ip: Option<SocketAddr>) -> Self {
        Self {
            endpoint_id,
            relay_url,
            ip,
        }
    }

    pub fn parse(text: &str) -> Result<Self, EvalError> {
        let mut lines = text.lines();
        let endpoint_id = parse_required(lines.next(), "endpoint_id")?
            .parse()
            .map_err(|_| EvalError::InvalidValue)?;
        let relay_url = parse_optional(lines.next(), "relay_url")?
            .map(|value| value.parse().map_err(|_| EvalError::InvalidValue))
            .transpose()?;
        let ip = parse_optional(lines.next(), "ip")?
            .map(|value| value.parse().map_err(|_| EvalError::InvalidValue))
            .transpose()?;

        if lines.next().is_some() {
            return Err(EvalError::InvalidValue);
        }

        Ok(Self::new(endpoint_id, relay_url, ip))
    }

    pub fn to_text(&self) -> String {
        let relay_url = self
            .relay_url
            .as_ref()
            .map_or_else(|| "-".to_owned(), ToString::to_string);
        let ip = self.ip.map_or_else(|| "-".to_owned(), |ip| ip.to_string());
        format!(
            "endpoint_id={}\nrelay_url={relay_url}\nip={ip}\n",
            self.endpoint_id
        )
    }

    pub fn endpoint_addr(&self) -> EndpointAddr {
        let mut addr = EndpointAddr::new(self.endpoint_id);
        if let Some(relay_url) = &self.relay_url {
            addr = addr.with_relay_url(relay_url.clone());
        }
        if let Some(ip) = self.ip {
            addr = addr.with_ip_addr(ip);
        }
        addr
    }
}

fn parse_required<'a>(line: Option<&'a str>, key: &str) -> Result<&'a str, EvalError> {
    let line = line.ok_or(EvalError::InvalidValue)?;
    let (actual, value) = line.split_once('=').ok_or(EvalError::InvalidValue)?;
    if actual != key || value.is_empty() {
        return Err(EvalError::InvalidValue);
    }
    Ok(value)
}

fn parse_optional<'a>(line: Option<&'a str>, key: &str) -> Result<Option<&'a str>, EvalError> {
    let value = parse_required(line, key)?;
    Ok((value != "-").then_some(value))
}
