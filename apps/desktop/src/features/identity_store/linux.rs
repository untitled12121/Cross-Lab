use std::{
    env,
    fs::{self, File, OpenOptions},
    io::{Read as _, Write as _},
    path::{Path, PathBuf},
};

use crosslab_crypto::{Signature, SigningKey, SigningProvider, SigningProviderError, VerifyingKey, random_bytes};
use crosslab_identity_store::{
    IdentityStoreAnchor, IdentityStoreEnvelope, IdentityStoreError, prepare_commit, validate_loaded,
};
use oo7::{Keyring, Secret};

const APP_ATTRIBUTE: (&str, &str) = ("application", "crosslab");
const ANCHOR_KIND: (&str, &str) = ("kind", "identity-currentness-anchor");
const SIGNER_KIND: (&str, &str) = ("kind", "device-signing-key");
const SIGNER_VERSION: (&str, &str) = ("version", "1");
const BUNDLE_MAGIC: &[u8; 5] = b"CLIS\x01";

#[derive(Debug)]
pub struct LinuxIdentityStore {
    path: PathBuf,
}

impl LinuxIdentityStore {
    pub fn from_environment() -> Result<Self, LinuxIdentityStoreError> {
        let base = if let Some(state_home) = env::var_os("XDG_STATE_HOME") {
            PathBuf::from(state_home)
        } else {
            let home = env::var_os("HOME").ok_or(LinuxIdentityStoreError::HomeUnavailable)?;
            PathBuf::from(home).join(".local/state")
        };

        Ok(Self {
            path: base.join("crosslab/identity/store-v1.bin"),
        })
    }

    pub async fn load_payload(&self) -> Result<Option<Vec<u8>>, LinuxIdentityStoreError> {
        let Some(bundle) = self.read_bundle()? else {
            return Ok(None);
        };

        let envelope = IdentityStoreEnvelope::decode(&bundle.envelope)?;
        let anchor = IdentityStoreAnchor::decode(&bundle.anchor)?;
        validate_loaded(&envelope, anchor)?;
        self.verify_anchor(anchor).await?;

        Ok(Some(envelope.payload().to_vec()))
    }

    pub async fn commit_payload(&self, payload: Vec<u8>) -> Result<u64, LinuxIdentityStoreError> {
        let current = self.read_bundle()?;
        let current_envelope = current
            .as_ref()
            .map(|bundle| IdentityStoreEnvelope::decode(&bundle.envelope))
            .transpose()?;
        if let Some(bundle) = current.as_ref() {
            let anchor = IdentityStoreAnchor::decode(&bundle.anchor)?;
            let envelope = current_envelope
                .as_ref()
                .ok_or(LinuxIdentityStoreError::Store(IdentityStoreError::StaleOrMixedState))?;
            validate_loaded(envelope, anchor)?;
            self.verify_anchor(anchor).await?;
        }

        let commit = prepare_commit(current_envelope.as_ref(), payload)?;
        let next_anchor = commit.anchor();
        self.create_anchor(next_anchor).await?;

        let next_bundle = StoreBundle {
            envelope: commit.envelope().encode(),
            anchor: next_anchor.encode().to_vec(),
        };
        if let Err(error) = self.write_bundle(&next_bundle) {
            let _ = self.delete_anchor(next_anchor.revision()).await;
            return Err(error);
        }

        if let Some(previous_revision) = commit.previous_revision() {
            self.delete_anchor(previous_revision).await?;
        }
        self.delete_orphan_anchors(next_anchor.revision()).await?;

        Ok(commit.envelope().revision())
    }

    pub async fn wipe(&self) -> Result<(), LinuxIdentityStoreError> {
        if self.path.exists() {
            fs::remove_file(&self.path)?;
        }
        let keyring = Keyring::new().await?;
        keyring.delete(&[APP_ATTRIBUTE, ANCHOR_KIND]).await?;
        Ok(())
    }

    fn read_bundle(&self) -> Result<Option<StoreBundle>, LinuxIdentityStoreError> {
        let mut file = match File::open(&self.path) {
            Ok(file) => file,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
            Err(error) => return Err(error.into()),
        };

        let mut bytes = Vec::new();
        file.read_to_end(&mut bytes)?;
        Ok(Some(StoreBundle::decode(&bytes)?))
    }

    fn write_bundle(&self, bundle: &StoreBundle) -> Result<(), LinuxIdentityStoreError> {
        let parent = self
            .path
            .parent()
            .ok_or(LinuxIdentityStoreError::InvalidPath)?;
        fs::create_dir_all(parent)?;
        set_private_dir(parent)?;

        let temp = self.path.with_extension("bin.new");
        let mut file = OpenOptions::new()
            .write(true)
            .create_new(true)
            .mode_private()
            .open(&temp)?;
        file.write_all(&bundle.encode())?;
        file.sync_all()?;
        drop(file);

        fs::rename(&temp, &self.path)?;
        set_private_file(&self.path)?;
        File::open(parent)?.sync_all()?;
        Ok(())
    }

