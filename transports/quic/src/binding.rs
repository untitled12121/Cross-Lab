use crosslab_core::ChannelBinding;
use quinn::Connection;

use crate::QuicTransportError;

const CHANNEL_BINDING_PROFILE: &str = "quic-tls-exporter-v1";
const EXPORTER_LABEL: &[u8] = b"EXPORTER-Cross-Lab-QUIC-Channel-Binding-v1";
const EXPORTER_CONTEXT: &[u8] = b"crosslab.quic.transport.v1";
const CHANNEL_BINDING_BYTES: usize = 32;

pub(crate) fn derive_channel_binding(
    connection: &Connection,
) -> Result<ChannelBinding, QuicTransportError> {
    let mut bytes = [0_u8; CHANNEL_BINDING_BYTES];
    connection
        .export_keying_material(&mut bytes, EXPORTER_LABEL, EXPORTER_CONTEXT)
        .map_err(|_| QuicTransportError::ChannelBinding)?;
    Ok(ChannelBinding::new(CHANNEL_BINDING_PROFILE, bytes.to_vec()))
}
