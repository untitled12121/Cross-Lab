use crate::{
    features::{
        appearance::{active_theme, font_weight},
        devices::{DesktopRuntimeController, DevicesFeatureState, TrustDisplay},
        owner::OwnerFeatureState,
    },
    pages::control_center::{
        _components::{devices_content, owner_content},
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

        Self {
            section: Section::Devices,
            devices,
            owner,
            runtime,
            notice: None,
        }
    }

    fn set_section(&mut self, section: Section, cx: &mut Context<Self>) {
        self.section = section;
        self.notice = None;
        cx.notify();
    }

    fn disconnect_peer(&mut self, cx: &mut Context<Self>) {
        self.notice = Some(match self.runtime.disconnect_current_peer() {
            Ok(()) => "Disconnect requested.".to_owned(),
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

        #[cfg(feature = "development-provisioning")]
        let actions = self.devices.current().map(|device| {
            let revoked = device.trust() == TrustDisplay::Revoked;
            div()
                .flex()
                .items_center()
                .gap(px(appearance.spacing.sm))
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
                )
        });

        #[cfg(not(feature = "development-provisioning"))]
        let actions = None;

        let content = match self.section {
            Section::Devices => devices_content(&self.devices, actions, self.notice.as_deref(), cx),
            Section::Owner => owner_content(&self.owner, cx),
        };

        control_center_layout(navigation, content, cx)
    }
}
