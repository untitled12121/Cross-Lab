use crate::features::{
    appearance::{active_theme, font_weight},
    owner::OwnerFeatureState,
};
use gpui_kit::{
    App, Div, ParentElement as _, Styled as _, component::theme::ActiveTheme as _, div, px,
};

pub(crate) fn owner_content(state: &OwnerFeatureState, cx: &App) -> Div {
    let theme = cx.theme();
    let appearance = active_theme(cx);

    div()
        .flex()
        .flex_col()
        .size_full()
        .p(px(appearance.spacing.xl))
        .gap(px(appearance.spacing.xl))
        .child(
            div()
                .flex()
                .flex_col()
                .gap(px(appearance.spacing.xs))
                .child(
                    div()
                        .text_size(px(appearance.typography.scales.title.size))
                        .font_weight(font_weight(appearance.typography.scales.title.weight))
                        .child("Owner"),
                )
                .child(
                    div()
                        .text_size(px(appearance.typography.scales.caption.size))
                        .text_color(theme.muted_foreground)
                        .child("Local-first owner identity and this device"),
                ),
        )
        .child(
            div()
                .flex()
                .flex_col()
                .border_1()
                .border_color(theme.border)
                .bg(theme.popover)
                .child(info_row(
                    "Account model",
                    "Owner-controlled local identity",
                    cx,
                ))
                .child(info_row("Cloud sign-in", "Not required", cx))
                .child(match state.current() {
                    Some(owner) => info_row("Owner ID", owner.owner_id(), cx),
                    None => info_row("Owner ID", "Available after authentication", cx),
                })
                .child(match state.current() {
                    Some(owner) => info_row("This device", owner.local_device_id(), cx),
                    None => info_row("This device", "Available after authentication", cx),
                }),
        )
        .child(
            div()
                .border_1()
                .border_color(theme.border)
                .bg(theme.muted)
                .p(px(appearance.spacing.lg))
                .text_size(px(appearance.typography.scales.caption.size))
                .text_color(theme.muted_foreground)
                .child(
                    "Cross-Lab does not require an email/password account or vendor cloud.                      Production persistent authority storage and recovery UI remain separate                      platform-security work; development provisioning is temporary test identity.",
                ),
        )
}

fn info_row(label: &str, value: &str, cx: &App) -> Div {
    let theme = cx.theme();
    let appearance = active_theme(cx);

    div()
        .flex()
        .items_center()
        .justify_between()
        .min_h(px(appearance.metrics.row_height))
        .px(px(appearance.spacing.xl))
        .border_b_1()
        .border_color(theme.border)
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
