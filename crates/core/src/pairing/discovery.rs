use core::fmt::Write as _;

use super::PairingId;

pub const PAIRING_DNS_SD_SERVICE_TYPE: &str = "_crosslab-pair._udp.local.";
pub const PAIRING_DNS_SD_TXT_VERSION_KEY: &str = "v";
pub const PAIRING_DNS_SD_TXT_VERSION_V1: &str = "1";

pub fn pairing_dns_sd_instance(pairing_id: PairingId) -> String {
    let mut output = String::with_capacity(34);
    output.push_str("p-");
    for byte in pairing_id.as_bytes() {
        write!(&mut output, "{byte:02x}").expect("writing to String cannot fail");
    }
    output
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn instance_name_uses_only_pairing_id() {
        assert_eq!(
            pairing_dns_sd_instance(PairingId::from_bytes([0xab; 16])),
            "p-abababababababababababababababab"
        );
    }
}
