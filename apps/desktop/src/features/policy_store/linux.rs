use std::{
    env,
    fs::{self, File, OpenOptions},
    io::{Read as _, Write as _},
    path::{Path, PathBuf},
};

use crosslab_policy::PolicyState;
use crosslab_policy_store::{
    MAX_POLICY_SNAPSHOT_BYTES, PolicyStoreAnchor, PolicyStoreEnvelope, PolicyStoreError,
    prepare_commit, validate_loaded,
};
use oo7::Keyring;

const APP_ATTRIBUTE: (&str, &str) = ("application", "crosslab");
const ANCHOR_KIND: (&str, &str) = ("kind", "policy-currentness-anchor");
const BUNDLE_MAGIC: &[u8; 5] = b"CLPS\x01";
const MAX_BUNDLE_FIELD_SIZE: usize = MAX_POLICY_SNAPSHOT_BYTES + 512;

#[derive(Debug)]
pub struct LinuxPolicyStore {
    path: PathBuf,
}

impl LinuxPolicyStore {
    pub fn from_environment() -> Result<Self, LinuxPolicyStoreError> {
        let base = if let Some(state_home) = env::var_os("XDG_STATE_HOME") {
            PathBuf::from(state_home)
        } else {
            let home = env::var_os("HOME").ok_or(LinuxPolicyStoreError::HomeUnavailable)?;
            PathBuf::from(home).join(".local/state")
        };

        Ok(Self {
            path: base.join("crosslab/policy/store-v1.bin"),
        })
    }

    pub async fn load(&self) -> Result<PolicyState, LinuxPolicyStoreError> {
        Ok(self
            .load_current()
            .await?
            .map(|current| current.policy)
            .unwrap_or_default())
    }

