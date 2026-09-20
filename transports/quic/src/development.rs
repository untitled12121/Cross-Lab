//! Explicit development-only provisioning for the first M10 platform slice.

use std::{fmt, fs, net::SocketAddr, path::Path};

use crosslab_crypto::{SigningKey, VerifyingKey, random_bytes};
use crosslab_identity::{
    AuthorityDelegation, AuthorityRole, DeviceCredential, DeviceId, OwnerAuthorityState, OwnerId,
    OwnerRootRecord,
};
use crosslab_policy::{PairingTrustTransition, TransitionId, TrustRecord, TrustTransition};
use crosslab_protocol::{FeatureSet, ProtocolRange};
use serde::Deserialize;

use crate::{
    AuthenticatedQuicSession, QuicClientEndpoint, QuicClientTlsConfig, QuicServerEndpoint,
    QuicServerTlsConfig, QuicSessionAuthConfig, QuicSessionError, QuicSessionTimeouts,
    QuicTransportConfig,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DevelopmentProvisioningError {
    Read,
    InsecurePermissions,
    Document,
    SecretEncoding,
    Identity,
    Trust,
    Protocol,
    Endpoint,
    WrongRole,
}

impl fmt::Display for DevelopmentProvisioningError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::Read => "development provisioning file could not be read",
            Self::InsecurePermissions => "development provisioning file permissions are too broad",
            Self::Document => "development provisioning document is invalid",
            Self::SecretEncoding => "development provisioning contains invalid encoded material",
            Self::Identity => "development identity provisioning is invalid",
            Self::Trust => "development peer trust provisioning is invalid",
            Self::Protocol => "development protocol provisioning is invalid",
            Self::Endpoint => "development endpoint provisioning is invalid",
            Self::WrongRole => "development provisioning has the wrong endpoint role",
        })
    }
}

impl std::error::Error for DevelopmentProvisioningError {}

pub struct DevelopmentProvisioning {
    authority: OwnerAuthorityState,
    device_signing_key: SigningKey,
    local_signing_key: SigningKey,
    local_credential: DeviceCredential,
    peer_trust: TrustRecord,
    protocol_ranges: Vec<ProtocolRange>,
    features: FeatureSet,
    endpoint: DevelopmentEndpoint,
}

impl DevelopmentProvisioning {
    pub fn from_path(path: impl AsRef<Path>) -> Result<Self, DevelopmentProvisioningError> {
        let path = path.as_ref();
        ensure_private_permissions(path)?;
        let bytes = fs::read(path).map_err(|_| DevelopmentProvisioningError::Read)?;
        Self::from_json(&bytes)
    }