    async fn create_anchor(&self, anchor: IdentityStoreAnchor) -> Result<(), LinuxIdentityStoreError> {
        let keyring = Keyring::new().await?;
        let revision = anchor.revision().to_string();
        let attributes = [
            APP_ATTRIBUTE,
            ANCHOR_KIND,
            ("revision", revision.as_str()),
        ];
        keyring
            .create_item(
                "Cross-Lab identity currentness anchor",
                &attributes,
                anchor.protected_digest(),
                true,
            )
            .await?;
        Ok(())
    }

    async fn verify_anchor(&self, anchor: IdentityStoreAnchor) -> Result<(), LinuxIdentityStoreError> {
        let keyring = Keyring::new().await?;
        let revision = anchor.revision().to_string();
        let attributes = [
            APP_ATTRIBUTE,
            ANCHOR_KIND,
            ("revision", revision.as_str()),
        ];
        let items = keyring.search_items(&attributes).await?;
        let item = items.first().ok_or(LinuxIdentityStoreError::CurrentnessMissing)?;
        let secret = item.secret().await?;
        if secret.as_bytes() != anchor.protected_digest() {
            return Err(LinuxIdentityStoreError::CurrentnessMismatch);
        }
        Ok(())
    }

    async fn delete_anchor(&self, revision: u64) -> Result<(), LinuxIdentityStoreError> {
        let keyring = Keyring::new().await?;
        let revision = revision.to_string();
        keyring
            .delete(&[
                APP_ATTRIBUTE,
                ANCHOR_KIND,
                ("revision", revision.as_str()),
            ])
            .await?;
        Ok(())
    }

    async fn delete_orphan_anchors(&self, current_revision: u64) -> Result<(), LinuxIdentityStoreError> {
        let keyring = Keyring::new().await?;
        let items = keyring.search_items(&[APP_ATTRIBUTE, ANCHOR_KIND]).await?;
        for item in items {
            let attributes = item.attributes().await?;
            if attributes
                .get("revision")
                .and_then(|value| value.parse::<u64>().ok())
                .is_some_and(|revision| revision != current_revision)
            {
                item.delete().await?;
            }
        }
        Ok(())
    }
}

pub struct LinuxEd25519Signer {
    key: SigningKey,
}

impl LinuxEd25519Signer {
    pub async fn load_or_create() -> Result<Self, LinuxIdentityStoreError> {
        let keyring = Keyring::new().await?;
        let attributes = [APP_ATTRIBUTE, SIGNER_KIND, SIGNER_VERSION];
        let items = keyring.search_items(&attributes).await?;

        if let Some(item) = items.first() {
            let secret = item.secret().await?;
            if secret.as_bytes().len() != 32 {
                return Err(LinuxIdentityStoreError::SignerMalformed);
            }
            let mut bytes = [0_u8; 32];
            bytes.copy_from_slice(secret.as_bytes());
            return Ok(Self {
                key: SigningKey::from_secret_bytes(bytes),
            });
        }

        let secret = random_bytes::<32>().map_err(|_| LinuxIdentityStoreError::Random)?;
        keyring
            .create_item(
                "Cross-Lab device signing key",
                &attributes,
                Secret::blob(secret),
                false,
            )
            .await?;
        Ok(Self {
            key: SigningKey::from_secret_bytes(secret),
        })
    }

    pub async fn wipe() -> Result<(), LinuxIdentityStoreError> {
        let keyring = Keyring::new().await?;
        keyring
            .delete(&[APP_ATTRIBUTE, SIGNER_KIND, SIGNER_VERSION])
            .await?;
        Ok(())
    }
}

impl SigningProvider for LinuxEd25519Signer {
    fn verifying_key(&self) -> VerifyingKey {
        self.key.verifying_key()
    }

    fn sign_message(&self, message: &[u8]) -> Result<Signature, SigningProviderError> {
        Ok(self.key.sign_message(message))
    }
}

#[derive(Debug)]
pub enum LinuxIdentityStoreError {
    HomeUnavailable,
    InvalidPath,
    CurrentnessMissing,
    CurrentnessMismatch,
    SignerMalformed,
    Random,
    Io(std::io::Error),
    Keyring(oo7::Error),
    Store(IdentityStoreError),
}

impl core::fmt::Display for LinuxIdentityStoreError {
    fn fmt(&self, formatter: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::HomeUnavailable => formatter.write_str("Linux home directory is unavailable"),
            Self::InvalidPath => formatter.write_str("Linux identity-store path is invalid"),
            Self::CurrentnessMissing => formatter.write_str("Linux identity currentness anchor is missing"),
            Self::CurrentnessMismatch => formatter.write_str("Linux identity currentness anchor does not match"),
            Self::SignerMalformed => formatter.write_str("Linux device signing key is malformed"),
            Self::Random => formatter.write_str("Linux identity random generation failed"),
            Self::Io(error) => write!(formatter, "Linux identity-store I/O failed: {error}"),
            Self::Keyring(error) => write!(formatter, "Linux secret service failed: {error}"),
            Self::Store(error) => write!(formatter, "identity-store validation failed: {error}"),
        }
    }
}

