use core::fmt;

use crosslab_crypto::random_bytes;

pub const NOTIFICATION_CAPABILITY_ID: &str = "notifications.read";
pub const NOTIFICATION_SUBSCRIBE_OPERATION: &str = "subscribe";
pub const NOTIFICATION_POSTED_EVENT_TYPE: &str = "notifications.posted";
pub const NOTIFICATION_REMOVED_EVENT_TYPE: &str = "notifications.removed";
pub const NOTIFICATION_PROFILE_V3: u16 = 3;
pub const MAX_NOTIFICATION_WIRE_BYTES: usize = 896;
pub const MAX_NOTIFICATION_APP_LABEL_BYTES: usize = 96;
pub const MAX_NOTIFICATION_TITLE_BYTES: usize = 256;
pub const MAX_NOTIFICATION_PREVIEW_BYTES: usize = 512;

const WIRE_VERSION: u8 = 1;
const POSTED: u8 = 1;
const REMOVED: u8 = 2;
const REDACTED: u8 = 1;
const HAS_TITLE: u8 = 2;
const HAS_PREVIEW: u8 = 4;
const HEADER_LEN: usize = 2 + 16;

#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct NotificationId([u8; 16]);

impl NotificationId {
    pub const fn from_bytes(value: [u8; 16]) -> Self {
        Self(value)
    }

    pub fn generate() -> Result<Self, NotificationProfileError> {
        random_bytes::<16>()
            .map(Self)
            .map_err(|_| NotificationProfileError::RandomUnavailable)
    }

    pub const fn to_bytes(self) -> [u8; 16] {
        self.0
    }
}

impl fmt::Debug for NotificationId {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("NotificationId([REDACTED])")
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NotificationProfileError {
    Malformed,
    UnsupportedVersion,
    Oversized,
    InvalidText,
    InvalidFlags,
    RandomUnavailable,
}

impl fmt::Display for NotificationProfileError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::Malformed => "notification payload is malformed",
            Self::UnsupportedVersion => "notification payload version is unsupported",
            Self::Oversized => "notification payload exceeds profile bounds",
            Self::InvalidText => "notification display text is invalid",
            Self::InvalidFlags => "notification payload flags are invalid",
            Self::RandomUnavailable => "notification identifier generation failed",
        })
    }
}

impl std::error::Error for NotificationProfileError {}

#[derive(Clone, PartialEq, Eq)]
pub struct NotificationPosted {
    id: NotificationId,
    app_label: String,
    title: Option<String>,
    preview: Option<String>,
    redacted: bool,
}

impl fmt::Debug for NotificationPosted {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("NotificationPosted")
            .field("id", &self.id)
            .field("redacted", &self.redacted)
            .field("app_label_len", &self.app_label.len())
            .field("has_title", &self.title.is_some())
            .field("has_preview", &self.preview.is_some())
            .finish()
    }
}

impl NotificationPosted {
    pub fn new(
        id: NotificationId,
        app_label: String,
        title: Option<String>,
        preview: Option<String>,
        redacted: bool,
    ) -> Result<Self, NotificationProfileError> {
        if !valid_display(&app_label, MAX_NOTIFICATION_APP_LABEL_BYTES)
            || title
                .as_ref()
                .is_some_and(|value| !valid_display(value, MAX_NOTIFICATION_TITLE_BYTES))
            || preview
                .as_ref()
                .is_some_and(|value| !valid_display(value, MAX_NOTIFICATION_PREVIEW_BYTES))
        {
            return Err(NotificationProfileError::InvalidText);
        }
        if redacted && (title.is_some() || preview.is_some()) {
            return Err(NotificationProfileError::InvalidFlags);
        }
        Ok(Self {
            id,
            app_label,
            title,
            preview,
            redacted,
        })
    }

    pub const fn id(&self) -> NotificationId {
        self.id
    }

    pub fn app_label(&self) -> &str {
        &self.app_label
    }

    pub fn title(&self) -> Option<&str> {
        self.title.as_deref()
    }

    pub fn preview(&self) -> Option<&str> {
        self.preview.as_deref()
    }

    pub const fn redacted(&self) -> bool {
        self.redacted
    }
}

#[derive(Clone, PartialEq, Eq)]
pub enum NotificationPayload {
    Posted(NotificationPosted),
    Removed(NotificationId),
}

