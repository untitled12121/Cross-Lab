use std::{
    net::{IpAddr, Ipv4Addr, Ipv6Addr, SocketAddr},
    sync::{Arc, Mutex},
};

use crosslab_identity::DeviceId;
use crosslab_runtime::{
    ProductPairingJoiner, ProductPairingJoinerCompletion, ProductPairingJoinerExchange,
    ProductPairingNetworkError,
};
use crosslab_transport_quic::{
    ProductPairingQuicChannel, ProductPairingQuicClient, ProductPairingQuicError,
    ProductPairingQuicTimeouts,
};

use crate::{
    network::socket_addr,
    pairing::{MobilePairingBootstrap, MobilePairingBootstrapError},
    pairing_persistence::MobileProductPairingJoinerCompletion,
    product_identity::{ForeignSigningProvider, MobileProductIdentityError, MobileSigningProvider},
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, uniffi::Error)]
pub enum MobileProductPairingNetworkError {
    InvalidInvitation,
    InvalidRoute,
    SigningProvider,
    Random,
    Runtime,
    Connect,
    Protocol,
    State,
}

impl core::fmt::Display for MobileProductPairingNetworkError {
    fn fmt(&self, formatter: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        formatter.write_str(match self {
            Self::InvalidInvitation => "pairing invitation is unavailable",
            Self::InvalidRoute => "pairing route is invalid",
            Self::SigningProvider => "platform signing provider is unavailable",
            Self::Random => "secure pairing identity generation failed",
            Self::Runtime => "pairing runtime is unavailable",
            Self::Connect => "could not connect to the pairing device",
            Self::Protocol => "secure pairing verification failed",
            Self::State => "pairing session is no longer available",
        })
    }
}

impl std::error::Error for MobileProductPairingNetworkError {}

struct PendingJoiner {
    runtime: tokio::runtime::Runtime,
    channel: ProductPairingQuicChannel,
    exchange: ProductPairingJoinerExchange,
    completion: ProductPairingJoinerCompletion,
}

#[derive(uniffi::Object)]
pub struct MobileProductPairingJoinerSession {
    pending: Mutex<Option<PendingJoiner>>,
}

impl core::fmt::Debug for MobileProductPairingJoinerSession {
    fn fmt(&self, formatter: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        formatter.write_str("MobileProductPairingJoinerSession([REDACTED])")
    }
}

#[uniffi::export]
impl MobileProductPairingJoinerSession {
    pub fn completion(
        &self,
    ) -> Result<Arc<MobileProductPairingJoinerCompletion>, MobileProductPairingNetworkError> {
        let pending = self
            .pending
            .lock()
            .expect("mobile product pairing session lock poisoned");
        let completion = pending
            .as_ref()
            .ok_or(MobileProductPairingNetworkError::State)?
            .completion;
        Ok(Arc::new(MobileProductPairingJoinerCompletion {
            inner: completion,
        }))
    }

    pub fn finish_persisted(&self) -> Result<(), MobileProductPairingNetworkError> {
        let mut pending = self
            .pending
            .lock()
            .expect("mobile product pairing session lock poisoned")
            .take()
            .ok_or(MobileProductPairingNetworkError::State)?;

        let persisted = pending.exchange.local_persisted()?;
        pending.runtime.block_on(async {
            pending.channel.send(&persisted).await?;
            let complete = pending.channel.receive().await?;
            pending.exchange.accept_complete(complete)?;
            pending.channel.finish().await?;
            Ok::<(), MobileProductPairingNetworkError>(())
        })
    }

    pub fn persistence_failed(&self) -> Result<(), MobileProductPairingNetworkError> {
        let mut pending = self
            .pending
            .lock()
            .expect("mobile product pairing session lock poisoned")
            .take()
            .ok_or(MobileProductPairingNetworkError::State)?;
        pending.exchange.persistence_failed()?;
        pending.channel.close();
        Ok(())
    }