impl std::error::Error for LinuxIdentityStoreError {}

impl From<std::io::Error> for LinuxIdentityStoreError {
    fn from(error: std::io::Error) -> Self {
        Self::Io(error)
    }
}

impl From<oo7::Error> for LinuxIdentityStoreError {
    fn from(error: oo7::Error) -> Self {
        Self::Keyring(error)
    }
}

impl From<IdentityStoreError> for LinuxIdentityStoreError {
    fn from(error: IdentityStoreError) -> Self {
        Self::Store(error)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct StoreBundle {
    envelope: Vec<u8>,
    anchor: Vec<u8>,
}

impl StoreBundle {
    fn encode(&self) -> Vec<u8> {
        let mut output = Vec::with_capacity(BUNDLE_MAGIC.len() + 8 + self.envelope.len() + self.anchor.len());
        output.extend_from_slice(BUNDLE_MAGIC);
        push_field(&mut output, &self.envelope);
        push_field(&mut output, &self.anchor);
        output
    }

    fn decode(bytes: &[u8]) -> Result<Self, LinuxIdentityStoreError> {
        if bytes.len() < BUNDLE_MAGIC.len() || &bytes[..BUNDLE_MAGIC.len()] != BUNDLE_MAGIC {
            return Err(LinuxIdentityStoreError::InvalidPath);
        }
        let mut offset = BUNDLE_MAGIC.len();
        let envelope = read_field(bytes, &mut offset)?;
        let anchor = read_field(bytes, &mut offset)?;
        if offset != bytes.len() {
            return Err(LinuxIdentityStoreError::InvalidPath);
        }
        Ok(Self { envelope, anchor })
    }
}

fn push_field(output: &mut Vec<u8>, value: &[u8]) {
    let len = u32::try_from(value.len()).expect("identity-store field length fits u32");
    output.extend_from_slice(&len.to_be_bytes());
    output.extend_from_slice(value);
}

fn read_field(bytes: &[u8], offset: &mut usize) -> Result<Vec<u8>, LinuxIdentityStoreError> {
    let header_end = offset.checked_add(4).ok_or(LinuxIdentityStoreError::InvalidPath)?;
    let len_bytes = bytes
        .get(*offset..header_end)
        .ok_or(LinuxIdentityStoreError::InvalidPath)?;
    let len = u32::from_be_bytes(len_bytes.try_into().expect("four-byte length")) as usize;
    if len > 16 * 1024 * 1024 {
        return Err(LinuxIdentityStoreError::InvalidPath);
    }
    let end = header_end.checked_add(len).ok_or(LinuxIdentityStoreError::InvalidPath)?;
    let value = bytes
        .get(header_end..end)
        .ok_or(LinuxIdentityStoreError::InvalidPath)?
        .to_vec();
    *offset = end;
    Ok(value)
}

#[cfg(unix)]
trait PrivateOpenOptionsExt {
    fn mode_private(&mut self) -> &mut Self;
}

#[cfg(unix)]
impl PrivateOpenOptionsExt for OpenOptions {
    fn mode_private(&mut self) -> &mut Self {
        use std::os::unix::fs::OpenOptionsExt as _;
        self.mode(0o600)
    }
}

#[cfg(not(unix))]
trait PrivateOpenOptionsExt {
    fn mode_private(&mut self) -> &mut Self;
}

#[cfg(not(unix))]
impl PrivateOpenOptionsExt for OpenOptions {
    fn mode_private(&mut self) -> &mut Self {
        self
    }
}

#[cfg(unix)]
fn set_private_dir(path: &Path) -> Result<(), std::io::Error> {
    use std::os::unix::fs::PermissionsExt as _;
    fs::set_permissions(path, fs::Permissions::from_mode(0o700))
}

#[cfg(not(unix))]
fn set_private_dir(_: &Path) -> Result<(), std::io::Error> {
    Ok(())
}

#[cfg(unix)]
fn set_private_file(path: &Path) -> Result<(), std::io::Error> {
    use std::os::unix::fs::PermissionsExt as _;
    fs::set_permissions(path, fs::Permissions::from_mode(0o600))
}

#[cfg(not(unix))]
fn set_private_file(_: &Path) -> Result<(), std::io::Error> {
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bundle_round_trips_and_rejects_trailing_data() {
        let bundle = StoreBundle {
            envelope: vec![1, 2, 3],
            anchor: vec![4, 5],
        };
        let encoded = bundle.encode();
        assert_eq!(StoreBundle::decode(&encoded).unwrap(), bundle);

        let mut trailing = encoded;
        trailing.push(9);
        assert!(StoreBundle::decode(&trailing).is_err());
    }
}