impl fmt::Debug for NotificationPayload {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Posted(value) => value.fmt(formatter),
            Self::Removed(id) => formatter.debug_tuple("Removed").field(id).finish(),
        }
    }
}

impl NotificationPayload {
    pub fn encode(&self) -> Vec<u8> {
        let mut bytes = Vec::with_capacity(MAX_NOTIFICATION_WIRE_BYTES);
        bytes.push(WIRE_VERSION);
        match self {
            Self::Posted(posted) => {
                bytes.push(POSTED);
                bytes.extend_from_slice(&posted.id.to_bytes());
                let mut flags = 0;
                if posted.redacted {
                    flags |= REDACTED;
                }
                if posted.title.is_some() {
                    flags |= HAS_TITLE;
                }
                if posted.preview.is_some() {
                    flags |= HAS_PREVIEW;
                }
                bytes.push(flags);
                bytes.push(posted.app_label.len() as u8);
                bytes.extend_from_slice(posted.app_label.as_bytes());
                if let Some(title) = &posted.title {
                    bytes.extend_from_slice(&(title.len() as u16).to_be_bytes());
                    bytes.extend_from_slice(title.as_bytes());
                }
                if let Some(preview) = &posted.preview {
                    bytes.extend_from_slice(&(preview.len() as u16).to_be_bytes());
                    bytes.extend_from_slice(preview.as_bytes());
                }
            }
            Self::Removed(id) => {
                bytes.push(REMOVED);
                bytes.extend_from_slice(&id.to_bytes());
            }
        }
        bytes
    }

    pub fn decode(bytes: &[u8]) -> Result<Self, NotificationProfileError> {
        if bytes.len() > MAX_NOTIFICATION_WIRE_BYTES {
            return Err(NotificationProfileError::Oversized);
        }
        if bytes.len() < HEADER_LEN {
            return Err(NotificationProfileError::Malformed);
        }
        if bytes[0] != WIRE_VERSION {
            return Err(NotificationProfileError::UnsupportedVersion);
        }
        let mut offset = 2;
        let id = NotificationId::from_bytes(read_array(bytes, &mut offset)?);
        match bytes[1] {
            REMOVED if offset == bytes.len() => Ok(Self::Removed(id)),
            POSTED => {
                let flags = read_array::<1>(bytes, &mut offset)?[0];
                if flags & !(REDACTED | HAS_TITLE | HAS_PREVIEW) != 0 {
                    return Err(NotificationProfileError::InvalidFlags);
                }
                let app_len = read_array::<1>(bytes, &mut offset)?[0] as usize;
                let app_label = read_text(bytes, &mut offset, app_len)?;
                let title = if flags & HAS_TITLE != 0 {
                    let size = u16::from_be_bytes(read_array(bytes, &mut offset)?) as usize;
                    Some(read_text(bytes, &mut offset, size)?)
                } else {
                    None
                };
                let preview = if flags & HAS_PREVIEW != 0 {
                    let size = u16::from_be_bytes(read_array(bytes, &mut offset)?) as usize;
                    Some(read_text(bytes, &mut offset, size)?)
                } else {
                    None
                };
                if offset != bytes.len() {
                    return Err(NotificationProfileError::Malformed);
                }
                NotificationPosted::new(id, app_label, title, preview, flags & REDACTED != 0)
                    .map(Self::Posted)
            }
            _ => Err(NotificationProfileError::Malformed),
        }
    }
}

pub fn validate_notification_subscribe(body: &[u8]) -> Result<(), NotificationProfileError> {
    if body.is_empty() {
        Ok(())
    } else {
        Err(NotificationProfileError::Malformed)
    }
}

fn valid_display(text: &str, max: usize) -> bool {
    text.len() <= max && !text.chars().any(char::is_control)
}

fn read_array<const N: usize>(
    bytes: &[u8],
    offset: &mut usize,
) -> Result<[u8; N], NotificationProfileError> {
    let end = offset
        .checked_add(N)
        .ok_or(NotificationProfileError::Malformed)?;
    let slice = bytes
        .get(*offset..end)
        .ok_or(NotificationProfileError::Malformed)?;
    let mut out = [0; N];
    out.copy_from_slice(slice);
    *offset = end;
    Ok(out)
}

