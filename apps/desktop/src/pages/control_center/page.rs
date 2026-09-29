#[cfg(target_os = "linux")]
use std::{collections::VecDeque, env, path::PathBuf, sync::Arc};

#[cfg(target_os = "linux")]
use crosslab_agent::{ClipboardPlatformError, ClipboardRequest};
#[cfg(target_os = "linux")]
use crosslab_protocol::TransferId;

#[cfg(feature = "development-provisioning")]
use crate::features::devices::TrustDisplay;
#[cfg(target_os = "linux")]
use crate::features::{
    devices::{DesktopPresenceError, DesktopProductPresenceController},
    file_transfer::{
        LinuxFileTransferReceiveFailure, LinuxFileTransferReceiveStatus,
        LinuxFileTransferSendFailure, LinuxFileTransferSendHandle, LinuxFileTransferSendStatus,
        LinuxFileTransferSendToken, LinuxIncomingFileTransfer,
    },
};
use crate::{
    features::{
        appearance::{active_theme, font_weight},
        clipboard::{ClipboardAction, ClipboardFeatureState, ClipboardResult},
        devices::{
            ConnectivityDisplay, DesktopRuntimeController, DevicesFeatureState, PresenceDisplay,
            SessionDisplay,
        },
        file_transfer::{FileTransferFailure, FileTransferFeatureState, FileTransferStage},
        owner::OwnerFeatureState,
        pairing::{DesktopPairingInvitation, DesktopPairingStage, load_existing_product_identity},
    },
    pages::control_center::{
        _components::{
            clipboard_panel, devices_content, file_transfer_panel, owner_content,
            pairing_invitation_panel,
        },
        layout::control_center_layout,
    },
};

