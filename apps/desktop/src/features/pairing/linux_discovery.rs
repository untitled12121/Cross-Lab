use core::fmt;

use crosslab_core::{
    PAIRING_DNS_SD_SERVICE_TYPE, PAIRING_DNS_SD_TXT_VERSION_KEY, PAIRING_DNS_SD_TXT_VERSION_V1,
    PairingId, pairing_dns_sd_instance,
};
use zbus::{Connection, Proxy, zvariant::OwnedObjectPath};

const AVAHI_DESTINATION: &str = "org.freedesktop.Avahi";
const AVAHI_SERVER_PATH: &str = "/";
const AVAHI_SERVER_INTERFACE: &str = "org.freedesktop.Avahi.Server";
const AVAHI_ENTRY_GROUP_INTERFACE: &str = "org.freedesktop.Avahi.EntryGroup";
const AVAHI_IF_UNSPEC: i32 = -1;
const AVAHI_PROTO_UNSPEC: i32 = -1;
const AVAHI_FLAGS_NONE: u32 = 0;

pub struct LinuxPairingAdvertisement {
    connection: Connection,
    group_path: OwnedObjectPath,
}

impl LinuxPairingAdvertisement {
    pub async fn start(
        pairing_id: PairingId,
        port: u16,
    ) -> Result<Self, LinuxPairingDiscoveryError> {
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

        let instance = pairing_dns_sd_instance(pairing_id);
        let service_type = avahi_service_type()?;
        let txt = advertisement_txt();
        let _: () = group
            .call(
                "AddService",
                &(
                    AVAHI_IF_UNSPEC,
                    AVAHI_PROTO_UNSPEC,
                    AVAHI_FLAGS_NONE,
                    instance.as_str(),
                    service_type,
                    "",
                    "",
                    port,
                    txt,
                ),
            )
            .await?;
        let _: () = group.call("Commit", &()).await?;

        drop(group);
        Ok(Self {
            connection,
            group_path,
        })
    }

    pub async fn stop(self) -> Result<(), LinuxPairingDiscoveryError> {
        let group = Proxy::new(
            &self.connection,
            AVAHI_DESTINATION,
            self.group_path.clone(),
            AVAHI_ENTRY_GROUP_INTERFACE,
        )
        .await?;
        let _: () = group.call("Free", &()).await?;
        Ok(())
    }
}

#[derive(Debug)]
pub struct LinuxPairingDiscoveryError(LinuxPairingDiscoveryErrorKind);

#[derive(Debug)]
enum LinuxPairingDiscoveryErrorKind {
    Dbus(zbus::Error),
    InvalidServiceType,
}

impl fmt::Display for LinuxPairingDiscoveryError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("local pairing discovery is unavailable")
    }
}

impl std::error::Error for LinuxPairingDiscoveryError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match &self.0 {
            LinuxPairingDiscoveryErrorKind::Dbus(error) => Some(error),
            LinuxPairingDiscoveryErrorKind::InvalidServiceType => None,
        }
    }
}

impl From<zbus::Error> for LinuxPairingDiscoveryError {
    fn from(error: zbus::Error) -> Self {
        Self(LinuxPairingDiscoveryErrorKind::Dbus(error))
    }
}

fn avahi_service_type() -> Result<&'static str, LinuxPairingDiscoveryError> {
    PAIRING_DNS_SD_SERVICE_TYPE
        .strip_suffix(".local.")
        .filter(|value| !value.is_empty())
        .ok_or(LinuxPairingDiscoveryError(
            LinuxPairingDiscoveryErrorKind::InvalidServiceType,
        ))
}

fn advertisement_txt() -> Vec<Vec<u8>> {
    vec![format!("{PAIRING_DNS_SD_TXT_VERSION_KEY}={PAIRING_DNS_SD_TXT_VERSION_V1}").into_bytes()]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn advertisement_txt_contains_only_profile_version() {
        assert_eq!(advertisement_txt(), vec![b"v=1".to_vec()]);
    }

    #[test]
    fn avahi_receives_service_type_without_local_domain() {
        assert_eq!(avahi_service_type().unwrap(), "_crosslab-pair._udp");
    }
}
