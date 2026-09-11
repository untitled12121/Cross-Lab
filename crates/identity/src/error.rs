use core::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IdentityError {
    UnsupportedSchema,
    UnsupportedAlgorithm,
    MalformedIdentifier,
    MalformedPublicKey,
    UnknownOwner,
    UnknownIssuer,
    WrongIssuerRole,
    InvalidSignature,
    WrongOwner,
    StaleCredentialEpoch,
    UnexpectedCredentialEpoch,
    RevokedDevice,
    InvalidAuthorityEpoch,
    InvalidRootSuccessor,
}

impl fmt::Display for IdentityError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::UnsupportedSchema => "unsupported identity schema",
            Self::UnsupportedAlgorithm => "unsupported identity algorithm",
            Self::MalformedIdentifier => "malformed identity identifier",
            Self::MalformedPublicKey => "malformed or inconsistent public key",
            Self::UnknownOwner => "unknown owner",
            Self::UnknownIssuer => "unknown credential issuer",
            Self::WrongIssuerRole => "issuer role is not authorized for this operation",
            Self::InvalidSignature => "identity signature verification failed",
            Self::WrongOwner => "identity belongs to a different owner",
            Self::StaleCredentialEpoch => "credential epoch is stale",
            Self::UnexpectedCredentialEpoch => "credential epoch transition is invalid",
            Self::RevokedDevice => "device is revoked",
            Self::InvalidAuthorityEpoch => "authority epoch is stale or invalid",
            Self::InvalidRootSuccessor => "owner root successor continuity is invalid",
        })
    }
}

impl std::error::Error for IdentityError {}
