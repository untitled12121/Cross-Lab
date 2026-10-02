mod lifecycle;

pub(crate) use lifecycle::{AuditIntent, AuditLifecycle};

use std::{
    env,
    fs::{self, File, OpenOptions},
    io::{Read as _, Write as _},
    path::{Path, PathBuf},
    time::{SystemTime, UNIX_EPOCH},
};

use crosslab_core::{AuditAction, AuditError, AuditHistory, AuditOutcome};
use crosslab_identity_store::{
    IdentityStoreAnchor, IdentityStoreEnvelope, IdentityStoreError, prepare_commit, validate_loaded,
};
use oo7::Keyring;

const APP: (&str, &str) = ("application", "crosslab");
const AUDIT_KIND: (&str, &str) = ("kind", "owner-audit-currentness");
const MAGIC: &[u8; 5] = b"CLAU\x01";
const MAX_FIELD: usize = 256 * 1024 + 64;

pub(crate) fn current_hour() -> Result<u64, LinuxAuditError> {
    Ok(SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|_| LinuxAuditError::Clock)?
        .as_secs()
        / 3600)
}

#[derive(Clone, Copy)]
enum AuditChange {
    Record(AuditAction, AuditOutcome, u64),
    Clear,
    Prune,
}

#[derive(Debug)]
pub(crate) struct LinuxAuditStore {
    path: PathBuf,
}

impl LinuxAuditStore {
    pub(crate) fn from_environment() -> Result<Self, LinuxAuditError> {
        let state = env::var_os("XDG_STATE_HOME")
            .map(PathBuf::from)
            .or_else(|| env::var_os("HOME").map(|home| PathBuf::from(home).join(".local/state")))
            .ok_or(LinuxAuditError::Path)?;
        Ok(Self {
            path: state.join("crosslab/audit/history-v1.bin"),
        })
    }

    pub(crate) async fn load(&self, hour: u64) -> Result<AuditHistory, LinuxAuditError> {
        self.mutate(hour, AuditChange::Prune).await
    }

    pub(crate) async fn record(
        &self,
        action: AuditAction,
        outcome: AuditOutcome,
        revision: u64,
        hour: u64,
    ) -> Result<AuditHistory, LinuxAuditError> {
        self.mutate(hour, AuditChange::Record(action, outcome, revision))
            .await
    }

    pub(crate) async fn clear(&self, hour: u64) -> Result<AuditHistory, LinuxAuditError> {
        self.mutate(hour, AuditChange::Clear).await
    }

    async fn mutate(
        &self,
        hour: u64,
        change: AuditChange,
    ) -> Result<AuditHistory, LinuxAuditError> {
        let parent = self.path.parent().ok_or(LinuxAuditError::Path)?;
        fs::create_dir_all(parent)?;
        private_dir(parent)?;
        let lock = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .mode_private()
            .open(self.path.with_extension("lock"))?;
        lock.lock()?;

        let previous = self.load_protected().await?;
        let mut history = match previous.as_ref() {
            Some(envelope) => AuditHistory::decode(envelope.payload(), hour)?,
            None => AuditHistory::default(),
        };
        match change {
            AuditChange::Record(action, outcome, revision) => {
                history.record(
                    crosslab_core::AuditRecord::new(action, outcome, hour, revision),
                    hour,
                )?;
            }
            AuditChange::Clear => history.clear(),
            AuditChange::Prune => {}
        }

        let payload = history.encode();
        if matches!(change, AuditChange::Prune)
            && previous
                .as_ref()
                .is_none_or(|envelope| envelope.payload() == payload)
        {
            return Ok(history);
        }
        let next = prepare_commit(previous.as_ref(), payload)?;
        let anchor = next.anchor();
        self.create_anchor(anchor).await?;

        let bundle = AuditBundle {
            envelope: next.envelope().encode(),
            anchor: anchor.encode().to_vec(),
        };
        if let Err(error) = self.write_bundle(&bundle) {
            let _ = self.delete_anchor(anchor.revision()).await;
            return Err(error);
        }
        if let Some(previous_revision) = next.previous_revision() {
            self.delete_anchor(previous_revision).await?;
        }
        Ok(history)
    }

