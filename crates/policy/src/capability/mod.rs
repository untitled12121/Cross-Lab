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

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct CapabilityVersion {
    major: u16,
    minor: u16,
}

impl CapabilityVersion {
    pub const fn new(major: u16, minor: u16) -> Self {
        Self { major, minor }
    }

    pub const fn major(self) -> u16 {
        self.major
    }

    pub const fn minor(self) -> u16 {
        self.minor
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CapabilityVersionRange {
    major: u16,
    min_minor: u16,
    max_minor: u16,
}

impl CapabilityVersionRange {
    pub const fn new(
        major: u16,
        min_minor: u16,
        max_minor: u16,
    ) -> Result<Self, VersionRangeError> {
        if min_minor > max_minor {
            return Err(VersionRangeError);
        }
        Ok(Self {
            major,
            min_minor,
            max_minor,
        })
    }

    pub const fn supports(self, version: CapabilityVersion) -> bool {
        version.major == self.major
            && version.minor >= self.min_minor
            && version.minor <= self.max_minor
    }

    pub const fn major(self) -> u16 {
        self.major
    }

    pub const fn min_minor(self) -> u16 {
        self.min_minor
    }

    pub const fn max_minor(self) -> u16 {
        self.max_minor
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LocalCapability {
    capability_id: CapabilityId,
    supported_versions: CapabilityVersionRange,
    runtime_available: bool,
}

impl LocalCapability {
    pub const fn new(
        capability_id: CapabilityId,
        supported_versions: CapabilityVersionRange,
        runtime_available: bool,
    ) -> Self {
        Self {
            capability_id,
            supported_versions,
            runtime_available,
        }
    }

    pub fn capability_id(&self) -> &CapabilityId {
        &self.capability_id
    }

    pub const fn supports(&self, version: CapabilityVersion) -> bool {
        self.supported_versions.supports(version)
    }

    pub const fn supported_versions(&self) -> CapabilityVersionRange {
        self.supported_versions
    }

    pub const fn runtime_available(&self) -> bool {
        self.runtime_available
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct VersionRangeError;

impl fmt::Display for VersionRangeError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("capability version range is inverted")
    }
}

impl std::error::Error for VersionRangeError {}

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