    pub fn from_json(bytes: &[u8]) -> Result<Self, DevelopmentProvisioningError> {
        let file: DevelopmentProvisioningFile =
            serde_json::from_slice(bytes).map_err(|_| DevelopmentProvisioningError::Document)?;

        let owner_id = OwnerId::from_bytes(parse_fixed(&file.identity.owner_id_hex)?);
        let root_key =
            SigningKey::from_secret_bytes(parse_fixed(&file.identity.owner_root_secret_hex)?);
        let root = OwnerRootRecord::new(owner_id, &root_key, file.identity.owner_root_epoch);
        let issuer_key =
            SigningKey::from_secret_bytes(parse_fixed(&file.identity.device_signing_secret_hex)?);
        let delegation = AuthorityDelegation::issue(
            owner_id,
            AuthorityRole::DeviceSigning,
            &issuer_key,
            file.identity.device_signing_epoch,
            &root_key,
        );
        let mut authority = OwnerAuthorityState::new(root);
        authority
            .accept_delegation(delegation)
            .map_err(|_| DevelopmentProvisioningError::Identity)?;

        let local_signing_key =
            SigningKey::from_secret_bytes(parse_fixed(&file.identity.local_device_secret_hex)?);
        let local_credential = DeviceCredential::issue(
            owner_id,
            DeviceId::from_bytes(parse_fixed(&file.identity.local_device_id_hex)?),
            &local_signing_key,
            file.identity.local_credential_epoch,
            &authority,
            &issuer_key,
        )
        .map_err(|_| DevelopmentProvisioningError::Identity)?;

        let peer_public_key =
            VerifyingKey::from_bytes(parse_fixed(&file.identity.peer_device_public_key_hex)?)
                .map_err(|_| DevelopmentProvisioningError::Identity)?;
        let peer_credential = DeviceCredential::issue_for_public_key(
            owner_id,
            DeviceId::from_bytes(parse_fixed(&file.identity.peer_device_id_hex)?),
            peer_public_key,
            file.identity.peer_credential_epoch,
            &authority,
            &issuer_key,
        )
        .map_err(|_| DevelopmentProvisioningError::Identity)?;

        let transition_id =
            TransitionId::generate().map_err(|_| DevelopmentProvisioningError::Trust)?;
        let pairing_nonce =
            random_bytes::<32>().map_err(|_| DevelopmentProvisioningError::Trust)?;
        let peer_trust = PairingTrustTransition::issue(
            &peer_credential,
            transition_id,
            pairing_nonce,
            &authority,
            &issuer_key,
        )
        .map_err(|_| DevelopmentProvisioningError::Trust)?
        .establish(&peer_credential, &authority)
        .map_err(|_| DevelopmentProvisioningError::Trust)?;

        let protocol_ranges = file
            .protocol
            .ranges
            .into_iter()
            .map(|range| ProtocolRange::new(range.major, range.min_minor, range.max_minor))
            .collect::<Result<Vec<_>, _>>()
            .map_err(|_| DevelopmentProvisioningError::Protocol)?;
        if protocol_ranges.is_empty() {
            return Err(DevelopmentProvisioningError::Protocol);
        }
        let features = FeatureSet::new(
            &file.protocol.supported_features,
            &file.protocol.required_features,
        )
        .map_err(|_| DevelopmentProvisioningError::Protocol)?;

        let endpoint = DevelopmentEndpoint::try_from(file.endpoint)?;

        Ok(Self {
            authority,
            device_signing_key: issuer_key,
            local_signing_key,
            local_credential,
            peer_trust,
            protocol_ranges,
            features,
            endpoint,
        })
    }

    pub fn into_client(self) -> Result<DevelopmentQuicClient, DevelopmentProvisioningError> {
        let DevelopmentProvisioning {
            authority,
            device_signing_key,
            local_signing_key,
            local_credential,
            peer_trust,
            protocol_ranges,
            features,
            endpoint,
        } = self;
        let DevelopmentEndpoint::Client {
            bind_addr,
            remote_addr,
            server_name,
            trusted_server_certificates,
        } = endpoint
        else {
            return Err(DevelopmentProvisioningError::WrongRole);
        };
        let endpoint = QuicClientEndpoint::bind(
            bind_addr,
            QuicClientTlsConfig::new(trusted_server_certificates),
            QuicTransportConfig::default(),
        )
        .map_err(|_| DevelopmentProvisioningError::Endpoint)?;

        Ok(DevelopmentQuicClient {
            endpoint,
            remote_addr,
            server_name,
            identity: DevelopmentIdentity {
                authority,
                device_signing_key,
                local_signing_key,
                local_credential,
                peer_trust,
                protocol_ranges,
                features,
            },
        })
    }

    pub fn into_server(self) -> Result<DevelopmentQuicServer, DevelopmentProvisioningError> {
        let DevelopmentProvisioning {
            authority,
            device_signing_key,
            local_signing_key,
            local_credential,
            peer_trust,
            protocol_ranges,
            features,
            endpoint,
        } = self;
        let DevelopmentEndpoint::Server {
            bind_addr,
            certificate_chain,
            private_key_pkcs8,
        } = endpoint
        else {
            return Err(DevelopmentProvisioningError::WrongRole);
        };
        let endpoint = QuicServerEndpoint::bind(
            bind_addr,
            QuicServerTlsConfig::new(certificate_chain, private_key_pkcs8),
            QuicTransportConfig::default(),
        )
        .map_err(|_| DevelopmentProvisioningError::Endpoint)?;

        Ok(DevelopmentQuicServer {
            endpoint,
            identity: DevelopmentIdentity {
                authority,
                device_signing_key,
                local_signing_key,
                local_credential,
                peer_trust,
                protocol_ranges,
                features,
            },
        })
    }
}

struct DevelopmentIdentity {
    authority: OwnerAuthorityState,
    device_signing_key: SigningKey,
    local_signing_key: SigningKey,
    local_credential: DeviceCredential,
    peer_trust: TrustRecord,
    protocol_ranges: Vec<ProtocolRange>,
    features: FeatureSet,
}

