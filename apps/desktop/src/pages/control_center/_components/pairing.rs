use crate::features::{
    appearance::{active_theme, font_weight},
    pairing::{DesktopPairingInvitation, DesktopPairingStage},
};
use gpui_kit::{
    App, Div, ParentElement as _, Styled as _, component::theme::ActiveTheme as _, div, px, rgb,
};

const MODULE_SIZE: f32 = 4.;
const QUIET_ZONE_MODULES: f32 = 4.;

pub(crate) fn pairing_invitation_panel(invitation: &DesktopPairingInvitation, cx: &App) -> Div {
    let theme = cx.theme();
    let appearance = active_theme(cx);
    let status = invitation.status();
    let modules = invitation.modules();
    let width = modules.width();

    let qr = div()
        .flex()
        .flex_col()
        .flex_shrink_0()
        .bg(rgb(0xffffff))
        .p(px(MODULE_SIZE * QUIET_ZONE_MODULES))
        .children((0..width).map(|y| {
            div().flex().flex_shrink_0().children((0..width).map(|x| {
                div()
                    .flex_shrink_0()
                    .size(px(MODULE_SIZE))
                    .bg(if modules.is_dark(x, y) {
                        rgb(0x000000)
                    } else {
                        rgb(0xffffff)
                    })
            }))
        }));

    let status_panel = div()
        .flex()
        .flex_col()
        .gap(px(appearance.spacing.xs))
        .child(
            div()
                .text_size(px(appearance.typography.scales.label.size))
                .font_weight(font_weight(appearance.typography.scales.label.weight))
                .child(status.label()),
        )
        .child(
            div()
                .text_size(px(appearance.typography.scales.caption.size))
                .text_color(if status == DesktopPairingStage::Failed {
                    theme.danger
                } else {
                    theme.muted_foreground
                })
                .child(status.message()),
        );

    div()
        .flex()
        .flex_col()
        .border_1()
        .border_color(theme.border)
        .bg(theme.popover)
        .p(px(appearance.spacing.xl))
        .gap(px(appearance.spacing.lg))
        .child(
            div()
                .flex()
                .flex_col()
                .gap(px(appearance.spacing.xs))
                .child(
                    div()
                        .text_size(px(appearance.typography.scales.label.size))
                        .font_weight(font_weight(appearance.typography.scales.label.weight))
                        .child("Add Device"),
                )
                .child(
                    div()
                        .text_size(px(appearance.typography.scales.caption.size))
                        .text_color(theme.muted_foreground)
                        .child(
                            "Scan this QR from the Cross-Lab app on the device you want to add.                              The invitation is single-use and expires after five minutes.",
                        ),
                ),
        )
        .child(
            div()
                .flex()
                .items_start()
                .gap(px(appearance.spacing.xl))
                .when(status == DesktopPairingStage::Waiting, |panel| panel.child(qr))
                .child(
                    div()
                        .flex()
                        .flex_col()
                        .gap(px(appearance.spacing.md))
                        .child(status_panel)
                        .child(info("Owner", invitation.owner_id(), cx))
                        .child(info("This device", invitation.local_device_id(), cx))
                        .child(info(
                            "Trust",
                            if status == DesktopPairingStage::Paired {
                                "Saved on both devices"
                            } else {
                                "Not granted until pairing completes"
                            },
                            cx,
                        ))
                        .child(info("Transport", "Route discovery is separate from identity", cx)),
                ),
        )
        .child(
            div()
                .text_size(px(appearance.typography.scales.caption.size))
                .text_color(theme.muted_foreground)
                .child(
                    "Treat this QR like a temporary credential. Cross-Lab never logs the pairing                      secret and does not encode IP addresses, hostnames, BLE identifiers, or TLS                      identity into the trust bootstrap.",
                ),
        )
}

fn info(label: &str, value: &str, cx: &App) -> Div {
    let theme = cx.theme();
    let appearance = active_theme(cx);

    div()
        .flex()
        .flex_col()
        .gap(px(appearance.spacing.xxs))
        .child(
            div()
                .text_size(px(appearance.typography.scales.caption.size))
                .text_color(theme.muted_foreground)
                .child(label.to_owned()),
        )
        .child(
            div()
                .text_size(px(appearance.typography.scales.body.size))
                .font_weight(font_weight(appearance.typography.scales.body.weight))
                .child(value.to_owned()),
        )
}
