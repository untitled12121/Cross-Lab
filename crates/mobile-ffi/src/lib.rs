//! Narrow mobile-facing FFI boundary for Cross-Lab.

#![allow(unsafe_code)]

#[cfg(feature = "development-provisioning")]
mod development;
mod dto;
mod error;
mod runtime;

pub use dto::{
    MobileConnectivityState, MobileLifecycleState, MobileNetworkClass, MobileProtocolVersion,
    MobileRuntimeSnapshot, MobileSessionState, MobileTransportSecurity, MobileTrustState,
};
pub use error::MobileRuntimeError;
pub use runtime::{MobileRuntime, MobileRuntimeEvent};

uniffi::setup_scaffolding!();

#[cfg(test)]
mod tests;
