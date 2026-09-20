use core::fmt;

use crosslab_identity::{DeviceId, OwnerId};
use zeroize::Zeroize;

use super::{PairingId, PairingInvitation, PairingSecret};

pub const PAIRING_BOOTSTRAP_PROFILE_V1: u16 = 1;

const PREFIX: &[u8] = b"crosslab:pair:v1:";
const PAYLOAD_LEN: usize = 2 + 16 + 32 + 32 + 32;
const CODE_LEN: usize = PREFIX.len() + PAYLOAD_LEN * 2;

pub struct PairingBootstrapCode {
    bytes: [u8; CODE_LEN],
}

impl PairingBootstrapCode {
    pub(crate) fn from_invitation(invitation: &PairingInvitation) -> Self {
        let mut payload = [0_u8; PAYLOAD_LEN];
        let mut offset = 0;

        write_field(
            &mut payload,
            &mut offset,
            &PAIRING_BOOTSTRAP_PROFILE_V1.to_be_bytes(),
        );
        write_field(
            &mut payload,
            &mut offset,
            invitation.pairing_id().as_bytes(),
        );
        write_field(
            &mut payload,
            &mut offset,
            invitation.secret().as_bytes(),
        );
        write_field(
            &mut payload,
            &mut offset,
            &invitation.owner_id().to_bytes(),
        );
        write_field(
            &mut payload,
            &mut offset,
            &invitation.inviter_device_id().to_bytes(),
        );

        let mut bytes = [0_u8; CODE_LEN];
        bytes[..PREFIX.len()].copy_from_slice(PREFIX);
        encode_hex(&payload, &mut bytes[PREFIX.len()..]);
        payload.zeroize();

        Self { bytes }
    }

    pub fn as_str(&self) -> &str {
        core::str::from_utf8(&self.bytes).expect("pairing bootstrap code is ASCII")
    }
}

impl fmt::Debug for PairingBootstrapCode {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("PairingBootstrapCode([REDACTED])")
    }
}

impl Drop for PairingBootstrapCode {
    fn drop(&mut self) {
        self.bytes.zeroize();
    }
}

pub struct PairingBootstrap {
    pairing_id: PairingId,
    secret: PairingSecret,
    owner_id: OwnerId,
    inviter_device_id: DeviceId,
}

impl PairingBootstrap {
    pub fn decode(code: &str) -> Result<Self, PairingBootstrapError> {
        let bytes = code.as_bytes();
        if !bytes.starts_with(PREFIX) {
            return Err(PairingBootstrapError::InvalidPrefix);
        }

        let encoded = &bytes[PREFIX.len()..];
        if encoded.len() != PAYLOAD_LEN * 2 {
            return Err(PairingBootstrapError::InvalidLength);
        }

        let mut payload = [0_u8; PAYLOAD_LEN];
        decode_hex(encoded, &mut payload)?;

        let profile = u16::from_be_bytes([payload[0], payload[1]]);
        if profile != PAIRING_BOOTSTRAP_PROFILE_V1 {
            payload.zeroize();
            return Err(PairingBootstrapError::UnsupportedProfile);
        }

        let pairing_id = PairingId::from_bytes(copy_array::<16>(&payload[2..18]));
        let secret = PairingSecret::from_bytes(copy_array::<32>(&payload[18..50]));
        let owner_id = OwnerId::from_bytes(copy_array::<32>(&payload[50..82]));
        let inviter_device_id = DeviceId::from_bytes(copy_array::<32>(&payload[82..114]));
        payload.zeroize();

        Ok(Self {
            pairing_id,
            secret,
            owner_id,
            inviter_device_id,
        })
    }

    pub const fn pairing_id(&self) -> PairingId {
        self.pairing_id
    }

    pub const fn owner_id(&self) -> OwnerId {
        self.owner_id
    }

    pub const fn inviter_device_id(&self) -> DeviceId {
        self.inviter_device_id
    }

    pub fn into_parts(self) -> (PairingId, PairingSecret, OwnerId, DeviceId) {
        (
            self.pairing_id,
            self.secret,
            self.owner_id,
            self.inviter_device_id,
        )
    }
}

impl fmt::Debug for PairingBootstrap {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("PairingBootstrap")
            .field("pairing_id", &self.pairing_id)
            .field("secret", &"[REDACTED]")
            .field("owner_id", &self.owner_id)
            .field("inviter_device_id", &self.inviter_device_id)
            .finish()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PairingBootstrapError {
    InvalidPrefix,
    InvalidLength,
    InvalidEncoding,
    UnsupportedProfile,
}

impl fmt::Display for PairingBootstrapError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::InvalidPrefix => "pairing bootstrap code has an invalid prefix",
            Self::InvalidLength => "pairing bootstrap code has an invalid length",
            Self::InvalidEncoding => "pairing bootstrap code has invalid encoding",
            Self::UnsupportedProfile => "pairing bootstrap profile is unsupported",
        })
    }
}

impl std::error::Error for PairingBootstrapError {}

fn write_field<const N: usize>(payload: &mut [u8], offset: &mut usize, value: &[u8; N]) {
    let end = *offset + N;
    payload[*offset..end].copy_from_slice(value);
    *offset = end;
}

fn encode_hex(input: &[u8], output: &mut [u8]) {
    debug_assert_eq!(output.len(), input.len() * 2);
    const HEX: &[u8; 16] = b"0123456789abcdef";

    for (index, byte) in input.iter().copied().enumerate() {
        output[index * 2] = HEX[usize::from(byte >> 4)];
        output[index * 2 + 1] = HEX[usize::from(byte & 0x0f)];
    }
}

fn decode_hex(input: &[u8], output: &mut [u8]) -> Result<(), PairingBootstrapError> {
    if input.len() != output.len() * 2 {
        return Err(PairingBootstrapError::InvalidLength);
    }

    for (index, output_byte) in output.iter_mut().enumerate() {
        let high = decode_nibble(input[index * 2])?;
        let low = decode_nibble(input[index * 2 + 1])?;
        *output_byte = (high << 4) | low;
    }

    Ok(())
}

fn decode_nibble(value: u8) -> Result<u8, PairingBootstrapError> {
    match value {
        b'0'..=b'9' => Ok(value - b'0'),
        b'a'..=b'f' => Ok(value - b'a' + 10),
        b'A'..=b'F' => Ok(value - b'A' + 10),
        _ => Err(PairingBootstrapError::InvalidEncoding),
    }
}

fn copy_array<const N: usize>(value: &[u8]) -> [u8; N] {
    let mut output = [0_u8; N];
    output.copy_from_slice(value);
    output
}