#[cfg(target_os = "linux")]
use gpui_kit::PathPromptOptions;
use gpui_kit::{
    ClipboardItem, Context, InteractiveElement as _, IntoElement, ParentElement as _, Render,
    Styled as _, Window, base::Button, component::theme::ActiveTheme as _, div, px,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Section {
    Devices,
    Owner,
}

pub struct ControlCenterPage {
    section: Section,
    devices: DevicesFeatureState,
    clipboard: ClipboardFeatureState,
    file_transfer: FileTransferFeatureState,
    owner: OwnerFeatureState,
    runtime: DesktopRuntimeController,
    #[cfg(target_os = "linux")]
    product_presence: Option<Arc<DesktopProductPresenceController>>,
    #[cfg(target_os = "linux")]
    presence_starting: bool,
    #[cfg(target_os = "linux")]
    pending_file_transfers: VecDeque<LinuxIncomingFileTransfer>,
    #[cfg(target_os = "linux")]
    file_transfer_send: Option<LinuxFileTransferSendHandle>,
    #[cfg(target_os = "linux")]
    file_transfer_retry: Option<LinuxFileTransferSendToken>,
    #[cfg(target_os = "linux")]
    file_transfer_send_generation: u64,
    #[cfg(target_os = "linux")]
    active_receive_transfer: Option<TransferId>,
    #[cfg(target_os = "linux")]
    file_transfer_send_dialog_open: bool,
    #[cfg(target_os = "linux")]
    file_transfer_receive_dialog_open: bool,
    pairing_invitation: Option<DesktopPairingInvitation>,
    pairing_generation: u64,
    pairing_busy: bool,
    notice: Option<String>,
}

impl ControlCenterPage {
    pub fn new(cx: &mut Context<Self>) -> Self {
        let runtime = DesktopRuntimeController::from_environment();
        let mut status = runtime.subscribe_status();
        let devices = status.borrow().as_ref().map_or_else(
            DevicesFeatureState::empty,
            DevicesFeatureState::from_runtime,
        );
        let owner = status
            .borrow()
            .as_ref()
            .map_or_else(OwnerFeatureState::empty, OwnerFeatureState::from_runtime);

        cx.spawn(async move |this, cx| {
            while status.changed().await.is_ok() {
                let next = status.borrow_and_update().clone();
                if this
                    .update(cx, |page, cx| {
                        match next.as_ref() {
                            Some(status) => {
                                page.devices.update_runtime(status);
                                page.owner.update_runtime(status);
                            }
                            None => {
                                page.devices.clear();
                                page.owner.clear();
                            }
                        }
                        page.notice = None;
                        cx.notify();
                    })
                    .is_err()
                {
                    return;
                }
            }
        })
        .detach();

        cx.spawn(async move |this, cx| {
            let result = load_existing_product_identity().await;
            let _ = this.update(cx, |page, cx| {
                match result {
                    Ok(Some(identity)) => page.owner.set_product_identity(
                        identity.owner_id().to_owned(),
                        identity.local_device_id().to_owned(),
                    ),
                    Ok(None) => {}
                    Err(error) => {
                        page.notice = Some(error.to_string());
                    }
                }
                cx.notify();
            });
        })
        .detach();

        let mut page = Self {
            section: Section::Devices,
            devices,
            clipboard: ClipboardFeatureState::new(),
            file_transfer: FileTransferFeatureState::new(),
            owner,
            runtime,
            #[cfg(target_os = "linux")]
            product_presence: None,
            #[cfg(target_os = "linux")]
            presence_starting: false,
            #[cfg(target_os = "linux")]
            pending_file_transfers: VecDeque::new(),
            #[cfg(target_os = "linux")]
            file_transfer_send: None,
            #[cfg(target_os = "linux")]
            file_transfer_retry: None,
            #[cfg(target_os = "linux")]
            file_transfer_send_generation: 0,
            #[cfg(target_os = "linux")]
            active_receive_transfer: None,
            #[cfg(target_os = "linux")]
            file_transfer_send_dialog_open: false,
            #[cfg(target_os = "linux")]
            file_transfer_receive_dialog_open: false,
            pairing_invitation: None,
            pairing_generation: 0,
            pairing_busy: false,
            notice: None,
        };
        #[cfg(target_os = "linux")]
        page.start_product_presence(cx);
        page
    }

    #[cfg(target_os = "linux")]
    fn start_product_presence(&mut self, cx: &mut Context<Self>) {
        if self.runtime.uses_development_provisioning()
            || self.product_presence.is_some()
            || self.presence_starting
        {
            return;
        }

        self.presence_starting = true;
        cx.spawn(async move |this, cx| {
            let result = DesktopProductPresenceController::start().await;
            let controller = match result {
                Ok(Some(controller)) => controller,
                Ok(None) => {
                    let _ = this.update(cx, |page, cx| {
                        page.presence_starting = false;
                        page.clipboard.set_available(false);
                        page.file_transfer.set_available(false);
                        cx.notify();
                    });
                    return;
                }
                Err(error) => {
                    let _ = this.update(cx, |page, cx| {
                        page.presence_starting = false;
                        page.clipboard.set_available(false);
                        page.file_transfer.set_available(false);
                        page.notice = Some(error.to_string());
                        cx.notify();
                    });
                    return;
                }
            };

            let controller = Arc::new(controller);
            let mut status = controller.subscribe_status();
            let mut permissions = controller.subscribe_permissions();
            let mut clipboard_requests = match controller.take_clipboard_requests() {
                Ok(requests) => requests,
                Err(error) => {
                    let _ = this.update(cx, |page, cx| {
                        page.presence_starting = false;
                        page.clipboard.set_available(false);
                        page.notice = Some(error.to_string());
                        cx.notify();
                    });
                    return;
                }
            };
            let mut file_transfer_offers = match controller.take_file_transfer_offers() {
                Ok(offers) => offers,
                Err(error) => {
                    let _ = this.update(cx, |page, cx| {
                        page.presence_starting = false;
                        page.clipboard.set_available(false);
                        page.file_transfer.set_available(false);
                        page.notice = Some(error.to_string());
                        cx.notify();
                    });
                    return;
                }
            };
            let mut file_transfer_statuses = match controller.take_file_transfer_statuses() {
                Ok(statuses) => statuses,
                Err(error) => {
                    let _ = this.update(cx, |page, cx| {
                        page.presence_starting = false;
                        page.clipboard.set_available(false);
                        page.file_transfer.set_available(false);
                        page.notice = Some(error.to_string());
                        cx.notify();
                    });
                    return;
                }
            };
            let initial = status.borrow().clone();
            let initial_permissions = permissions.borrow().clone();
            if this
                .update(cx, |page, cx| {
                    page.presence_starting = false;
                    page.devices.update_presence(&initial);
                    page.devices.update_permissions(&initial_permissions);
                    page.clipboard.set_available(true);
                    page.file_transfer.set_available(true);
                    if let Some(runtime) = initial.runtime() {
                        page.owner.update_runtime(runtime);
                    }
                    page.product_presence = Some(Arc::clone(&controller));
                    page.notice = None;
                    cx.notify();
                })
                .is_err()
            {
                return;
            }

            loop {
                tokio::select! {
                    changed = status.changed() => {
                        if changed.is_err() {
                            return;
                        }
                        let snapshot = status.borrow_and_update().clone();
                        if this
                            .update(cx, |page, cx| {
                                page.devices.update_presence(&snapshot);
                                if let Some(runtime) = snapshot.runtime() {
                                    page.owner.update_runtime(runtime);
                                }
                                cx.notify();
                            })
                            .is_err()
                        {
                            return;
                        }
                    }
                    changed = permissions.changed() => {
                        if changed.is_err() {
                            return;
                        }
                        let snapshot = permissions.borrow_and_update().clone();
                        if this
                            .update(cx, |page, cx| {
                                page.devices.update_permissions(&snapshot);
                                cx.notify();
                            })
                            .is_err()
                        {
                            return;
                        }
                    }
                    request = clipboard_requests.recv() => {
                        let Some(request) = request else {
                            return;
                        };
                        match request {
                            ClipboardRequest::Read { request_id } => {
                                let result = this.update(cx, |_, cx| {
                                    cx.read_from_clipboard()
                                        .and_then(|item| item.text())
                                        .ok_or(ClipboardPlatformError::Unavailable)
                                });
                                let Ok(result) = result else {
                                    return;
                                };
                                let _ = controller.complete_clipboard_read(request_id, result).await;
                            }
                            ClipboardRequest::Write { request_id, text } => {
                                if this
                                    .update(cx, move |_, cx| {
                                        cx.write_to_clipboard(ClipboardItem::new_string(text));
                                    })
                                    .is_err()
                                {
                                    return;
                                }
                                let _ = controller.complete_clipboard_write(request_id, Ok(())).await;
                            }
                        }
                    }
                    incoming = file_transfer_offers.recv() => {
                        let Some(incoming) = incoming else {
                            let _ = this.update(cx, |page, cx| {
                                page.file_transfer.set_available(false);
                                cx.notify();
                            });
                            return;
                        };
                        if this
                            .update(cx, move |page, cx| {
                                page.enqueue_file_transfer(incoming);
                                cx.notify();
                            })
                            .is_err()
                        {
                            return;
                        }
                    }
                    transfer_status = file_transfer_statuses.recv() => {
                        let Some(transfer_status) = transfer_status else {
                            let _ = this.update(cx, |page, cx| {
                                page.file_transfer.set_available(false);
                                cx.notify();
                            });
                            return;
                        };
                        if this
                            .update(cx, move |page, cx| {
                                page.apply_receive_status(transfer_status);
                                cx.notify();
                            })
                            .is_err()
                        {
                            return;
                        }
                    }
                }
            }
        })
        .detach();
    }

    fn set_section(&mut self, section: Section, cx: &mut Context<Self>) {
        self.section = section;
        self.notice = None;
        cx.notify();
    }

    fn create_pairing_invitation(&mut self, cx: &mut Context<Self>) {
        if self.pairing_busy {
            return;
        }

        if let Some(mut invitation) = self.pairing_invitation.take()
            && !invitation.status().terminal()
        {
            let _ = invitation.cancel();
        }
        self.pairing_generation = self.pairing_generation.wrapping_add(1);
        let generation = self.pairing_generation;
        self.pairing_busy = true;
        self.notice = Some("Creating a protected one-time pairing invitation…".to_owned());
        cx.notify();

        cx.spawn(async move |this, cx| {
            let invitation = match DesktopPairingInvitation::create().await {
                Ok(invitation) => invitation,
                Err(error) => {
                    let _ = this.update(cx, |page, cx| {
                        if page.pairing_generation != generation {
                            return;
                        }
                        page.pairing_busy = false;
                        page.pairing_invitation = None;
                        page.notice = Some(error.to_string());
                        cx.notify();
                    });
                    return;
                }
            };

            let mut status = invitation.subscribe_status();
            if this
                .update(cx, |page, cx| {
                    if page.pairing_generation != generation {
                        return;
                    }
                    page.pairing_busy = false;
                    page.owner.set_product_identity(
                        invitation.owner_id().to_owned(),
                        invitation.local_device_id().to_owned(),
                    );
                    page.pairing_invitation = Some(invitation);
                    page.notice = None;
                    cx.notify();
                })
                .is_err()
            {
                return;
            }

            while status.changed().await.is_ok() {
                let stage = *status.borrow_and_update();
                if this
                    .update(cx, |page, cx| {
                        if page.pairing_generation != generation {
                            return;
                        }
                        page.notice = match stage {
                            DesktopPairingStage::Paired => {
                                #[cfg(target_os = "linux")]
                                page.start_product_presence(cx);
                                Some("Device paired and trust saved on both devices.".to_owned())
                            }
                            DesktopPairingStage::Failed => Some(
                                "Pairing did not complete. Generate a new invitation to retry."
                                    .to_owned(),
                            ),
                            DesktopPairingStage::Expired => {
                                Some("Pairing invitation expired.".to_owned())
                            }
                            DesktopPairingStage::Cancelled => {
                                Some("Pairing invitation cancelled.".to_owned())
                            }
                            _ => None,
                        };
                        cx.notify();
                    })
                    .is_err()
                {
                    return;
                }
            }
        })
        .detach();
    }

    fn cancel_pairing_invitation(&mut self, cx: &mut Context<Self>) {
        let Some(invitation) = self.pairing_invitation.as_mut() else {
            return;
        };

        if invitation.status().terminal() {
            self.pairing_generation = self.pairing_generation.wrapping_add(1);
            self.pairing_invitation = None;
            self.notice = None;
        } else {
            match invitation.cancel() {
                Ok(()) => {
                    self.notice = Some("Cancelling pairing invitation…".to_owned());
                }
                Err(error) => {
                    self.notice = Some(error.to_string());
                }
            }
        }
        cx.notify();
    }

    fn disconnect_peer(&mut self, cx: &mut Context<Self>) {
        #[cfg(target_os = "linux")]
        if let Some(presence) = self.product_presence.as_ref() {
            self.notice = Some(match presence.disconnect() {
                Ok(()) => "Automatic trusted-device connection paused.".to_owned(),
                Err(error) => error.to_string(),
            });
            cx.notify();
            return;
        }

        self.notice = Some(match self.runtime.disconnect_current_peer() {
            Ok(()) => "Disconnect requested.".to_owned(),
            Err(error) => error.to_string(),
        });
        cx.notify();
    }

    #[cfg(target_os = "linux")]
    fn reconnect_peer(&mut self, cx: &mut Context<Self>) {
        let Some(presence) = self.product_presence.as_ref() else {
            self.start_product_presence(cx);
            return;
        };
        self.notice = Some(match presence.reconnect() {
            Ok(()) => "Trusted-device reconnect requested.".to_owned(),
            Err(error) => error.to_string(),
        });
        cx.notify();
    }

    #[cfg(target_os = "linux")]
    fn send_clipboard(&mut self, cx: &mut Context<Self>) {
        if !self.clipboard.begin(ClipboardAction::Send) {
            return;
        }
        let Some(controller) = self.product_presence.as_ref().map(Arc::clone) else {
            self.clipboard
                .finish(ClipboardAction::Send, ClipboardResult::Unavailable);
            cx.notify();
            return;
        };
        let Some(text) = cx.read_from_clipboard().and_then(|item| item.text()) else {
            self.clipboard
                .finish(ClipboardAction::Send, ClipboardResult::Unavailable);
            cx.notify();
            return;
        };

        cx.notify();
        cx.spawn(async move |this, cx| {
            let result = match controller.send_clipboard_text(text).await {
                Ok(()) => ClipboardResult::Success,
                Err(error) => error.into(),
            };
            let _ = this.update(cx, |page, cx| {
                page.clipboard.finish(ClipboardAction::Send, result);
                cx.notify();
            });
        })
        .detach();
    }

    #[cfg(target_os = "linux")]
    fn fetch_clipboard(&mut self, cx: &mut Context<Self>) {
        if !self.clipboard.begin(ClipboardAction::Fetch) {
            return;
        }
        let Some(controller) = self.product_presence.as_ref().map(Arc::clone) else {
            self.clipboard
                .finish(ClipboardAction::Fetch, ClipboardResult::Unavailable);
            cx.notify();
            return;
        };

        cx.notify();
        cx.spawn(
            async move |this, cx| match controller.fetch_clipboard_text().await {
                Ok(text) => {
                    let _ = this.update(cx, move |page, cx| {
                        cx.write_to_clipboard(ClipboardItem::new_string(text));
                        page.clipboard
                            .finish(ClipboardAction::Fetch, ClipboardResult::Success);
                        cx.notify();
                    });
                }
                Err(error) => {
                    let result = ClipboardResult::from(error);
                    let _ = this.update(cx, |page, cx| {
                        page.clipboard.finish(ClipboardAction::Fetch, result);
                        cx.notify();
                    });
                }
            },
        )
        .detach();
    }

    #[cfg(target_os = "linux")]
    fn enqueue_file_transfer(&mut self, incoming: LinuxIncomingFileTransfer) {
        self.pending_file_transfers.push_back(incoming);
        self.surface_next_file_transfer();
    }

    #[cfg(target_os = "linux")]
    fn surface_next_file_transfer(&mut self) {
        if self.file_transfer.receive().active() {
            return;
        }
        if let Some(incoming) = self.pending_file_transfers.front() {
            self.file_transfer.incoming_offer(
                incoming.offer().display_name().to_owned(),
                incoming.offer().file_size(),
            );
        }
    }

    #[cfg(target_os = "linux")]
    fn choose_file_to_send(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.file_transfer_send_dialog_open || self.file_transfer.send().active() {
            return;
        }
        let Some(controller) = self.product_presence.as_ref().map(Arc::clone) else {
            self.file_transfer
                .send_failed(0, 0, FileTransferFailure::NotConnected);
            cx.notify();
            return;
        };

        self.file_transfer_send_dialog_open = true;
        let receiver = cx.prompt_for_paths(PathPromptOptions {
            files: true,
            directories: false,
            multiple: false,
            prompt: Some("Select a file to send".into()),
        });
        let view = cx.entity();

        cx.spawn_in(window, async move |_, window| {
            let path = receiver.await.ok()?.ok()??.into_iter().next();
            view.update_in(window, move |page, _, cx| {
                page.file_transfer_send_dialog_open = false;
                if let Some(path) = path {
                    page.start_file_transfer_send(controller, path, cx);
                } else {
                    cx.notify();
                }
            })
            .ok()
        })
        .detach();
    }

    #[cfg(target_os = "linux")]
    fn start_file_transfer_send(
        &mut self,
        controller: Arc<DesktopProductPresenceController>,
        path: PathBuf,
        cx: &mut Context<Self>,
    ) {
        let display_name = path
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or("Selected file")
            .to_owned();
        self.file_transfer.begin_send(display_name);

        match controller.start_file_transfer_send(path) {
            Ok(handle) => self.track_file_transfer_send(handle, cx),
            Err(error) => {
                self.file_transfer
                    .send_failed(0, 0, FileTransferFailure::Failed);
                self.notice = Some(error.to_string());
                cx.notify();
            }
        }
    }

    #[cfg(target_os = "linux")]
    fn retry_file_transfer_send(&mut self, cx: &mut Context<Self>) {
        let Some(controller) = self.product_presence.as_ref().map(Arc::clone) else {
            return;
        };
        let Some(token) = self.file_transfer_retry.clone() else {
            return;
        };
        let display_name = self
            .file_transfer
            .send()
            .display_name()
            .unwrap_or("Selected file")
            .to_owned();
        self.file_transfer.begin_send(display_name);

        match controller.retry_file_transfer_send(token) {
            Ok(handle) => self.track_file_transfer_send(handle, cx),
            Err(error) => {
                self.file_transfer
                    .send_failed(0, 0, FileTransferFailure::Failed);
                self.notice = Some(error.to_string());
                cx.notify();
            }
        }
    }

    #[cfg(target_os = "linux")]
    fn track_file_transfer_send(
        &mut self,
        handle: LinuxFileTransferSendHandle,
        cx: &mut Context<Self>,
    ) {
        self.file_transfer_send_generation = self.file_transfer_send_generation.wrapping_add(1);
        let generation = self.file_transfer_send_generation;
        self.file_transfer_retry = Some(handle.retry_token());
        let mut status = handle.subscribe_status();
        self.file_transfer_send = Some(handle);
        cx.notify();

        cx.spawn(async move |this, cx| {
            while status.changed().await.is_ok() {
                let next = *status.borrow_and_update();
                if this
                    .update(cx, |page, cx| {
                        if page.file_transfer_send_generation != generation {
                            return;
                        }
                        page.apply_send_status(next);
                        cx.notify();
                    })
                    .is_err()
                {
                    return;
                }
            }
        })
        .detach();
    }

    #[cfg(target_os = "linux")]
    fn cancel_file_transfer_send(&mut self, cx: &mut Context<Self>) {
        if let Some(handle) = self.file_transfer_send.as_ref() {
            handle.cancel();
            self.notice = Some("Cancelling file transfer…".to_owned());
            cx.notify();
        }
    }

    #[cfg(target_os = "linux")]
    fn choose_file_transfer_destination(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.file_transfer_receive_dialog_open {
            return;
        }
        let Some(incoming) = self.pending_file_transfers.front().cloned() else {
            return;
        };
        let Some(controller) = self.product_presence.as_ref().map(Arc::clone) else {
            return;
        };

        let directory = env::var_os("HOME")
            .map(PathBuf::from)
            .unwrap_or_else(|| PathBuf::from("/"));
        let receiver = cx.prompt_for_new_path(&directory, Some(incoming.offer().display_name()));
        self.file_transfer_receive_dialog_open = true;
        let view = cx.entity();

        cx.spawn_in(window, async move |_, window| {
            let path = receiver.await.ok().into_iter().flatten().flatten().next();
            view.update_in(window, move |page, _, cx| {
                page.file_transfer_receive_dialog_open = false;
                if let Some(path) = path {
                    page.accept_file_transfer_destination(controller, incoming, path, cx);
                } else {
                    cx.notify();
                }
            })
            .ok()
        })
        .detach();
    }

    #[cfg(target_os = "linux")]
    fn accept_file_transfer_destination(
        &mut self,
        controller: Arc<DesktopProductPresenceController>,
        incoming: LinuxIncomingFileTransfer,
        path: PathBuf,
        cx: &mut Context<Self>,
    ) {
        let request_id = incoming.request_id();
        if self
            .pending_file_transfers
            .front()
            .is_none_or(|pending| pending.request_id() != request_id)
        {
            cx.notify();
            return;
        }
        self.pending_file_transfers.pop_front();
        self.file_transfer_receive_dialog_open = true;
        cx.notify();

        cx.spawn(async move |this, cx| {
            let result = controller
                .accept_file_transfer_destination(incoming.clone(), path)
                .await;
            let _ = this.update(cx, |page, cx| {
                page.file_transfer_receive_dialog_open = false;
                match result {
                    Ok(()) => {}
                    Err(DesktopPresenceError::FileTransferCancelled) => {
                        page.file_transfer.receive_cancelled(
                            0,
                            page.file_transfer.receive().total_bytes(),
                        );
                        page.notice = None;
                    }
                    Err(error @ DesktopPresenceError::FileTransferConnection) => {
                        page.file_transfer.receive_failed(
                            0,
                            page.file_transfer.receive().total_bytes(),
                            FileTransferFailure::Connection,
                        );
                        page.notice = Some(error.to_string());
                    }
                    Err(error) => {
                        page.pending_file_transfers.push_front(incoming);
                        page.file_transfer.receive_failed(
                            0,
                            page.file_transfer.receive().total_bytes(),
                            FileTransferFailure::Storage,
                        );
                        page.notice = Some(error.to_string());
                    }
                }
                page.surface_next_file_transfer();
                cx.notify();
            });
        })
        .detach();
    }

    #[cfg(target_os = "linux")]
    fn decline_file_transfer_offer(&mut self, cx: &mut Context<Self>) {
        if self.file_transfer_receive_dialog_open {
            return;
        }
        let Some(incoming) = self.pending_file_transfers.front().cloned() else {
            return;
        };
        let Some(controller) = self.product_presence.as_ref().map(Arc::clone) else {
            return;
        };

        let request_id = incoming.request_id();
        if self
            .pending_file_transfers
            .front()
            .is_none_or(|pending| pending.request_id() != request_id)
        {
            return;
        }

        let total_bytes = incoming.offer().file_size();
        self.pending_file_transfers.pop_front();
        self.file_transfer.receive_cancelled(0, total_bytes);
        self.notice = None;
        self.surface_next_file_transfer();
        cx.notify();

        cx.spawn(async move |this, cx| {
            let result = controller.decline_file_transfer_offer(incoming).await;
            let _ = this.update(cx, |page, cx| {
                match result {
                    Ok(()) | Err(DesktopPresenceError::FileTransferCancelled) => {}
                    Err(error) => {
                        page.notice = Some(error.to_string());
                    }
                }
                cx.notify();
            });
        })
        .detach();
    }

    #[cfg(target_os = "linux")]
    fn cancel_file_transfer_receive(&mut self, cx: &mut Context<Self>) {
        let Some(transfer_id) = self.active_receive_transfer else {
            return;
        };
        let Some(controller) = self.product_presence.as_ref().map(Arc::clone) else {
            return;
        };
        self.notice = Some("Cancelling incoming file transfer…".to_owned());
        cx.notify();

        cx.spawn(async move |this, cx| {
            let result = controller.cancel_file_transfer_receive(transfer_id).await;
            let _ = this.update(cx, |page, cx| {
                if let Err(error) = result {
                    page.notice = Some(error.to_string());
                }
                cx.notify();
            });
        })
        .detach();
    }

    #[cfg(target_os = "linux")]
    fn apply_send_status(&mut self, status: LinuxFileTransferSendStatus) {
        match status {
            LinuxFileTransferSendStatus::Preparing => {
                let display_name = self
                    .file_transfer
                    .send()
                    .display_name()
                    .unwrap_or("Selected file")
                    .to_owned();
                self.file_transfer.begin_send(display_name);
            }
            LinuxFileTransferSendStatus::WaitingForPeer { total_bytes } => {
                self.file_transfer.send_waiting(total_bytes);
            }
            LinuxFileTransferSendStatus::Transferring {
                transferred_bytes,
                total_bytes,
            } => self
                .file_transfer
                .send_progress(transferred_bytes, total_bytes),
            LinuxFileTransferSendStatus::Finalizing { total_bytes } => {
                self.file_transfer.send_finalizing(total_bytes);
            }
            LinuxFileTransferSendStatus::Completed { total_bytes } => {
                self.file_transfer.send_completed(total_bytes, false);
                self.notice = None;
            }
            LinuxFileTransferSendStatus::AlreadyComplete { total_bytes } => {
                self.file_transfer.send_completed(total_bytes, true);
                self.notice = None;
            }
            LinuxFileTransferSendStatus::Cancelled {
                transferred_bytes,
                total_bytes,
            } => self
                .file_transfer
                .send_cancelled(transferred_bytes, total_bytes),
            LinuxFileTransferSendStatus::Failed {
                transferred_bytes,
                total_bytes,
                failure,
            } => self.file_transfer.send_failed(
                transferred_bytes,
                total_bytes,
                map_send_failure(failure),
            ),
        }
    }

    #[cfg(target_os = "linux")]
    fn apply_receive_status(&mut self, status: LinuxFileTransferReceiveStatus) {
        match status {
            LinuxFileTransferReceiveStatus::OfferCancelled { request_id } => {
                let was_visible = self
                    .pending_file_transfers
                    .front()
                    .is_some_and(|incoming| incoming.request_id() == request_id);
                self.pending_file_transfers
                    .retain(|incoming| incoming.request_id() != request_id);
                if was_visible
                    && self.file_transfer.receive().stage() == FileTransferStage::AwaitingDestination
                {
                    self.file_transfer.reset_receive();
                    self.notice = None;
                }
                self.surface_next_file_transfer();
            }
            LinuxFileTransferReceiveStatus::Ready {
                transfer_id,
                resume_offset,
                total_bytes,
            } => {
                self.active_receive_transfer = Some(transfer_id);
                self.file_transfer.receive_ready(resume_offset, total_bytes);
            }
            LinuxFileTransferReceiveStatus::Receiving {
                transfer_id,
                received_bytes,
                total_bytes,
            } => {
                self.active_receive_transfer = Some(transfer_id);
                self.file_transfer
                    .receive_progress(received_bytes, total_bytes);
            }
            LinuxFileTransferReceiveStatus::Completed {
                transfer_id,
                total_bytes,
            } => {
                if self.active_receive_transfer == Some(transfer_id) {
                    self.active_receive_transfer = None;
                }
                self.file_transfer.receive_completed(total_bytes, false);
                self.surface_next_file_transfer();
            }
            LinuxFileTransferReceiveStatus::AlreadyComplete {
                transfer_id,
                total_bytes,
            } => {
                if self.active_receive_transfer == Some(transfer_id) {
                    self.active_receive_transfer = None;
                }
                self.file_transfer.receive_completed(total_bytes, true);
                self.surface_next_file_transfer();
            }
            LinuxFileTransferReceiveStatus::Cancelled {
                transfer_id,
                received_bytes,
                total_bytes,
            } => {
                if self.active_receive_transfer == Some(transfer_id) {
                    self.active_receive_transfer = None;
                }
                self.file_transfer
                    .receive_cancelled(received_bytes, total_bytes);
                self.surface_next_file_transfer();
            }
            LinuxFileTransferReceiveStatus::Failed {
                transfer_id,
                received_bytes,
                total_bytes,
                failure,
            } => {
                if self.active_receive_transfer == Some(transfer_id) {
                    self.active_receive_transfer = None;
                }
                self.file_transfer.receive_failed(
                    received_bytes,
                    total_bytes,
                    map_receive_failure(failure),
                );
                self.surface_next_file_transfer();
            }
        }
    }

    fn revoke_peer(&mut self, cx: &mut Context<Self>) {
        self.notice = Some(match self.runtime.revoke_current_peer() {
            Ok(()) => "Revocation requested. Fresh reconnects should now be rejected.".to_owned(),
            Err(error) => error.to_string(),
        });
        cx.notify();
    }
}

impl Render for ControlCenterPage {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = cx.theme();
        let appearance = active_theme(cx);
        let devices_active = self.section == Section::Devices;
        let owner_active = self.section == Section::Owner;

        let navigation = div()
            .flex()
            .flex_col()
            .gap(px(appearance.spacing.sm))
            .child(
                Button::new("nav-devices")
                    .accessibility_label("Devices")
                    .on_click(cx.listener(|this, _, _, cx| {
                        this.set_section(Section::Devices, cx);
                    }))
                    .w_full()
                    .h(px(appearance.metrics.control_height_default))
                    .px(px(appearance.spacing.md))
                    .justify_start()
                    .rounded(px(appearance.radius.sm))
                    .border_1()
                    .border_color(theme.border)
                    .bg(if devices_active {
                        theme.accent
                    } else {
                        theme.popover
                    })
                    .text_color(if devices_active {
                        theme.accent_foreground
                    } else {
                        theme.foreground
                    })
                    .text_size(px(appearance.typography.scales.body.size))
                    .font_weight(font_weight(appearance.typography.scales.body.weight))
                    .hover(|style| style.bg(theme.secondary))
                    .focus_visible(|style| style.border_color(theme.ring))
                    .child("Devices"),
            )
            .child(
                Button::new("nav-owner")
                    .accessibility_label("Owner")
                    .on_click(cx.listener(|this, _, _, cx| {
                        this.set_section(Section::Owner, cx);
                    }))
                    .w_full()
                    .h(px(appearance.metrics.control_height_default))
                    .px(px(appearance.spacing.md))
                    .justify_start()
                    .rounded(px(appearance.radius.sm))
                    .border_1()
                    .border_color(theme.border)
                    .bg(if owner_active {
                        theme.accent
                    } else {
                        theme.popover
                    })
                    .text_color(if owner_active {
                        theme.accent_foreground
                    } else {
                        theme.foreground
                    })
                    .text_size(px(appearance.typography.scales.body.size))
                    .font_weight(font_weight(appearance.typography.scales.body.weight))
                    .hover(|style| style.bg(theme.secondary))
                    .focus_visible(|style| style.border_color(theme.ring))
                    .child("Owner"),
            );

        let mut actions = div()
            .flex()
            .items_center()
            .gap(px(appearance.spacing.sm))
            .child(
                Button::new("add-device")
                    .accessibility_label(match self.pairing_invitation.as_ref() {
                        Some(invitation) if invitation.status().terminal() => {
                            "Create another Add Device invitation"
                        }
                        Some(_) => "Regenerate Add Device invitation",
                        None => "Add device",
                    })
                    .disabled(self.pairing_busy)
                    .on_click(cx.listener(|this, _, _, cx| {
                        this.create_pairing_invitation(cx);
                    }))
                    .h(px(appearance.metrics.control_height_default))
                    .px(px(appearance.spacing.lg))
                    .border_1()
                    .border_color(theme.border)
                    .bg(theme.accent)
                    .text_color(theme.accent_foreground)
                    .focus_visible(|style| style.border_color(theme.ring))
                    .child(if self.pairing_busy {
                        "Preparing…"
                    } else {
                        match self.pairing_invitation.as_ref() {
                            Some(invitation) if invitation.status().terminal() => {
                                "Add another device"
                            }
                            Some(_) => "Regenerate",
                            None => "Add Device",
                        }
                    }),
            );

        if let Some(invitation) = self.pairing_invitation.as_ref() {
            let terminal = invitation.status().terminal();
            actions = actions.child(
                Button::new("cancel-pairing")
                    .accessibility_label(if terminal {
                        "Dismiss pairing status"
                    } else {
                        "Cancel pairing invitation"
                    })
                    .on_click(cx.listener(|this, _, _, cx| {
                        this.cancel_pairing_invitation(cx);
                    }))
                    .h(px(appearance.metrics.control_height_default))
                    .px(px(appearance.spacing.lg))
                    .border_1()
                    .border_color(theme.border)
                    .bg(theme.secondary)
                    .text_color(theme.secondary_foreground)
                    .focus_visible(|style| style.border_color(theme.ring))
                    .child(if terminal { "Dismiss" } else { "Cancel" }),
            );
        }

        #[cfg(target_os = "linux")]
        if self.product_presence.is_some() {
            let paused = self.devices.presence() == PresenceDisplay::Paused;
            actions = actions.child(
                Button::new("presence-control")
                    .accessibility_label(if paused {
                        "Reconnect trusted devices"
                    } else {
                        "Pause automatic trusted-device connection"
                    })
                    .on_click(cx.listener(move |this, _, _, cx| {
                        if paused {
                            this.reconnect_peer(cx);
                        } else {
                            this.disconnect_peer(cx);
                        }
                    }))
                    .h(px(appearance.metrics.control_height_default))
                    .px(px(appearance.spacing.lg))
                    .border_1()
                    .border_color(theme.border)
                    .bg(theme.secondary)
                    .text_color(theme.secondary_foreground)
                    .focus_visible(|style| style.border_color(theme.ring))
                    .child(if paused { "Reconnect" } else { "Disconnect" }),
            );
        }

        #[cfg(feature = "development-provisioning")]
        if let Some(device) = self.devices.current() {
            let revoked = device.trust() == TrustDisplay::Revoked;
            actions = actions
                .child(
                    Button::new("disconnect-peer")
                        .accessibility_label("Disconnect current device")
                        .on_click(cx.listener(|this, _, _, cx| {
                            this.disconnect_peer(cx);
                        }))
                        .h(px(appearance.metrics.control_height_default))
                        .px(px(appearance.spacing.lg))
                        .border_1()
                        .border_color(theme.border)
                        .bg(theme.secondary)
                        .text_color(theme.secondary_foreground)
                        .focus_visible(|style| style.border_color(theme.ring))
                        .child("Disconnect"),
                )
                .child(
                    Button::new("revoke-peer")
                        .accessibility_label("Revoke current device")
                        .disabled(revoked)
                        .on_click(cx.listener(|this, _, _, cx| {
                            this.revoke_peer(cx);
                        }))
                        .h(px(appearance.metrics.control_height_default))
                        .px(px(appearance.spacing.lg))
                        .border_1()
                        .border_color(theme.danger)
                        .bg(theme.danger)
                        .text_color(theme.danger_foreground)
                        .focus_visible(|style| style.border_color(theme.ring))
                        .child("Revoke"),
                );
        }

        #[cfg(target_os = "linux")]
        let clipboard_controls = self.devices.current().map(|device| {
            let send_negotiated = device
                .capability_ids()
                .iter()
                .any(|id| id == "clipboard.write");
            let fetch_negotiated = device
                .capability_ids()
                .iter()
                .any(|id| id == "clipboard.read");
            let session_ready = device.connectivity() == ConnectivityDisplay::Connected
                && device.session() == SessionDisplay::Active;
            let enabled = self.clipboard.available() && session_ready && !self.clipboard.busy();

            let actions = div()
                .flex()
                .items_center()
                .gap(px(appearance.spacing.sm))
                .child(
                    Button::new("clipboard-send")
                        .accessibility_label("Send local clipboard to this device")
                        .disabled(!enabled || !send_negotiated)
                        .on_click(cx.listener(|this, _, _, cx| {
                            this.send_clipboard(cx);
                        }))
                        .h(px(appearance.metrics.control_height_default))
                        .px(px(appearance.spacing.lg))
                        .border_1()
                        .border_color(theme.border)
                        .bg(theme.secondary)
                        .text_color(theme.secondary_foreground)
                        .focus_visible(|style| style.border_color(theme.ring))
                        .child(
                            if self.clipboard.busy()
                                && self.clipboard.action() == Some(ClipboardAction::Send)
                            {
                                "Sending…"
                            } else {
                                "Send clipboard"
                            },
                        ),
                )
                .child(
                    Button::new("clipboard-fetch")
                        .accessibility_label("Fetch this device clipboard")
                        .disabled(!enabled || !fetch_negotiated)
                        .on_click(cx.listener(|this, _, _, cx| {
                            this.fetch_clipboard(cx);
                        }))
                        .h(px(appearance.metrics.control_height_default))
                        .px(px(appearance.spacing.lg))
                        .border_1()
                        .border_color(theme.border)
                        .bg(theme.secondary)
                        .text_color(theme.secondary_foreground)
                        .focus_visible(|style| style.border_color(theme.ring))
                        .child(
                            if self.clipboard.busy()
                                && self.clipboard.action() == Some(ClipboardAction::Fetch)
                            {
                                "Fetching…"
                            } else {
                                "Fetch clipboard"
                            },
                        ),
                );

            clipboard_panel(
                device,
                &self.devices.current_permissions(),
                &self.clipboard,
                actions,
                cx,
            )
        });
        #[cfg(not(target_os = "linux"))]
        let clipboard_controls = None;

        #[cfg(target_os = "linux")]
        let file_transfer_controls = self.devices.current().map(|device| {
            let negotiated = device
                .capability_ids()
                .iter()
                .any(|id| id == "files.transfer");
            let session_ready = device.connectivity() == ConnectivityDisplay::Connected
                && device.session() == SessionDisplay::Active;
            let send_stage = self.file_transfer.send().stage();
            let send_active = self.file_transfer.send().active();
            let send_cancellable = matches!(
                send_stage,
                FileTransferStage::Preparing
                    | FileTransferStage::WaitingForPeer
                    | FileTransferStage::Transferring
            );
            let receive_stage = self.file_transfer.receive().stage();
            let send_can_retry = matches!(
                self.file_transfer.send().stage(),
                FileTransferStage::Cancelled | FileTransferStage::Failed
            ) && self.file_transfer_retry.is_some();
            let enabled = self.file_transfer.available() && session_ready && negotiated;

            let mut transfer_actions = div()
                .flex()
                .items_center()
                .gap(px(appearance.spacing.sm))
                .child(
                    Button::new("file-transfer-send")
                        .accessibility_label("Select a local file to send to this device")
                        .disabled(!enabled || send_active || self.file_transfer_send_dialog_open)
                        .on_click(cx.listener(|this, _, window, cx| {
                            this.choose_file_to_send(window, cx);
                        }))
                        .h(px(appearance.metrics.control_height_default))
                        .px(px(appearance.spacing.lg))
                        .border_1()
                        .border_color(theme.border)
                        .bg(theme.secondary)
                        .text_color(theme.secondary_foreground)
                        .focus_visible(|style| style.border_color(theme.ring))
                        .child("Send file"),
                );

            if send_cancellable {
                transfer_actions = transfer_actions.child(
                    Button::new("file-transfer-cancel-send")
                        .accessibility_label("Cancel the outgoing file transfer")
                        .on_click(cx.listener(|this, _, _, cx| {
                            this.cancel_file_transfer_send(cx);
                        }))
                        .h(px(appearance.metrics.control_height_default))
                        .px(px(appearance.spacing.lg))
                        .border_1()
                        .border_color(theme.border)
                        .bg(theme.secondary)
                        .text_color(theme.secondary_foreground)
                        .focus_visible(|style| style.border_color(theme.ring))
                        .child("Cancel send"),
                );
            } else if send_can_retry {
                transfer_actions = transfer_actions.child(
                    Button::new("file-transfer-retry-send")
                        .accessibility_label("Retry and resume the outgoing file transfer")
                        .disabled(!enabled)
                        .on_click(cx.listener(|this, _, _, cx| {
                            this.retry_file_transfer_send(cx);
                        }))
                        .h(px(appearance.metrics.control_height_default))
                        .px(px(appearance.spacing.lg))
                        .border_1()
                        .border_color(theme.border)
                        .bg(theme.secondary)
                        .text_color(theme.secondary_foreground)
                        .focus_visible(|style| style.border_color(theme.ring))
                        .child("Retry / resume"),
                );
            }

            if receive_stage == FileTransferStage::AwaitingDestination {
                transfer_actions = transfer_actions
                    .child(
                        Button::new("file-transfer-save")
                            .accessibility_label("Choose where to save the incoming file")
                            .disabled(!enabled || self.file_transfer_receive_dialog_open)
                            .on_click(cx.listener(|this, _, window, cx| {
                                this.choose_file_transfer_destination(window, cx);
                            }))
                            .h(px(appearance.metrics.control_height_default))
                            .px(px(appearance.spacing.lg))
                            .border_1()
                            .border_color(theme.border)
                            .bg(theme.accent)
                            .text_color(theme.accent_foreground)
                            .focus_visible(|style| style.border_color(theme.ring))
                            .child("Save as…"),
                    )
                    .child(
                        Button::new("file-transfer-decline")
                            .accessibility_label("Decline the incoming file transfer")
                            .disabled(self.file_transfer_receive_dialog_open)
                            .on_click(cx.listener(|this, _, _, cx| {
                                this.decline_file_transfer_offer(cx);
                            }))
                            .h(px(appearance.metrics.control_height_default))
                            .px(px(appearance.spacing.lg))
                            .border_1()
                            .border_color(theme.border)
                            .bg(theme.secondary)
                            .text_color(theme.secondary_foreground)
                            .focus_visible(|style| style.border_color(theme.ring))
                            .child("Decline"),
                    );
            } else if matches!(
                receive_stage,
                FileTransferStage::Ready | FileTransferStage::Transferring
            ) {
                transfer_actions = transfer_actions.child(
                    Button::new("file-transfer-cancel-receive")
                        .accessibility_label("Cancel the incoming file transfer")
                        .on_click(cx.listener(|this, _, _, cx| {
                            this.cancel_file_transfer_receive(cx);
                        }))
                        .h(px(appearance.metrics.control_height_default))
                        .px(px(appearance.spacing.lg))
                        .border_1()
                        .border_color(theme.border)
                        .bg(theme.secondary)
                        .text_color(theme.secondary_foreground)
                        .focus_visible(|style| style.border_color(theme.ring))
                        .child("Cancel receive"),
                );
            }

            file_transfer_panel(
                device,
                &self.devices.current_permissions(),
                &self.file_transfer,
                transfer_actions,
                cx,
            )
        });
        #[cfg(not(target_os = "linux"))]
        let file_transfer_controls = None;

        let pairing_panel = self
            .pairing_invitation
            .as_ref()
            .map(|invitation| pairing_invitation_panel(invitation, cx));

        let content = match self.section {
            Section::Devices => devices_content(
                &self.devices,
                Some(actions),
                pairing_panel,
                clipboard_controls,
                file_transfer_controls,
                self.notice.as_deref(),
                cx,
            ),
            Section::Owner => owner_content(&self.owner, cx),
        };

        control_center_layout(navigation, content, cx)
    }
}

