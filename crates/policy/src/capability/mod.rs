use core::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct IdentifierError;

impl fmt::Display for IdentifierError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("invalid canonical policy identifier")
    }
}

impl std::error::Error for IdentifierError {}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct CapabilityId(String);

impl CapabilityId {
    pub fn parse(value: &str) -> Result<Self, IdentifierError> {
        if value.len() > 128 || !value.is_ascii() {
            return Err(IdentifierError);
        }

        let mut segments = value.split('.');
        let first = segments.next().ok_or(IdentifierError)?;
        let second = segments.next().ok_or(IdentifierError)?;
        if !valid_segment(first) || !valid_segment(second) || !segments.all(valid_segment) {
            return Err(IdentifierError);
        }

        Ok(Self(value.to_owned()))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct OperationName(String);

impl OperationName {
    pub fn parse(value: &str) -> Result<Self, IdentifierError> {
        if value.len() > 64 || !valid_segment(value) {
            return Err(IdentifierError);
        }
        Ok(Self(value.to_owned()))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

fn valid_segment(value: &str) -> bool {
    let bytes = value.as_bytes();
    if bytes.is_empty() || !bytes[0].is_ascii_lowercase() || bytes.last() == Some(&b'-') {
        return false;
    }

    bytes
        .iter()
        .skip(1)
        .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || *byte == b'-')
}