fn read_text(
    bytes: &[u8],
    offset: &mut usize,
    size: usize,
) -> Result<String, NotificationProfileError> {
    let end = offset
        .checked_add(size)
        .ok_or(NotificationProfileError::Malformed)?;
    let raw = bytes
        .get(*offset..end)
        .ok_or(NotificationProfileError::Malformed)?;
    *offset = end;
    let text = std::str::from_utf8(raw).map_err(|_| NotificationProfileError::InvalidText)?;
    Ok(text.to_owned())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn notification() -> NotificationPayload {
        NotificationPayload::Posted(
            NotificationPosted::new(
                NotificationId::from_bytes([7; 16]),
                "Messages".to_owned(),
                Some("Subject".to_owned()),
                Some("Text".to_owned()),
                false,
            )
            .unwrap(),
        )
    }

    #[test]
    fn posted_and_removed_roundtrip_with_no_platform_identifiers() {
        let posted = notification();
        assert_eq!(NotificationPayload::decode(&posted.encode()), Ok(posted));
        let removed = NotificationPayload::Removed(NotificationId::from_bytes([2; 16]));
        let encoded = removed.encode();
        assert_eq!(encoded.len(), HEADER_LEN);
        assert_eq!(NotificationPayload::decode(&encoded), Ok(removed));
    }

    #[test]
    fn exact_golden_removed_payload_and_empty_subscribe() {
        let removed = NotificationPayload::Removed(NotificationId::from_bytes([0x7a; 16]));
        let mut golden = vec![1, 2];
        golden.extend_from_slice(&[0x7a; 16]);
        assert_eq!(removed.encode(), golden);
        assert_eq!(validate_notification_subscribe(&[]), Ok(()));
        assert_eq!(
            validate_notification_subscribe(&[0]),
            Err(NotificationProfileError::Malformed)
        );
    }

    #[test]
    fn corrupted_unknown_and_trailing_payloads_fail_closed() {
        let mut encoded = notification().encode();
        encoded[0] = 9;
        assert_eq!(
            NotificationPayload::decode(&encoded),
            Err(NotificationProfileError::UnsupportedVersion)
        );
        encoded[0] = 1;
        encoded[18] = 0x80;
        assert_eq!(
            NotificationPayload::decode(&encoded),
            Err(NotificationProfileError::InvalidFlags)
        );
        let mut removed =
            NotificationPayload::Removed(NotificationId::from_bytes([3; 16])).encode();
        removed.push(0);
        assert_eq!(
            NotificationPayload::decode(&removed),
            Err(NotificationProfileError::Malformed)
        );
        assert_eq!(
            NotificationPayload::decode(&vec![0; MAX_NOTIFICATION_WIRE_BYTES + 1]),
            Err(NotificationProfileError::Oversized)
        );
    }

    #[test]
    fn display_bounds_redaction_and_debug_privacy() {
        let id = NotificationId::from_bytes([5; 16]);
        assert!(
            NotificationPosted::new(
                id,
                "A".repeat(MAX_NOTIFICATION_APP_LABEL_BYTES + 1),
                None,
                None,
                false
            )
            .is_err()
        );
        assert!(
            NotificationPosted::new(
                id,
                String::new(),
                Some("T".repeat(MAX_NOTIFICATION_TITLE_BYTES + 1)),
                None,
                false
            )
            .is_err()
        );
        assert!(
            NotificationPosted::new(
                id,
                String::new(),
                None,
                Some("P".repeat(MAX_NOTIFICATION_PREVIEW_BYTES + 1)),
                false
            )
            .is_err()
        );
        assert!(
            NotificationPosted::new(id, String::new(), Some("secret".to_owned()), None, true)
                .is_err()
        );
        assert!(NotificationPosted::new(id, "line\nfeed".to_owned(), None, None, false).is_err());
        let payload = notification();
        assert!(!format!("{payload:?}").contains("Subject"));
        assert!(!format!("{payload:?}").contains("Messages"));
        assert!(!format!("{payload:?}").contains("Text"));
    }

    #[test]
    fn invalid_utf8_is_not_lossily_decoded() {
        let mut bytes = notification().encode();
        bytes[20] = 0xff;
        assert_eq!(
            NotificationPayload::decode(&bytes),
            Err(NotificationProfileError::InvalidText)
        );
    }
}
