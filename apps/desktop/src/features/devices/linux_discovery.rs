use core::fmt;
use std::{
    collections::{HashMap, HashSet},
    net::{IpAddr, SocketAddr, SocketAddrV6},
};

use crosslab_core::{
    MAX_SESSION_DISCOVERY_CANDIDATES, SESSION_DNS_SD_SERVICE_TYPE, SESSION_DNS_SD_TXT_VERSION_KEY,
    SESSION_DNS_SD_TXT_VERSION_V1, is_session_dns_sd_instance, should_initiate_session,
};
use futures_util::StreamExt as _;
use tokio::{
    sync::{mpsc, watch},
    task::JoinHandle,
};
use zbus::{Connection, Proxy, zvariant::OwnedObjectPath};

const AVAHI_DESTINATION: &str = "org.freedesktop.Avahi";
const AVAHI_SERVER_PATH: &str = "/";
const AVAHI_SERVER_INTERFACE: &str = "org.freedesktop.Avahi.Server";
const AVAHI_ENTRY_GROUP_INTERFACE: &str = "org.freedesktop.Avahi.EntryGroup";
const AVAHI_SERVICE_BROWSER_INTERFACE: &str = "org.freedesktop.Avahi.ServiceBrowser";
const AVAHI_IF_UNSPEC: i32 = -1;
const AVAHI_PROTO_UNSPEC: i32 = -1;
const AVAHI_FLAGS_NONE: u32 = 0;
const EVENT_CAPACITY: usize = MAX_SESSION_DISCOVERY_CANDIDATES * 2;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LinuxTrustedSessionRoute {
    instance: String,
    address: SocketAddr,
    should_initiate: bool,
}

impl LinuxTrustedSessionRoute {
    pub fn instance(&self) -> &str {
        &self.instance
    }

    pub const fn address(&self) -> SocketAddr {
        self.address
    }

    pub const fn should_initiate(&self) -> bool {
        self.should_initiate
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LinuxTrustedSessionDiscoveryEvent {
    Candidate(LinuxTrustedSessionRoute),
    Lost(String),
    Failed,
}

pub struct LinuxTrustedSessionDiscovery {
    connection: Connection,
    group_path: OwnedObjectPath,
    browser_path: OwnedObjectPath,
    stop_tx: watch::Sender<bool>,
    events: mpsc::Receiver<LinuxTrustedSessionDiscoveryEvent>,
    task: JoinHandle<()>,
}

impl LinuxTrustedSessionDiscovery {
    pub async fn start(
        local_instance: String,
        port: u16,
    ) -> Result<Self, LinuxTrustedSessionDiscoveryError> {
        if !is_session_dns_sd_instance(&local_instance) || port == 0 {
            return Err(LinuxTrustedSessionDiscoveryError::InvalidProfile);
        }

        let service_type = avahi_service_type()?;
        let connection = Connection::system().await?;
        let server = Proxy::new(
            &connection,
            AVAHI_DESTINATION,
            AVAHI_SERVER_PATH,
            AVAHI_SERVER_INTERFACE,
        )
        .await?;

        let group_path: OwnedObjectPath = server.call("EntryGroupNew", &()).await?;
        let group = Proxy::new(
            &connection,
            AVAHI_DESTINATION,
            group_path.clone(),
            AVAHI_ENTRY_GROUP_INTERFACE,
        )
        .await?;
        let txt = advertisement_txt();
        let _: () = group
            .call(
                "AddService",
                &(
                    AVAHI_IF_UNSPEC,
                    AVAHI_PROTO_UNSPEC,
                    AVAHI_FLAGS_NONE,
                    local_instance.as_str(),
                    service_type,
                    "",
                    "",
                    port,
                    txt,
                ),
            )
            .await?;
        let _: () = group.call("Commit", &()).await?;

        let browser_path: OwnedObjectPath = server
            .call(
                "ServiceBrowserNew",
                &(
                    AVAHI_IF_UNSPEC,
                    AVAHI_PROTO_UNSPEC,
                    service_type,
                    "",
                    AVAHI_FLAGS_NONE,
                ),
            )
            .await?;
        let browser = Proxy::new_owned(
            connection.clone(),
            AVAHI_DESTINATION,
            browser_path.clone(),
            AVAHI_SERVICE_BROWSER_INTERFACE,
        )
        .await?;
        let signals = browser.receive_all_signals().await?;
        let server = Proxy::new_owned(
            connection.clone(),
            AVAHI_DESTINATION,
            AVAHI_SERVER_PATH,
            AVAHI_SERVER_INTERFACE,
        )
        .await?;

        let (events_tx, events) = mpsc::channel(EVENT_CAPACITY);
        let (stop_tx, stop_rx) = watch::channel(false);
        let task = tokio::spawn(run_browser(
            local_instance,
            server,
            signals,
            stop_rx,
            events_tx,
        ));

        Ok(Self {
            connection,
            group_path,
            browser_path,
            stop_tx,
            events,
            task,
        })
    }

    pub async fn next_event(&mut self) -> Option<LinuxTrustedSessionDiscoveryEvent> {
        self.events.recv().await
    }

    pub async fn stop(mut self) -> Result<(), LinuxTrustedSessionDiscoveryError> {
        self.stop_tx.send_replace(true);
        let _ = (&mut self.task).await;
        free_object(
            &self.connection,
            self.browser_path.clone(),
            AVAHI_SERVICE_BROWSER_INTERFACE,
        )
        .await?;
        free_object(
            &self.connection,
            self.group_path.clone(),
            AVAHI_ENTRY_GROUP_INTERFACE,
        )
        .await?;
        Ok(())
    }
}

impl Drop for LinuxTrustedSessionDiscovery {
    fn drop(&mut self) {
        self.stop_tx.send_replace(true);
        self.task.abort();
    }
}

#[derive(Debug)]
pub enum LinuxTrustedSessionDiscoveryError {
    Dbus(zbus::Error),
    InvalidProfile,
}

impl fmt::Display for LinuxTrustedSessionDiscoveryError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::Dbus(_) => "trusted-session local discovery is unavailable",
            Self::InvalidProfile => "trusted-session discovery profile is invalid",
        })
    }
}

