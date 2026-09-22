pub const SESSION_DNS_SD_SERVICE_TYPE: &str = "_crosslab-session._udp.local.";
pub const SESSION_DNS_SD_TXT_VERSION_KEY: &str = "v";
pub const SESSION_DNS_SD_TXT_VERSION_V1: &str = "1";
pub const SESSION_DNS_SD_INSTANCE_PREFIX: &str = "s-";
pub const SESSION_DNS_SD_INSTANCE_NONCE_LEN: usize = 16;
pub const MAX_SESSION_DISCOVERY_CANDIDATES: usize = 32;

pub fn session_dns_sd_instance(nonce: [u8; SESSION_DNS_SD_INSTANCE_NONCE_LEN]) -> String {
    let mut output = String::with_capacity(
        SESSION_DNS_SD_INSTANCE_PREFIX.len() + SESSION_DNS_SD_INSTANCE_NONCE_LEN * 2,
    );
    output.push_str(SESSION_DNS_SD_INSTANCE_PREFIX);
    for byte in nonce {
        use core::fmt::Write as _;
        write!(&mut output, "{byte:02x}").expect("writing to String cannot fail");
    }
    output
}

pub fn should_initiate_session(local_instance: &str, remote_instance: &str) -> bool {
    is_session_dns_sd_instance(local_instance)
        && is_session_dns_sd_instance(remote_instance)
        && local_instance < remote_instance
}

pub fn is_session_dns_sd_instance(value: &str) -> bool {
    let Some(hex) = value.strip_prefix(SESSION_DNS_SD_INSTANCE_PREFIX) else {
        return false;
    };
    hex.len() == SESSION_DNS_SD_INSTANCE_NONCE_LEN * 2
        && hex
            .bytes()
            .all(|byte| byte.is_ascii_hexdigit() && !byte.is_ascii_uppercase())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn instance_is_ephemeral_lowercase_hex() {
        assert_eq!(
            session_dns_sd_instance([0xab; SESSION_DNS_SD_INSTANCE_NONCE_LEN]),
            "s-abababababababababababababababab"
        );
    }

    #[test]
    fn instance_validation_is_exact() {
        assert!(is_session_dns_sd_instance(
            "s-00112233445566778899aabbccddeeff"
        ));
        assert!(!is_session_dns_sd_instance(
            "s-00112233445566778899AABBCCDDEEFF"
        ));
        assert!(!is_session_dns_sd_instance(
            "p-00112233445566778899aabbccddeeff"
        ));
        assert!(!is_session_dns_sd_instance("s-0011"));
    }

    #[test]
    fn lower_random_instance_is_the_only_initiator() {
        let lower = "s-00112233445566778899aabbccddeeff";
        let higher = "s-10112233445566778899aabbccddeeff";

        assert!(should_initiate_session(lower, higher));
        assert!(!should_initiate_session(higher, lower));
        assert!(!should_initiate_session(lower, lower));
    }
}
