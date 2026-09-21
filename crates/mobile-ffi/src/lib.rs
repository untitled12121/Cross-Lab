//! Narrow mobile-facing FFI boundary for Cross-Lab.

#![allow(unsafe_code)]

#[cfg(feature = "development-provisioning")]
mod development;
mod dto;
mod error;
mod identity_store;
mod pairing;
mod pairing_network;
mod pairing_persistence;
mod product_identity;
mod runtime;

pub use dto::{
    MobileConnectivityState, MobileLifecycleState, MobileNetworkClass, MobileProtocolVersion,
    MobileRuntimeSnapshot, MobileSessionState, MobileTransportSecurity, MobileTrustState,
};
pub use error::MobileRuntimeError;
pub use identity_store::{MobileIdentityCommit, MobileIdentityStoreError};
pub use pairing::{
    MobilePairingBootstrap, MobilePairingBootstrapError, MobilePairingBootstrapSummary,
};
pub use pairing_network::{
    MobileProductPairingJoinerSession, MobileProductPairingNetworkError,
};
pub use pairing_persistence::{MobileProductPairingCommit, MobileProductPairingJoinerCompletion};
pub use product_identity::{
    MobileProductIdentity, MobileProductIdentityError, MobileSigningCallbackError,
    MobileSigningProvider,
};
pub use runtime::{MobileRuntime, MobileRuntimeEvent};

uniffi::setup_scaffolding!();

#[cfg(test)]
mod tests;