    pub fn cancel(&self) -> Result<(), MobileProductPairingNetworkError> {
        let mut pending = self
            .pending
            .lock()
            .expect("mobile product pairing session lock poisoned")
            .take()
            .ok_or(MobileProductPairingNetworkError::State)?;

        let cancel = pending.exchange.cancel()?;
        let _ = pending.runtime.block_on(async {
            pending.channel.send(&cancel).await?;
            pending.channel.finish().await
        });
        Ok(())
    }
}

#[uniffi::export]
pub fn start_product_pairing_joiner(
    bootstrap: Arc<MobilePairingBootstrap>,
    address: Vec<u8>,
    port: i32,
    scope_id: i32,
    local_device_signer: Arc<dyn MobileSigningProvider>,
) -> Result<Arc<MobileProductPairingJoinerSession>, MobileProductPairingNetworkError> {
    let bootstrap = bootstrap.take()?;
    let signer = ForeignSigningProvider::new(local_device_signer)?;
    let joiner_id = DeviceId::generate().map_err(|_| MobileProductPairingNetworkError::Random)?;
    let pairing = ProductPairingJoiner::new(bootstrap, joiner_id, &signer)?;
    let exchange = ProductPairingJoinerExchange::new(pairing);
    let remote = socket_addr(address, port, scope_id)
        .ok_or(MobileProductPairingNetworkError::InvalidRoute)?;
    let bind = unspecified_bind(remote);
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .map_err(|_| MobileProductPairingNetworkError::Runtime)?;

    let (channel, exchange, completion) = runtime.block_on(async move {
        let mut exchange = exchange;
        let mut channel =
            ProductPairingQuicClient::connect(bind, remote, ProductPairingQuicTimeouts::default())
                .await?;

        channel.send(&exchange.hello()).await?;

        let inviter_hello = channel.receive().await?;
        let confirmation = exchange.accept_inviter_hello(inviter_hello)?;
        channel.send(&confirmation).await?;

        let inviter_confirmation = channel.receive().await?;
        exchange.accept_inviter_confirmation(inviter_confirmation)?;

        let credential = channel.receive().await?;
        let proof = exchange.accept_credential_bundle(credential, &signer)?;
        channel.send(&proof).await?;

        let trust = channel.receive().await?;
        let completion = exchange.accept_trust_bundle(trust)?;

        Ok::<_, MobileProductPairingNetworkError>((channel, exchange, completion))
    })?;

    Ok(Arc::new(MobileProductPairingJoinerSession {
        pending: Mutex::new(Some(PendingJoiner {
            runtime,
            channel,
            exchange,
            completion,
        })),
    }))
}

fn unspecified_bind(remote: SocketAddr) -> SocketAddr {
    match remote.ip() {
        IpAddr::V4(_) => SocketAddr::from((Ipv4Addr::UNSPECIFIED, 0)),
        IpAddr::V6(_) => SocketAddr::from((Ipv6Addr::UNSPECIFIED, 0)),
    }
}

impl From<MobilePairingBootstrapError> for MobileProductPairingNetworkError {
    fn from(_: MobilePairingBootstrapError) -> Self {
        Self::InvalidInvitation
    }
}

impl From<MobileProductIdentityError> for MobileProductPairingNetworkError {
    fn from(_: MobileProductIdentityError) -> Self {
        Self::SigningProvider
    }
}

impl From<crosslab_runtime::ProductPairingError> for MobileProductPairingNetworkError {
    fn from(_: crosslab_runtime::ProductPairingError) -> Self {
        Self::Protocol
    }
}

impl From<ProductPairingNetworkError> for MobileProductPairingNetworkError {
    fn from(_: ProductPairingNetworkError) -> Self {
        Self::Protocol
    }
}

impl From<ProductPairingQuicError> for MobileProductPairingNetworkError {
    fn from(error: ProductPairingQuicError) -> Self {
        match error {
            ProductPairingQuicError::Bind
            | ProductPairingQuicError::Tls
            | ProductPairingQuicError::Connect
            | ProductPairingQuicError::Accept
            | ProductPairingQuicError::Timeout => Self::Connect,
            ProductPairingQuicError::AlreadyAccepted
            | ProductPairingQuicError::Stream
            | ProductPairingQuicError::Record
            | ProductPairingQuicError::Protocol(_) => Self::Protocol,
        }
    }
}
