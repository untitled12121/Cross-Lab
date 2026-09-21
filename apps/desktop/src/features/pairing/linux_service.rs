use std::{
    net::{Ipv4Addr, SocketAddr},
    thread,
    time::Instant,
};

use crosslab_core::{PairingInstant, PairingInvitation};
use crosslab_identity_store::{ProductIdentityError, ProductIdentityState};
use crosslab_protocol::ProductPairingMessage;
use crosslab_runtime::{
    ProductPairingInviter, ProductPairingInviterExchange, ProductPairingNetworkError,
};
use crosslab_transport_quic::{
    ProductPairingQuicChannel, ProductPairingQuicError, ProductPairingQuicServer,
    ProductPairingQuicTimeouts,
};
use tokio::sync::{mpsc, watch};

use super::{
    INVITATION_LIFETIME, LinuxEd25519Signer, LinuxPairingAdvertisement, LinuxPairingDiscoveryError,
    persist_product_pairing_commit,
};

const COMMAND_CAPACITY: usize = 2;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DesktopPairingStage {
    Waiting,
    Connected,
    Verifying,
    SavingTrust,
    Finalizing,
    Paired,
    Expired,
    Cancelled,
    Failed,
}

impl DesktopPairingStage {
    pub const fn label(self) -> &'static str {
        match self {
            Self::Waiting => "Waiting for device",
            Self::Connected => "Device found",
            Self::Verifying => "Verifying",
            Self::SavingTrust => "Saving trust",
            Self::Finalizing => "Finalizing",
            Self::Paired => "Paired",
            Self::Expired => "Expired",
            Self::Cancelled => "Cancelled",
            Self::Failed => "Pairing failed",
        }
    }

    pub const fn message(self) -> &'static str {
        match self {
            Self::Waiting => "Scan this QR on the device you want to add.",
            Self::Connected => "A device connected. Verifying the one-time invitation…",
            Self::Verifying => "Checking the pairing transcript and device proof…",
            Self::SavingTrust => "Pairing verified. Saving reciprocal device trust…",
            Self::Finalizing => "Trust saved. Waiting for the other device to confirm…",
            Self::Paired => "Device paired successfully.",
            Self::Expired => "This one-time invitation expired. Generate a new QR to try again.",
            Self::Cancelled => "Pairing invitation cancelled.",
            Self::Failed => "Secure pairing did not complete. Generate a new invitation to retry.",
        }
    }

    pub const fn terminal(self) -> bool {
        matches!(
            self,
            Self::Paired | Self::Expired | Self::Cancelled | Self::Failed
        )
    }
}

#[derive(Debug)]
pub enum LinuxPairingServiceError {
    Identity(ProductIdentityError),
    Pairing(ProductPairingNetworkError),
    Discovery(LinuxPairingDiscoveryError),
    Quic(ProductPairingQuicError),
    Thread,
    CommandQueue,
}

impl core::fmt::Display for LinuxPairingServiceError {
    fn fmt(&self, formatter: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::Identity(error) => core::fmt::Display::fmt(error, formatter),
            Self::Pairing(error) => core::fmt::Display::fmt(error, formatter),
            Self::Discovery(error) => core::fmt::Display::fmt(error, formatter),
            Self::Quic(error) => core::fmt::Display::fmt(error, formatter),
            Self::Thread => formatter.write_str("failed to start the pairing service"),
            Self::CommandQueue => formatter.write_str("pairing control queue is unavailable"),
        }
    }
}

impl std::error::Error for LinuxPairingServiceError {}

impl From<ProductIdentityError> for LinuxPairingServiceError {
    fn from(error: ProductIdentityError) -> Self {
        Self::Identity(error)
    }
}

impl From<ProductPairingNetworkError> for LinuxPairingServiceError {
    fn from(error: ProductPairingNetworkError) -> Self {
        Self::Pairing(error)
    }
}

impl From<LinuxPairingDiscoveryError> for LinuxPairingServiceError {
    fn from(error: LinuxPairingDiscoveryError) -> Self {
        Self::Discovery(error)
    }
}

impl From<ProductPairingQuicError> for LinuxPairingServiceError {
    fn from(error: ProductPairingQuicError) -> Self {
        Self::Quic(error)
    }
}

pub struct LinuxProductPairingService {
    command_tx: mpsc::Sender<ServiceCommand>,
    status: watch::Receiver<DesktopPairingStage>,
}