    pub async fn commit(
        &self,
        expected_revision: u64,
        policy: &PolicyState,
    ) -> Result<u64, LinuxPolicyStoreError> {
        let current = self.load_current().await?;
        let current_revision = current
            .as_ref()
            .map(|current| current.policy.revision())
            .unwrap_or(0);
        if current_revision != expected_revision {
            return Err(PolicyStoreError::RevisionConflict.into());
        }

        let commit = prepare_commit(
            current.as_ref().map(|current| &current.envelope),
            policy,
        )?;
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

    async fn load_current(&self) -> Result<Option<CurrentPolicy>, LinuxPolicyStoreError> {
        let Some(bundle) = self.read_bundle()? else {
            if self.has_any_anchor().await? {
                return Err(LinuxPolicyStoreError::StateMissing);
            }
            return Ok(None);
        };

        let envelope = PolicyStoreEnvelope::decode(&bundle.envelope)?;
        let anchor = PolicyStoreAnchor::decode(&bundle.anchor)?;
        let policy = validate_loaded(&envelope, anchor)?;
        self.verify_anchor(anchor).await?;
        Ok(Some(CurrentPolicy { envelope, policy }))
    }

    fn read_bundle(&self) -> Result<Option<StoreBundle>, LinuxPolicyStoreError> {
        let mut file = match File::open(&self.path) {
            Ok(file) => file,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
            Err(error) => return Err(error.into()),
        };
        let mut bytes = Vec::new();
        file.read_to_end(&mut bytes)?;
        Ok(Some(StoreBundle::decode(&bytes)?))
    }

    fn write_bundle(&self, bundle: &StoreBundle) -> Result<(), LinuxPolicyStoreError> {
        let parent = self
            .path
            .parent()
            .ok_or(LinuxPolicyStoreError::InvalidPath)?;
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

    async fn has_any_anchor(&self) -> Result<bool, LinuxPolicyStoreError> {
        let keyring = Keyring::new().await?;
        Ok(!keyring
            .search_items(&[APP_ATTRIBUTE, ANCHOR_KIND])
            .await?
            .is_empty())
    }

    async fn create_anchor(
        &self,
        anchor: PolicyStoreAnchor,
    ) -> Result<(), LinuxPolicyStoreError> {
        let keyring = Keyring::new().await?;
        let revision = anchor.revision().to_string();
        let attributes = [APP_ATTRIBUTE, ANCHOR_KIND, ("revision", revision.as_str())];
        let protected_digest = anchor.protected_digest();
        keyring
            .create_item(
                "Cross-Lab policy currentness anchor",
                &attributes,
                &protected_digest,
                true,
            )
            .await?;
        Ok(())
    }

    async fn verify_anchor(
        &self,
        anchor: PolicyStoreAnchor,
    ) -> Result<(), LinuxPolicyStoreError> {
        let keyring = Keyring::new().await?;
        let revision = anchor.revision().to_string();
        let attributes = [APP_ATTRIBUTE, ANCHOR_KIND, ("revision", revision.as_str())];
        let items = keyring.search_items(&attributes).await?;
        let item = items
            .first()
            .ok_or(LinuxPolicyStoreError::CurrentnessMissing)?;
        let secret = item.secret().await?;
        if secret.as_bytes() != anchor.protected_digest() {
            return Err(LinuxPolicyStoreError::CurrentnessMismatch);
        }
        Ok(())
    }

    async fn delete_anchor(&self, revision: u64) -> Result<(), LinuxPolicyStoreError> {
        let keyring = Keyring::new().await?;
        let revision = revision.to_string();
        keyring
            .delete(&[APP_ATTRIBUTE, ANCHOR_KIND, ("revision", revision.as_str())])
            .await?;
        Ok(())
    }

    async fn delete_orphan_anchors(
        &self,
        current_revision: u64,
    ) -> Result<(), LinuxPolicyStoreError> {
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

struct CurrentPolicy {
    envelope: PolicyStoreEnvelope,
    policy: PolicyState,
}

#[derive(Debug)]
pub enum LinuxPolicyStoreError {
    HomeUnavailable,
    InvalidPath,
    StateMissing,
    CurrentnessMissing,
    CurrentnessMismatch,
    Io(std::io::Error),
    Keyring(Box<oo7::Error>),
    Store(PolicyStoreError),
}

impl core::fmt::Display for LinuxPolicyStoreError {
    fn fmt(&self, formatter: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::HomeUnavailable => formatter.write_str("Linux home directory is unavailable"),
            Self::InvalidPath => formatter.write_str("Linux policy-store path is invalid"),
            Self::StateMissing => formatter.write_str("Linux committed policy state is missing"),
            Self::CurrentnessMissing => {
                formatter.write_str("Linux policy currentness anchor is missing")
            }
            Self::CurrentnessMismatch => {
                formatter.write_str("Linux policy currentness anchor does not match")
            }
            Self::Io(error) => write!(formatter, "Linux policy-store I/O failed: {error}"),
            Self::Keyring(error) => write!(formatter, "Linux secret service failed: {error}"),
            Self::Store(error) => write!(formatter, "policy-store validation failed: {error}"),
        }
    }
}

impl std::error::Error for LinuxPolicyStoreError {}

impl From<std::io::Error> for LinuxPolicyStoreError {
    fn from(error: std::io::Error) -> Self {
        Self::Io(error)
    }
}

impl From<oo7::Error> for LinuxPolicyStoreError {
    fn from(error: oo7::Error) -> Self {
        Self::Keyring(Box::new(error))
    }
}

impl From<PolicyStoreError> for LinuxPolicyStoreError {
    fn from(error: PolicyStoreError) -> Self {
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
        let mut output =
            Vec::with_capacity(BUNDLE_MAGIC.len() + 8 + self.envelope.len() + self.anchor.len());
        output.extend_from_slice(BUNDLE_MAGIC);
        push_field(&mut output, &self.envelope);
        push_field(&mut output, &self.anchor);
        output
    }

    fn decode(bytes: &[u8]) -> Result<Self, LinuxPolicyStoreError> {
        if bytes.len() < BUNDLE_MAGIC.len() || &bytes[..BUNDLE_MAGIC.len()] != BUNDLE_MAGIC {
            return Err(LinuxPolicyStoreError::InvalidPath);
        }
        let mut offset = BUNDLE_MAGIC.len();
        let envelope = read_field(bytes, &mut offset)?;
        let anchor = read_field(bytes, &mut offset)?;
        if offset != bytes.len() {
            return Err(LinuxPolicyStoreError::InvalidPath);
        }
        Ok(Self { envelope, anchor })
    }
}

fn push_field(output: &mut Vec<u8>, value: &[u8]) {
    let len = u32::try_from(value.len()).expect("policy-store field length fits u32");
    output.extend_from_slice(&len.to_be_bytes());
    output.extend_from_slice(value);
}

fn read_field(bytes: &[u8], offset: &mut usize) -> Result<Vec<u8>, LinuxPolicyStoreError> {
    let header_end = offset
        .checked_add(4)
        .ok_or(LinuxPolicyStoreError::InvalidPath)?;
    let len_bytes = bytes
        .get(*offset..header_end)
        .ok_or(LinuxPolicyStoreError::InvalidPath)?;
    let len = u32::from_be_bytes(len_bytes.try_into().expect("four-byte length")) as usize;
    if len > MAX_BUNDLE_FIELD_SIZE {
        return Err(LinuxPolicyStoreError::InvalidPath);
    }
    let end = header_end
        .checked_add(len)
        .ok_or(LinuxPolicyStoreError::InvalidPath)?;
    let value = bytes
        .get(header_end..end)
        .ok_or(LinuxPolicyStoreError::InvalidPath)?
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
