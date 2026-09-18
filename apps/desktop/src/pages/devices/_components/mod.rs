use crate::{
    components::ui::{StatusTone, status_badge},
    features::{
        appearance::{active_theme, font_weight},
        devices::{ConnectivityDisplay, DevicePresentation, DevicesFeatureState, TrustDisplay},
    },
};
use gpui_kit::{
    App, Div, ParentElement as _, Styled as _, component::theme::ActiveTheme as _, div, px,
};

pub(super) fn device_content(state: &DevicesFeatureState, cx: &App) -> Div {
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
                        .child("Devices"),
                )
                .child(
                    div()
                        .text_size(px(appearance.typography.scales.caption.size))
                        .text_color(theme.muted_foreground)
                        .child("Authenticated local device connection status"),
                ),
        )
        .child(match state.current() {
            Some(device) => device_panel(device, cx),
            None => empty_state(cx),
        })
}

fn empty_state(cx: &App) -> Div {
    let theme = cx.theme();
    let appearance = active_theme(cx);

    div()
        .flex()
        .flex_col()
        .rounded(px(appearance.radius.md))
        .border_1()
        .border_color(theme.border)
        .bg(theme.popover)
        .p(px(appearance.spacing.xl))
        .gap(px(appearance.spacing.lg))
        .child(
            div()
                .flex()
                .items_center()
                .gap(px(appearance.spacing.md))
                .child(status_badge("Disconnected", StatusTone::Neutral, cx))
                .child(
                    div()
                        .text_size(px(appearance.typography.scales.body.size))
                        .text_color(theme.muted_foreground)
                        .child("No active authenticated device session"),
                ),
        )
        .child(
            div()
                .text_size(px(appearance.typography.scales.caption.size))
                .text_color(theme.muted_foreground)
                .child("Runtime wiring arrives in M10 Task 9; this shell does not own networking."),
        )
}

fn device_panel(device: &DevicePresentation, cx: &App) -> Div {
    let theme = cx.theme();
    let appearance = active_theme(cx);
    let trust_tone = match device.trust() {
        TrustDisplay::Trusted => StatusTone::Accent,
        TrustDisplay::Pending => StatusTone::Neutral,
        TrustDisplay::Revoked => StatusTone::Critical,
    };
    let connectivity_tone = match device.connectivity() {
        ConnectivityDisplay::Connected => StatusTone::Accent,
        ConnectivityDisplay::Disconnected => StatusTone::Neutral,
    };

    div()
        .flex()
        .flex_col()
        .rounded(px(appearance.radius.md))
        .border_1()
        .border_color(theme.border)
        .bg(theme.popover)
        .child(
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
                        .flex()
                        .flex_col()
                        .gap(px(appearance.spacing.xxs))
                        .child(
                            div()
                                .text_size(px(appearance.typography.scales.label.size))
                                .font_weight(font_weight(appearance.typography.scales.label.weight))
                                .child(device.peer_id().unwrap_or("Unknown peer").to_owned()),
                        )
                        .child(
                            div()
                                .text_size(px(appearance.typography.scales.caption.size))
                                .text_color(theme.muted_foreground)
                                .child("Cross-Lab device"),
                        ),
                )
                .child(
                    div()
                        .flex()
                        .items_center()
                        .gap(px(appearance.spacing.sm))
                        .child(status_badge(
                            device.connectivity().label(),
                            connectivity_tone,
                            cx,
                        ))
                        .child(status_badge(device.trust().label(), trust_tone, cx)),
                ),
        )
        .child(detail_row("Session", device.session().label(), cx))
        .child(detail_row(
            "Protocol",
            device.protocol().unwrap_or("Unavailable"),
            cx,
        ))
        .child(detail_row("Network", device.network().label(), cx))
        .child(detail_row("Security", device.security().label(), cx))
        .child(detail_row(
            "Metered",
            match device.metered() {
                Some(true) => "Yes",
                Some(false) => "No",
                None => "Unknown",
            },
            cx,
        ))
        .child(detail_row(
            "Capabilities",
            &device.capability_count().to_string(),
            cx,
        ))
}

fn detail_row(label: &str, value: &str, cx: &App) -> Div {
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
