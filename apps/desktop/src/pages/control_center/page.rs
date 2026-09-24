#[cfg(target_os = "linux")]
use crate::features::devices::DesktopProductPresenceController;
#[cfg(feature = "development-provisioning")]
use crate::features::devices::TrustDisplay;
use crate::{
    features::{
        appearance::{active_theme, font_weight},
        devices::{DesktopRuntimeController, DevicesFeatureState, PresenceDisplay},
        owner::OwnerFeatureState,
        pairing::{DesktopPairingInvitation, DesktopPairingStage, load_existing_product_identity},
    },
    pages::control_center::{
        _components::{devices_content, owner_content, pairing_invitation_panel},
        layout::control_center_layout,
    },
};

use gpui_kit::{
    Context, InteractiveElement as _, IntoElement, ParentElement as _, Render, Styled as _, Window,
    base::Button, component::theme::ActiveTheme as _, div, px,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Section {
    Devices,
    Owner,
}

pub struct ControlCenterPage {
    section: Section,
    devices: DevicesFeatureState,
    owner: OwnerFeatureState,
    runtime: DesktopRuntimeController,
    #[cfg(target_os = "linux")]
    product_presence: Option<DesktopProductPresenceController>,
    #[cfg(target_os = "linux")]
    presence_starting: bool,
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
            owner,
            runtime,
            #[cfg(target_os = "linux")]
            product_presence: None,
            #[cfg(target_os = "linux")]
            presence_starting: false,
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
                        cx.notify();
                    });
                    return;
                }
                Err(error) => {
                    let _ = this.update(cx, |page, cx| {
                        page.presence_starting = false;
                        page.notice = Some(error.to_string());
                        cx.notify();
                    });
                    return;
                }
            };

            let mut status = controller.subscribe_status();
            let mut permissions = controller.subscribe_permissions();
            let initial = status.borrow().clone();
            let initial_permissions = permissions.borrow().clone();
            if this
                .update(cx, |page, cx| {
                    page.presence_starting = false;
                    page.devices.update_presence(&initial);
                    page.devices.update_permissions(&initial_permissions);
                    if let Some(runtime) = initial.runtime() {
                        page.owner.update_runtime(runtime);
                    }
                    page.product_presence = Some(controller);
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

        let pairing_panel = self
            .pairing_invitation
            .as_ref()
            .map(|invitation| pairing_invitation_panel(invitation, cx));

        let content = match self.section {
            Section::Devices => devices_content(
                &self.devices,
                Some(actions),
                pairing_panel,
                self.notice.as_deref(),
                cx,
            ),
            Section::Owner => owner_content(&self.owner, cx),
        };

        control_center_layout(navigation, content, cx)
    }
}