impl LinuxProductPairingService {
    pub async fn start(
        invitation: PairingInvitation,
        identity: &ProductIdentityState,
        issuer: LinuxEd25519Signer,
        started_at: Instant,
    ) -> Result<Self, LinuxPairingServiceError> {
        let pairing_id = invitation.pairing_id();
        let authority = identity.authority_state()?;
        let pairing =
            ProductPairingInviter::new(invitation, identity.local_credential(), &authority)
                .map_err(ProductPairingNetworkError::from)?;
        let exchange = ProductPairingInviterExchange::new(pairing);

        let server = ProductPairingQuicServer::bind(
            SocketAddr::from((Ipv4Addr::UNSPECIFIED, 0)),
            ProductPairingQuicTimeouts::default(),
        )?;
        let port = server.local_addr()?.port();
        let advertisement = LinuxPairingAdvertisement::start(pairing_id, port).await?;

        let (command_tx, command_rx) = mpsc::channel(COMMAND_CAPACITY);
        let (status_tx, status) = watch::channel(DesktopPairingStage::Waiting);

        thread::Builder::new()
            .name("crosslab-product-pairing".into())
            .spawn(move || {
                let Ok(runtime) = tokio::runtime::Builder::new_current_thread()
                    .enable_all()
                    .build()
                else {
                    status_tx.send_replace(DesktopPairingStage::Failed);
                    return;
                };
                runtime.block_on(run_service(
                    server,
                    advertisement,
                    exchange,
                    ServiceContext {
                        authority,
                        issuer,
                        started_at,
                        status_tx,
                    },
                    command_rx,
                ));
            })
            .map_err(|_| LinuxPairingServiceError::Thread)?;

        Ok(Self { command_tx, status })
    }

    pub fn subscribe_status(&self) -> watch::Receiver<DesktopPairingStage> {
        self.status.clone()
    }

    pub fn status(&self) -> DesktopPairingStage {
        *self.status.borrow()
    }

    pub fn cancel(&self) -> Result<(), LinuxPairingServiceError> {
        self.command_tx
            .try_send(ServiceCommand::Cancel)
            .map_err(|_| LinuxPairingServiceError::CommandQueue)
    }
}

impl Drop for LinuxProductPairingService {
    fn drop(&mut self) {
        let _ = self.command_tx.try_send(ServiceCommand::Cancel);
    }
}

#[derive(Debug, Clone, Copy)]
enum ServiceCommand {
    Cancel,
}

struct ServiceContext {
    authority: crosslab_identity::OwnerAuthorityState,
    issuer: LinuxEd25519Signer,
    started_at: Instant,
    status_tx: watch::Sender<DesktopPairingStage>,
}

async fn run_service(
    server: ProductPairingQuicServer,
    advertisement: LinuxPairingAdvertisement,
    mut exchange: ProductPairingInviterExchange,
    context: ServiceContext,
    mut command_rx: mpsc::Receiver<ServiceCommand>,
) {
    let ServiceContext {
        authority,
        issuer,
        started_at,
        status_tx,
    } = context;
    let mut advertisement = Some(advertisement);
    let mut channel = loop {
        let remaining = INVITATION_LIFETIME.saturating_sub(started_at.elapsed());
        if remaining.is_zero() {
            let _ = exchange.cancel();
            if let Some(advertisement) = advertisement.take() {
                let _ = advertisement.stop().await;
            }
            status_tx.send_replace(DesktopPairingStage::Expired);
            return;
        }

        tokio::select! {
            command = command_rx.recv() => {
                if matches!(command, Some(ServiceCommand::Cancel) | None) {
                    let _ = exchange.cancel();
                    if let Some(advertisement) = advertisement.take() {
                        let _ = advertisement.stop().await;
                    }
                    status_tx.send_replace(DesktopPairingStage::Cancelled);
                    return;
                }
            }
            _ = tokio::time::sleep(remaining) => {
                let _ = exchange.cancel();
                if let Some(advertisement) = advertisement.take() {
                    let _ = advertisement.stop().await;
                }
                status_tx.send_replace(DesktopPairingStage::Expired);
                return;
            }
            accepted = server.accept() => {
                match accepted {
                    Ok(channel) => break channel,
                    Err(ProductPairingQuicError::Timeout) => continue,
                    Err(_) => {
                        if let Some(advertisement) = advertisement.take() {
                            let _ = advertisement.stop().await;
                        }
                        status_tx.send_replace(DesktopPairingStage::Failed);
                        return;
                    }
                }
            }
        }
    };

    if let Some(advertisement) = advertisement.take() {
        let _ = advertisement.stop().await;
    }
    status_tx.send_replace(DesktopPairingStage::Connected);

    let result = run_exchange(
        &mut exchange,
        &authority,
        &issuer,
        started_at,
        &mut channel,
        &mut command_rx,
        &status_tx,
    )
    .await;

    match result {
        Ok(()) => {
            let stage = if channel.finish().await.is_ok() {
                DesktopPairingStage::Paired
            } else {
                DesktopPairingStage::Failed
            };
            status_tx.send_replace(stage);
        }
        Err(ServiceRunError::Cancelled) => {
            status_tx.send_replace(DesktopPairingStage::Cancelled);
        }
        Err(ServiceRunError::Failed) => {
            status_tx.send_replace(DesktopPairingStage::Failed);
        }
    }
}