impl std::error::Error for LinuxTrustedSessionDiscoveryError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Dbus(error) => Some(error),
            Self::InvalidProfile => None,
        }
    }
}

impl From<zbus::Error> for LinuxTrustedSessionDiscoveryError {
    fn from(error: zbus::Error) -> Self {
        Self::Dbus(error)
    }
}

async fn run_browser(
    local_instance: String,
    server: Proxy<'static>,
    mut signals: zbus::proxy::SignalStream<'static>,
    mut stop_rx: watch::Receiver<bool>,
    events_tx: mpsc::Sender<LinuxTrustedSessionDiscoveryEvent>,
) {
    let mut sightings: HashMap<String, HashSet<(i32, i32)>> =
        HashMap::with_capacity(MAX_SESSION_DISCOVERY_CANDIDATES);

    loop {
        tokio::select! {
            changed = stop_rx.changed() => {
                if changed.is_err() || *stop_rx.borrow() {
                    return;
                }
            }
            message = signals.next() => {
                let Some(message) = message else {
                    let _ = events_tx.send(LinuxTrustedSessionDiscoveryEvent::Failed).await;
                    return;
                };
                let member = message
                    .header()
                    .member()
                    .map(|member| member.as_str())
                    .unwrap_or_default();
                match member {
                    "ItemNew" => {
                        let Ok((interface, protocol, name, service_type, domain, _flags)) =
                            message.body().deserialize::<(i32, i32, String, String, String, u32)>()
                        else {
                            continue;
                        };
                        if name == local_instance || !is_session_dns_sd_instance(&name) {
                            continue;
                        }
                        if let Some(existing) = sightings.get_mut(&name) {
                            existing.insert((interface, protocol));
                            continue;
                        }
                        if sightings.len() >= MAX_SESSION_DISCOVERY_CANDIDATES {
                            continue;
                        }
                        let Some(route) = resolve_route(
                            &server,
                            &local_instance,
                            interface,
                            protocol,
                            &name,
                            &service_type,
                            &domain,
                        )
                        .await
                        else {
                            continue;
                        };
                        sightings.insert(name.clone(), HashSet::from([(interface, protocol)]));
                        if events_tx
                            .send(LinuxTrustedSessionDiscoveryEvent::Candidate(route))
                            .await
                            .is_err()
                        {
                            return;
                        }
                    }
                    "ItemRemove" => {
                        let Ok((interface, protocol, name, _service_type, _domain, _flags)) =
                            message.body().deserialize::<(i32, i32, String, String, String, u32)>()
                        else {
                            continue;
                        };
                        let Some(existing) = sightings.get_mut(&name) else {
                            continue;
                        };
                        existing.remove(&(interface, protocol));
                        if existing.is_empty() {
                            sightings.remove(&name);
                            if events_tx
                                .send(LinuxTrustedSessionDiscoveryEvent::Lost(name))
                                .await
                                .is_err()
                            {
                                return;
                            }
                        }
                    }
                    "Failure" => {
                        let _ = events_tx.send(LinuxTrustedSessionDiscoveryEvent::Failed).await;
                        return;
                    }
                    _ => {}
                }
            }
        }
    }
}

