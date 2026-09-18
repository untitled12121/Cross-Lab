use crate::{
    components::ui::{StatusTone, status_badge},
    features::devices::{
        ConnectivityDisplay, DevicePresentation, DevicesFeatureState, TrustDisplay,
    },
};
use gpui_kit::{
    App, Div, ParentElement as _, Styled as _, component::theme::ActiveTheme as _, div, px,
};

pub(super) fn device_content(state: &DevicesFeatureState, cx: &App) -> Div {
    let theme = cx.theme();

    div()
        .flex()
        .flex_col()
        .size_full()
        .p(px(24.))
        .gap(px(20.))
        .child(
            div()
                .flex()
                .flex_col()
                .gap(px(4.))
                .child(div().text_size(px(18.)).font_semibold().child("Devices"))
                .child(
                    div()
                        .text_size(px(12.))
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

    div()
        .flex()
        .flex_col()
        .border_1()
        .border_color(theme.border)
        .bg(theme.popover)
        .p(px(16.))
        .gap(px(10.))
        .child(
            div()
                .flex()
                .items_center()
                .gap(px(8.))
                .child(status_badge("Disconnected", StatusTone::Neutral, cx))
                .child(
                    div()
                        .text_size(px(12.))
                        .text_color(theme.muted_foreground)
                        .child("No active authenticated device session"),
                ),
        )
        .child(
            div()
                .text_size(px(12.))
                .text_color(theme.muted_foreground)
                .child("Runtime wiring arrives in M10 Task 9; this shell does not own networking."),
        )
}

fn device_panel(device: &DevicePresentation, cx: &App) -> Div {
    let theme = cx.theme();
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
        .border_1()
        .border_color(theme.border)
        .bg(theme.popover)
        .child(
            div()
                .flex()
                .items_center()
                .justify_between()
                .min_h(px(48.))
                .px(px(16.))
                .border_b_1()
                .border_color(theme.border)
                .child(
                    div()
                        .flex()
                        .flex_col()
                        .gap(px(2.))
                        .child(
                            div()
                                .text_size(px(13.))
                                .font_semibold()
                                .child(device.peer_id().unwrap_or("Unknown peer")),
                        )
                        .child(
                            div()
                                .text_size(px(11.))
                                .text_color(theme.muted_foreground)
                                .child("Cross-Lab device"),
                        ),
                )
                .child(
                    div()
                        .flex()
                        .items_center()
                        .gap(px(6.))
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

    div()
        .flex()
        .items_center()
        .justify_between()
        .min_h(px(36.))
        .px(px(16.))
        .border_b_1()
        .border_color(theme.border)
        .child(
            div()
                .text_size(px(11.))
                .text_color(theme.muted_foreground)
                .child(label.to_owned()),
        )
        .child(div().text_size(px(12.)).child(value.to_owned()))
}
