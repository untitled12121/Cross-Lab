use crosslab_core::{
    MAX_SESSION_DISCOVERY_CANDIDATES, SESSION_DNS_SD_INSTANCE_NONCE_LEN,
    SESSION_DNS_SD_SERVICE_TYPE, SESSION_DNS_SD_TXT_VERSION_KEY, SESSION_DNS_SD_TXT_VERSION_V1,
    is_session_dns_sd_instance, session_dns_sd_instance, should_initiate_session,
};
use crosslab_crypto::random_bytes;

#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct MobileTrustedSessionDiscoveryProfile {
    pub instance: String,
    pub service_type: String,
    pub txt_version_key: String,
    pub txt_version: String,
    pub max_candidates: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, uniffi::Error)]
pub enum MobileTrustedSessionDiscoveryError {
    Random,
}

impl core::fmt::Display for MobileTrustedSessionDiscoveryError {
    fn fmt(&self, formatter: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        formatter.write_str("trusted-session discovery identity generation failed")
    }
}

impl std::error::Error for MobileTrustedSessionDiscoveryError {}

#[uniffi::export]
pub fn new_trusted_session_discovery_profile()
-> Result<MobileTrustedSessionDiscoveryProfile, MobileTrustedSessionDiscoveryError> {
    let nonce = random_bytes::<SESSION_DNS_SD_INSTANCE_NONCE_LEN>()
        .map_err(|_| MobileTrustedSessionDiscoveryError::Random)?;
    Ok(MobileTrustedSessionDiscoveryProfile {
        instance: session_dns_sd_instance(nonce),
        service_type: SESSION_DNS_SD_SERVICE_TYPE.to_owned(),
        txt_version_key: SESSION_DNS_SD_TXT_VERSION_KEY.to_owned(),
        txt_version: SESSION_DNS_SD_TXT_VERSION_V1.to_owned(),
        max_candidates: MAX_SESSION_DISCOVERY_CANDIDATES as u32,
    })
}

#[uniffi::export]
pub fn trusted_session_discovery_instance_valid(instance: String) -> bool {
    is_session_dns_sd_instance(&instance)
}

#[uniffi::export]
pub fn trusted_session_should_initiate(local_instance: String, remote_instance: String) -> bool {
    should_initiate_session(&local_instance, &remote_instance)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generated_profile_contains_only_ephemeral_routing_metadata() {
        let profile = new_trusted_session_discovery_profile().unwrap();

        assert!(is_session_dns_sd_instance(&profile.instance));
        assert_eq!(profile.service_type, "_crosslab-session._udp.local.");
        assert_eq!(profile.txt_version_key, "v");
        assert_eq!(profile.txt_version, "1");
        assert_eq!(profile.instance.len(), 34);
        assert_eq!(profile.max_candidates, 32);
    }
}