impl DevelopmentIdentity {
    fn auth_config(&self) -> QuicSessionAuthConfig<'_> {
        QuicSessionAuthConfig::new(
            &self.authority,
            self.local_credential,
            &self.local_signing_key,
            &self.peer_trust,
            self.protocol_ranges.clone(),
            self.features.clone(),
        )
    }

    fn peer_trust(&self) -> TrustRecord {
        self.peer_trust
    }

    fn revoke_peer_trust(&mut self) -> Result<TrustRecord, DevelopmentProvisioningError> {
        let mut revoked = self.peer_trust;
        let transition_id =
            TransitionId::generate().map_err(|_| DevelopmentProvisioningError::Trust)?;
        let transition = TrustTransition::issue_delegated_revocation(
            &revoked,
            transition_id,
            &self.authority,
            AuthorityRole::DeviceSigning,
            &self.device_signing_key,
        )
        .map_err(|_| DevelopmentProvisioningError::Trust)?;
        transition
            .apply_delegated(&mut revoked, &self.authority)
            .map_err(|_| DevelopmentProvisioningError::Trust)?;
        self.peer_trust = revoked;
        Ok(revoked)
    }
}

pub struct DevelopmentQuicClient {
    endpoint: QuicClientEndpoint,
    remote_addr: SocketAddr,
    server_name: String,
    identity: DevelopmentIdentity,
}

impl DevelopmentQuicClient {
    pub async fn connect_authenticated(
        &self,
        timeouts: QuicSessionTimeouts,
    ) -> Result<AuthenticatedQuicSession, QuicSessionError> {
        let auth = self.identity.auth_config();
        self.endpoint
            .connect_authenticated(self.remote_addr, &self.server_name, &auth, timeouts)
            .await
    }

    pub fn peer_trust(&self) -> TrustRecord {
        self.identity.peer_trust()
    }
}

pub struct DevelopmentQuicServer {
    endpoint: QuicServerEndpoint,
    identity: DevelopmentIdentity,
}

impl DevelopmentQuicServer {
    pub async fn accept_authenticated(
        &self,
        timeouts: QuicSessionTimeouts,
    ) -> Result<AuthenticatedQuicSession, QuicSessionError> {
        let auth = self.identity.auth_config();
        self.endpoint.accept_authenticated(&auth, timeouts).await
    }

    pub fn local_addr(&self) -> Result<SocketAddr, DevelopmentProvisioningError> {
        self.endpoint
            .local_addr()
            .map_err(|_| DevelopmentProvisioningError::Endpoint)
    }

    pub fn peer_trust(&self) -> TrustRecord {
        self.identity.peer_trust()
    }

    pub fn revoke_peer_trust(&mut self) -> Result<TrustRecord, DevelopmentProvisioningError> {
        self.identity.revoke_peer_trust()
    }
}

enum DevelopmentEndpoint {
    Client {
        bind_addr: SocketAddr,
        remote_addr: SocketAddr,
        server_name: String,
        trusted_server_certificates: Vec<Vec<u8>>,
    },
    Server {
        bind_addr: SocketAddr,
        certificate_chain: Vec<Vec<u8>>,
        private_key_pkcs8: Vec<u8>,
    },
}

impl TryFrom<DevelopmentEndpointFile> for DevelopmentEndpoint {
    type Error = DevelopmentProvisioningError;

    fn try_from(file: DevelopmentEndpointFile) -> Result<Self, Self::Error> {
        match file {
            DevelopmentEndpointFile::Client {
                bind_addr,
                remote_addr,
                server_name,
                trusted_server_certificate_der_hex,
            } => {
                if server_name.is_empty() || trusted_server_certificate_der_hex.is_empty() {
                    return Err(DevelopmentProvisioningError::Endpoint);
                }
                Ok(Self::Client {
                    bind_addr: parse_addr(&bind_addr)?,
                    remote_addr: parse_addr(&remote_addr)?,
                    server_name,
                    trusted_server_certificates: trusted_server_certificate_der_hex
                        .iter()
                        .map(|value| parse_hex(value))
                        .collect::<Result<Vec<_>, _>>()?,
                })
            }
            DevelopmentEndpointFile::Server {
                bind_addr,
                certificate_chain_der_hex,
                private_key_pkcs8_der_hex,
            } => {
                if certificate_chain_der_hex.is_empty() {
                    return Err(DevelopmentProvisioningError::Endpoint);
                }
                Ok(Self::Server {
                    bind_addr: parse_addr(&bind_addr)?,
                    certificate_chain: certificate_chain_der_hex
                        .iter()
                        .map(|value| parse_hex(value))
                        .collect::<Result<Vec<_>, _>>()?,
                    private_key_pkcs8: parse_hex(&private_key_pkcs8_der_hex)?,
                })
            }
        }
    }
}