async fn run_exchange(
    exchange: &mut ProductPairingInviterExchange,
    authority: &crosslab_identity::OwnerAuthorityState,
    issuer: &LinuxEd25519Signer,
    started_at: Instant,
    channel: &mut ProductPairingQuicChannel,
    command_rx: &mut mpsc::Receiver<ServiceCommand>,
    status_tx: &watch::Sender<DesktopPairingStage>,
) -> Result<(), ServiceRunError> {
    let hello = receive_or_cancel(exchange, channel, command_rx).await?;
    let inviter_hello = exchange
        .accept_joiner_hello(hello, now(started_at))
        .map_err(|_| ServiceRunError::Failed)?;
    channel
        .send(&inviter_hello)
        .await
        .map_err(|_| ServiceRunError::Failed)?;

    status_tx.send_replace(DesktopPairingStage::Verifying);
    let confirmation = receive_or_cancel(exchange, channel, command_rx).await?;
    let inviter_confirmation = exchange
        .accept_joiner_confirmation(confirmation, now(started_at))
        .map_err(|_| ServiceRunError::Failed)?;
    channel
        .send(&inviter_confirmation)
        .await
        .map_err(|_| ServiceRunError::Failed)?;

    let credential = exchange
        .issue_credential_bundle(authority, issuer, now(started_at))
        .map_err(|_| ServiceRunError::Failed)?;
    channel
        .send(&credential)
        .await
        .map_err(|_| ServiceRunError::Failed)?;

    let proof = receive_or_cancel(exchange, channel, command_rx).await?;
    let commit = exchange
        .accept_credential_proof(proof, authority, issuer, now(started_at))
        .map_err(|_| ServiceRunError::Failed)?;

    status_tx.send_replace(DesktopPairingStage::SavingTrust);
    if persist_product_pairing_commit(commit).await.is_err() {
        let _ = exchange.persistence_failed();
        channel.close();
        return Err(ServiceRunError::Failed);
    }

    let trust = exchange
        .local_persisted()
        .map_err(|_| ServiceRunError::Failed)?;
    channel
        .send(&trust)
        .await
        .map_err(|_| ServiceRunError::Failed)?;

    status_tx.send_replace(DesktopPairingStage::Finalizing);
    let persisted = receive_or_cancel(exchange, channel, command_rx).await?;
    let complete = exchange
        .accept_peer_persisted(persisted)
        .map_err(|_| ServiceRunError::Failed)?;
    channel
        .send(&complete)
        .await
        .map_err(|_| ServiceRunError::Failed)?;

    Ok(())
}

async fn receive_or_cancel(
    exchange: &mut ProductPairingInviterExchange,
    channel: &mut ProductPairingQuicChannel,
    command_rx: &mut mpsc::Receiver<ServiceCommand>,
) -> Result<ProductPairingMessage, ServiceRunError> {
    tokio::select! {
        command = command_rx.recv() => {
            if matches!(command, Some(ServiceCommand::Cancel) | None) {
                if let Ok(cancel) = exchange.cancel() {
                    let _ = channel.send(&cancel).await;
                }
                channel.close();
                Err(ServiceRunError::Cancelled)
            } else {
                Err(ServiceRunError::Failed)
            }
        }
        message = channel.receive() => {
            message.map_err(|_| ServiceRunError::Failed)
        }
    }
}

fn now(started_at: Instant) -> PairingInstant {
    PairingInstant::from_ticks(
        started_at
            .elapsed()
            .as_millis()
            .try_into()
            .unwrap_or(u64::MAX),
    )
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ServiceRunError {
    Cancelled,
    Failed,
}
