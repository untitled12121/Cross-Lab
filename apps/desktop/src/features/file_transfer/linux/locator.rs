use core::fmt;
use std::{
    fmt::Write as _,
    os::unix::ffi::{OsStrExt as _, OsStringExt as _},
    path::{Path, PathBuf},
};

use crosslab_agent::FileTransferLocalLocator;
use crosslab_protocol::TransferId;

use super::LinuxFileTransferError;

const LOCATOR_MAGIC: &[u8; 5] = b"CLFL\x01";

#[derive(Clone, PartialEq, Eq)]
pub struct LinuxFileTransferLocator {
    final_path: PathBuf,
    partial_path: PathBuf,
}

impl LinuxFileTransferLocator {
    pub fn for_destination(
        final_path: PathBuf,
        transfer_id: TransferId,
    ) -> Result<Self, LinuxFileTransferError> {
        if !valid_final_path(&final_path) {
            return Err(LinuxFileTransferError::InvalidPath);
        }
        let parent = final_path
            .parent()
            .ok_or(LinuxFileTransferError::InvalidPath)?;
        let partial_path = parent.join(partial_file_name(transfer_id));
        if partial_path == final_path {
            return Err(LinuxFileTransferError::InvalidPath);
        }
        Ok(Self {
            final_path,
            partial_path,
        })
    }

    pub fn decode(
        locator: &FileTransferLocalLocator,
        transfer_id: TransferId,
    ) -> Result<Self, LinuxFileTransferError> {
        let bytes = locator.bytes();
        if !bytes.starts_with(LOCATOR_MAGIC) {
            return Err(LinuxFileTransferError::InvalidLocator);
        }
        let mut offset = LOCATOR_MAGIC.len();
        let final_path = read_path(bytes, &mut offset)?;
        let partial_path = read_path(bytes, &mut offset)?;
        if offset != bytes.len() {
            return Err(LinuxFileTransferError::InvalidLocator);
        }

        let expected = Self::for_destination(final_path, transfer_id)?;
        if expected.partial_path != partial_path {
            return Err(LinuxFileTransferError::InvalidLocator);
        }
        Ok(expected)
    }

    pub fn encode(&self) -> Result<FileTransferLocalLocator, LinuxFileTransferError> {
        let mut bytes = Vec::with_capacity(
            LOCATOR_MAGIC.len()
                + 4
                + self.final_path.as_os_str().as_bytes().len()
                + self.partial_path.as_os_str().as_bytes().len(),
        );
        bytes.extend_from_slice(LOCATOR_MAGIC);
        push_path(&mut bytes, &self.final_path)?;
        push_path(&mut bytes, &self.partial_path)?;
        FileTransferLocalLocator::new(bytes).map_err(Into::into)
    }

    pub fn final_path(&self) -> &Path {
        &self.final_path
    }

    pub fn partial_path(&self) -> &Path {
        &self.partial_path
    }
}

impl fmt::Debug for LinuxFileTransferLocator {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("LinuxFileTransferLocator")
            .field("final_path", &"[REDACTED]")
            .field("partial_path", &"[REDACTED]")
            .finish()
    }
}

fn valid_final_path(path: &Path) -> bool {
    path.is_absolute() && path.file_name().is_some() && path.parent().is_some()
}

fn partial_file_name(transfer_id: TransferId) -> String {
    let mut output = String::with_capacity(11 + 64 + 5);
    output.push_str(".crosslab-");
    for byte in transfer_id.to_bytes() {
        write!(&mut output, "{byte:02x}").expect("writing to String cannot fail");
    }
    output.push_str(".part");
    output
}

fn push_path(output: &mut Vec<u8>, path: &Path) -> Result<(), LinuxFileTransferError> {
    let bytes = path.as_os_str().as_bytes();
    let len = u16::try_from(bytes.len()).map_err(|_| LinuxFileTransferError::InvalidLocator)?;
    output.extend_from_slice(&len.to_be_bytes());
    output.extend_from_slice(bytes);
    Ok(())
}

fn read_path(bytes: &[u8], offset: &mut usize) -> Result<PathBuf, LinuxFileTransferError> {
    let header_end = offset
        .checked_add(2)
        .ok_or(LinuxFileTransferError::InvalidLocator)?;
    let header = bytes
        .get(*offset..header_end)
        .ok_or(LinuxFileTransferError::InvalidLocator)?;
    let len = usize::from(u16::from_be_bytes(
        header
            .try_into()
            .map_err(|_| LinuxFileTransferError::InvalidLocator)?,
    ));
    let end = header_end
        .checked_add(len)
        .ok_or(LinuxFileTransferError::InvalidLocator)?;
    let path = bytes
        .get(header_end..end)
        .ok_or(LinuxFileTransferError::InvalidLocator)?;
    if path.is_empty() {
        return Err(LinuxFileTransferError::InvalidLocator);
    }
    *offset = end;
    Ok(PathBuf::from(std::ffi::OsString::from_vec(path.to_vec())))
}

#[cfg(test)]
mod tests {
    use std::{env, fs};

    use crosslab_crypto::random_bytes;

    use super::*;

    #[test]
    fn locator_round_trips_without_exposing_paths() {
        let root = test_root("locator");
        fs::create_dir_all(&root).unwrap();
        let transfer_id = TransferId::from_bytes([0x21; 32]);
        let final_path = root.join("owner-selected.bin");
        let locator =
            LinuxFileTransferLocator::for_destination(final_path.clone(), transfer_id).unwrap();
        let encoded = locator.encode().unwrap();
        let decoded = LinuxFileTransferLocator::decode(&encoded, transfer_id).unwrap();

        assert_eq!(decoded.final_path(), final_path);
        assert_eq!(decoded.partial_path(), locator.partial_path());
        let rendered = format!("{decoded:?}");
        assert!(!rendered.contains("owner-selected"));
        assert!(LinuxFileTransferLocator::decode(
            &encoded,
            TransferId::from_bytes([0x22; 32])
        )
        .is_err());

        let _ = fs::remove_dir_all(root);
    }

    fn test_root(label: &str) -> PathBuf {
        let nonce = random_bytes::<8>().unwrap();
        let mut suffix = String::with_capacity(16);
        for byte in nonce {
            write!(&mut suffix, "{byte:02x}").unwrap();
        }
        env::temp_dir().join(format!("crosslab-file-transfer-{label}-{suffix}"))
    }
}