    async fn load_protected(&self) -> Result<Option<IdentityStoreEnvelope>, LinuxAuditError> {
        let keyring = Keyring::new().await?;
        let items = keyring.search_items(&[APP, AUDIT_KIND]).await?;
        let Some(bundle) = self.read_bundle()? else {
            if !items.is_empty() {
                return Err(LinuxAuditError::Currentness);
            }
            return Ok(None);
        };
        if items.len() != 1 {
            return Err(LinuxAuditError::Currentness);
        }
        let envelope = IdentityStoreEnvelope::decode(&bundle.envelope)?;
        let anchor = IdentityStoreAnchor::decode(&bundle.anchor)?;
        validate_loaded(&envelope, anchor)?;
        let revision = anchor.revision().to_string();
        let attributes = items[0].attributes().await?;
        if attributes.get("revision").map(String::as_str) != Some(revision.as_str()) {
            return Err(LinuxAuditError::Currentness);
        }
        let secret = items[0].secret().await?;
        if secret.as_bytes() != anchor.protected_digest() {
            return Err(LinuxAuditError::Currentness);
        }
        Ok(Some(envelope))
    }

    async fn create_anchor(&self, anchor: IdentityStoreAnchor) -> Result<(), LinuxAuditError> {
        let keyring = Keyring::new().await?;
        let revision = anchor.revision().to_string();
        keyring
            .create_item(
                "Cross-Lab private owner audit anchor",
                &[APP, AUDIT_KIND, ("revision", revision.as_str())],
                &anchor.protected_digest(),
                true,
            )
            .await?;
        Ok(())
    }

    async fn delete_anchor(&self, revision: u64) -> Result<(), LinuxAuditError> {
        let keyring = Keyring::new().await?;
        let revision = revision.to_string();
        keyring
            .delete(&[APP, AUDIT_KIND, ("revision", revision.as_str())])
            .await?;
        Ok(())
    }

    fn read_bundle(&self) -> Result<Option<AuditBundle>, LinuxAuditError> {
        let file = match File::open(&self.path) {
            Ok(file) => file,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
            Err(error) => return Err(error.into()),
        };
        let mut bytes = Vec::new();
        file.take((MAX_FIELD * 2 + 16) as u64 + 1)
            .read_to_end(&mut bytes)?;
        Ok(Some(AuditBundle::decode(&bytes)?))
    }

    fn write_bundle(&self, bundle: &AuditBundle) -> Result<(), LinuxAuditError> {
        let parent = self.path.parent().ok_or(LinuxAuditError::Path)?;
        let temporary = self.path.with_extension("bin.new");
        let mut file = OpenOptions::new()
            .write(true)
            .create_new(true)
            .mode_private()
            .open(&temporary)?;
        file.write_all(&bundle.encode())?;
        file.sync_all()?;
        drop(file);
        fs::rename(&temporary, &self.path)?;
        private_file(&self.path)?;
        File::open(parent)?.sync_all()?;
        Ok(())
    }
}

pub(crate) enum LinuxAuditError {
    Path,
    Clock,
    Currentness,
    Malformed,
    Io(std::io::Error),
    Keyring(Box<oo7::Error>),
    Store(IdentityStoreError),
    Audit(AuditError),
}

impl core::fmt::Debug for LinuxAuditError {
    fn fmt(&self, formatter: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        formatter
            .debug_tuple("LinuxAuditError")
            .field(&self.to_string())
            .finish()
    }
}

impl core::fmt::Display for LinuxAuditError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.write_str(match self {
            Self::Path => "protected audit directory unavailable",
            Self::Clock => "local audit clock unavailable",
            Self::Currentness => "protected audit history currentness mismatch",
            Self::Malformed => "protected audit history container is malformed",
            Self::Io(_) => "protected audit storage I/O unavailable",
            Self::Keyring(_) => "audit protection keyring unavailable",
            Self::Store(_) | Self::Audit(_) => "protected audit history validation failed",
        })
    }
}

impl std::error::Error for LinuxAuditError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Io(error) => Some(error),
            Self::Keyring(error) => Some(error),
            Self::Store(error) => Some(error),
            Self::Audit(error) => Some(error),
            Self::Path | Self::Clock | Self::Currentness | Self::Malformed => None,
        }
    }
}