async fn resolve_route(
    server: &Proxy<'static>,
    local_instance: &str,
    interface: i32,
    protocol: i32,
    name: &str,
    service_type: &str,
    domain: &str,
) -> Option<LinuxTrustedSessionRoute> {
    type ResolveReply = (
        i32,
        i32,
        String,
        String,
        String,
        String,
        i32,
        String,
        u16,
        Vec<Vec<u8>>,
        u32,
    );

    let reply: ResolveReply = server
        .call(
            "ResolveService",
            &(
                interface,
                protocol,
                name,
                service_type,
                domain,
                AVAHI_PROTO_UNSPEC,
                AVAHI_FLAGS_NONE,
            ),
        )
        .await
        .ok()?;
    if !exact_txt_profile(&reply.9) || reply.8 == 0 {
        return None;
    }

    let ip = reply.7.parse::<IpAddr>().ok()?;
    let address = match ip {
        IpAddr::V4(ip) => SocketAddr::new(IpAddr::V4(ip), reply.8),
        IpAddr::V6(ip) => {
            let scope_id = if ip.is_unicast_link_local() {
                u32::try_from(reply.0).ok()?
            } else {
                0
            };
            SocketAddr::V6(SocketAddrV6::new(ip, reply.8, 0, scope_id))
        }
    };
    Some(LinuxTrustedSessionRoute {
        instance: name.to_owned(),
        address,
        should_initiate: should_initiate_session(local_instance, name),
    })
}

async fn free_object(
    connection: &Connection,
    path: OwnedObjectPath,
    interface: &'static str,
) -> Result<(), zbus::Error> {
    let proxy = Proxy::new(connection, AVAHI_DESTINATION, path, interface).await?;
    let _: () = proxy.call("Free", &()).await?;
    Ok(())
}

fn avahi_service_type() -> Result<&'static str, LinuxTrustedSessionDiscoveryError> {
    SESSION_DNS_SD_SERVICE_TYPE
        .strip_suffix(".local.")
        .filter(|value| !value.is_empty())
        .ok_or(LinuxTrustedSessionDiscoveryError::InvalidProfile)
}

fn advertisement_txt() -> Vec<Vec<u8>> {
    vec![format!("{SESSION_DNS_SD_TXT_VERSION_KEY}={SESSION_DNS_SD_TXT_VERSION_V1}").into_bytes()]
}

fn exact_txt_profile(txt: &[Vec<u8>]) -> bool {
    txt.len() == 1 && txt[0] == advertisement_txt()[0]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn avahi_type_drops_only_local_domain() {
        assert_eq!(avahi_service_type().unwrap(), "_crosslab-session._udp");
    }

    #[test]
    fn txt_profile_is_exact() {
        assert!(exact_txt_profile(&[b"v=1".to_vec()]));
        assert!(!exact_txt_profile(&[]));
        assert!(!exact_txt_profile(&[b"v=2".to_vec()]));
        assert!(!exact_txt_profile(&[
            b"v=1".to_vec(),
            b"device=hidden".to_vec(),
        ]));
    }
}