#[derive(Deserialize)]
struct DevelopmentProvisioningFile {
    identity: DevelopmentIdentityFile,
    protocol: DevelopmentProtocolFile,
    endpoint: DevelopmentEndpointFile,
}

#[derive(Deserialize)]
struct DevelopmentIdentityFile {
    owner_id_hex: String,
    owner_root_secret_hex: String,
    owner_root_epoch: u64,
    device_signing_secret_hex: String,
    device_signing_epoch: u64,
    local_device_id_hex: String,
    local_device_secret_hex: String,
    local_credential_epoch: u64,
    peer_device_id_hex: String,
    peer_device_public_key_hex: String,
    peer_credential_epoch: u64,
}

#[derive(Deserialize)]
struct DevelopmentProtocolFile {
    ranges: Vec<DevelopmentProtocolRangeFile>,
    supported_features: Vec<u16>,
    required_features: Vec<u16>,
}

#[derive(Deserialize)]
struct DevelopmentProtocolRangeFile {
    major: u16,
    min_minor: u16,
    max_minor: u16,
}

#[derive(Deserialize)]
#[serde(tag = "role", rename_all = "snake_case")]
enum DevelopmentEndpointFile {
    Client {
        bind_addr: String,
        remote_addr: String,
        server_name: String,
        trusted_server_certificate_der_hex: Vec<String>,
    },
    Server {
        bind_addr: String,
        certificate_chain_der_hex: Vec<String>,
        private_key_pkcs8_der_hex: String,
    },
}

fn parse_addr(value: &str) -> Result<SocketAddr, DevelopmentProvisioningError> {
    value
        .parse()
        .map_err(|_| DevelopmentProvisioningError::Endpoint)
}

fn parse_fixed<const N: usize>(value: &str) -> Result<[u8; N], DevelopmentProvisioningError> {
    let bytes = parse_hex(value)?;
    bytes
        .try_into()
        .map_err(|_| DevelopmentProvisioningError::SecretEncoding)
}

fn parse_hex(value: &str) -> Result<Vec<u8>, DevelopmentProvisioningError> {
    if !value.len().is_multiple_of(2) {
        return Err(DevelopmentProvisioningError::SecretEncoding);
    }

    let mut output = Vec::with_capacity(value.len() / 2);
    let bytes = value.as_bytes();
    for index in (0..bytes.len()).step_by(2) {
        let high = decode_nibble(bytes[index])?;
        let low = decode_nibble(bytes[index + 1])?;
        output.push((high << 4) | low);
    }
    Ok(output)
}

fn decode_nibble(value: u8) -> Result<u8, DevelopmentProvisioningError> {
    match value {
        b'0'..=b'9' => Ok(value - b'0'),
        b'a'..=b'f' => Ok(value - b'a' + 10),
        b'A'..=b'F' => Ok(value - b'A' + 10),
        _ => Err(DevelopmentProvisioningError::SecretEncoding),
    }
}

#[cfg(unix)]
fn ensure_private_permissions(path: &Path) -> Result<(), DevelopmentProvisioningError> {
    use std::os::unix::fs::PermissionsExt as _;

    let metadata = fs::metadata(path).map_err(|_| DevelopmentProvisioningError::Read)?;
    if metadata.permissions().mode() & 0o077 != 0 {
        return Err(DevelopmentProvisioningError::InsecurePermissions);
    }
    Ok(())
}

#[cfg(not(unix))]
fn ensure_private_permissions(path: &Path) -> Result<(), DevelopmentProvisioningError> {
    fs::metadata(path)
        .map(|_| ())
        .map_err(|_| DevelopmentProvisioningError::Read)
}
