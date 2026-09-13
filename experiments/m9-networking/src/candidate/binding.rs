use crosslab_core::ChannelBinding;
use iroh::endpoint::Connection;

use crate::error::EvalError;

const PROFILE_ID: &str = "quic-tls-exporter-v1";
const OUTPUT_LEN: usize = 32;
const LABEL: &[u8] = b"EXPORTER-Cross-Lab-QUIC-Channel-Binding-v1";
const CONTEXT: &[u8] = b"crosslab.quic.transport.v1";

pub fn derive_channel_binding(connection: &Connection) -> Result<ChannelBinding, EvalError> {
    let mut bytes = vec![0; OUTPUT_LEN];
    connection
        .export_keying_material(&mut bytes, LABEL, CONTEXT)
        .map_err(|_| EvalError::ChannelBinding)?;
    Ok(ChannelBinding::new(PROFILE_ID, bytes))
}