#[cfg(target_os = "linux")]
fn map_send_failure(failure: LinuxFileTransferSendFailure) -> FileTransferFailure {
    match failure {
        LinuxFileTransferSendFailure::Source => FileTransferFailure::Source,
        LinuxFileTransferSendFailure::NotConnected => FileTransferFailure::NotConnected,
        LinuxFileTransferSendFailure::NotNegotiated => FileTransferFailure::NotNegotiated,
        LinuxFileTransferSendFailure::Denied => FileTransferFailure::Denied,
        LinuxFileTransferSendFailure::ResourceLimit => FileTransferFailure::ResourceLimit,
        LinuxFileTransferSendFailure::TimedOut => FileTransferFailure::TimedOut,
        LinuxFileTransferSendFailure::RemoteIntegrity => FileTransferFailure::Integrity,
        LinuxFileTransferSendFailure::RemoteStorage => FileTransferFailure::Storage,
        LinuxFileTransferSendFailure::Failed => FileTransferFailure::Failed,
    }
}

#[cfg(target_os = "linux")]
fn map_receive_failure(failure: LinuxFileTransferReceiveFailure) -> FileTransferFailure {
    match failure {
        LinuxFileTransferReceiveFailure::Connection => FileTransferFailure::Connection,
        LinuxFileTransferReceiveFailure::Integrity => FileTransferFailure::Integrity,
        LinuxFileTransferReceiveFailure::Storage => FileTransferFailure::Storage,
    }
}