impl From<std::io::Error> for LinuxAuditError {
    fn from(error: std::io::Error) -> Self {
        Self::Io(error)
    }
}
impl From<oo7::Error> for LinuxAuditError {
    fn from(error: oo7::Error) -> Self {
        Self::Keyring(Box::new(error))
    }
}
impl From<IdentityStoreError> for LinuxAuditError {
    fn from(error: IdentityStoreError) -> Self {
        Self::Store(error)
    }
}
impl From<AuditError> for LinuxAuditError {
    fn from(error: AuditError) -> Self {
        Self::Audit(error)
    }
}

struct AuditBundle {
    envelope: Vec<u8>,
    anchor: Vec<u8>,
}

impl AuditBundle {
    fn encode(&self) -> Vec<u8> {
        let mut bytes = MAGIC.to_vec();
        for field in [&self.envelope, &self.anchor] {
            bytes.extend_from_slice(&(field.len() as u32).to_be_bytes());
            bytes.extend_from_slice(field);
        }
        bytes
    }

    fn decode(bytes: &[u8]) -> Result<Self, LinuxAuditError> {
        if !bytes.starts_with(MAGIC) {
            return Err(LinuxAuditError::Malformed);
        }
        let mut offset = MAGIC.len();
        let mut next = || -> Result<Vec<u8>, LinuxAuditError> {
            let len_bytes = bytes
                .get(offset..offset + 4)
                .ok_or(LinuxAuditError::Malformed)?;
            let len = u32::from_be_bytes(
                len_bytes
                    .try_into()
                    .map_err(|_| LinuxAuditError::Malformed)?,
            ) as usize;
            if len > MAX_FIELD {
                return Err(LinuxAuditError::Malformed);
            }
            offset += 4;
            let data = bytes
                .get(offset..offset + len)
                .ok_or(LinuxAuditError::Malformed)?
                .to_vec();
            offset += len;
            Ok(data)
        };
        let envelope = next()?;
        let anchor = next()?;
        if offset != bytes.len() {
            return Err(LinuxAuditError::Malformed);
        }
        Ok(Self { envelope, anchor })
    }
}

#[cfg(unix)]
trait PrivateOpenOptions {
    fn mode_private(&mut self) -> &mut Self;
}
#[cfg(unix)]
impl PrivateOpenOptions for OpenOptions {
    fn mode_private(&mut self) -> &mut Self {
        use std::os::unix::fs::OpenOptionsExt as _;
        self.mode(0o600)
    }
}

#[cfg(unix)]
fn private_dir(path: &Path) -> Result<(), std::io::Error> {
    use std::os::unix::fs::PermissionsExt as _;
    fs::set_permissions(path, fs::Permissions::from_mode(0o700))
}
#[cfg(unix)]
fn private_file(path: &Path) -> Result<(), std::io::Error> {
    use std::os::unix::fs::PermissionsExt as _;
    fs::set_permissions(path, fs::Permissions::from_mode(0o600))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn owner_visible_errors_hide_sources_and_remain_small() {
        let error = LinuxAuditError::Io(std::io::Error::other("PRIVATE_AUDIT_PAYLOAD"));
        assert_eq!(error.to_string(), "protected audit storage I/O unavailable");
        assert!(!format!("{error:?}").contains("PRIVATE_AUDIT_PAYLOAD"));
        assert!(std::error::Error::source(&error).is_some());
        assert!(std::mem::size_of::<LinuxAuditError>() <= 64);
    }

    #[test]
    fn malformed_and_trailing_audit_bundles_fail_closed() {
        let bundle = AuditBundle {
            envelope: vec![1, 2],
            anchor: vec![3],
        };
        let bytes = bundle.encode();
        let restored = AuditBundle::decode(&bytes).unwrap();
        assert_eq!(restored.envelope, vec![1, 2]);
        assert_eq!(restored.anchor, vec![3]);
        let mut trailing = bytes;
        trailing.push(1);
        assert!(AuditBundle::decode(&trailing).is_err());
        assert!(AuditBundle::decode(b"CLAU").is_err());
    }
}
