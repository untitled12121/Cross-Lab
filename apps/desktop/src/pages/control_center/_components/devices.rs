use crate::{
    components::ui::{StatusTone, status_badge},
    features::{
        appearance::{active_theme, font_weight},
        devices::{
            ConnectivityDisplay, DevicePresentation, DevicesFeatureState, PermissionPresentation,
            SavedTrustedPeerPresentation, TrustDisplay, VerifiedSessionDisplay,
        },
    },
};
use gpui_kit::{
    App, Div, ParentElement as _, Styled as _, component::theme::ActiveTheme as _, div, px,
};

pub(crate) struct DevicesContentSlots<'a> {
    pub actions: Option<Div>,
    pub pairing: Option<Div>,
    pub clipboard: Option<Div>,
    pub file_transfer: Option<Div>,
    pub notice: Option<&'a str>,
}

pub(crate) fn devices_content(
    state: &DevicesFeatureState,
    saved_peers: &[SavedTrustedPeerPresentation],
    additional_saved_peers: usize,
    slots: DevicesContentSlots<'_>,
    cx: &App,
) -> Div {
    let DevicesContentSlots {
        actions,
        pairing,
        clipboard,
        file_transfer,
        notice,
    } = slots;
    let theme = cx.theme();
    let appearance = active_theme(cx);

    let mut header = div()
        .flex()
        .items_start()
        .justify_between()
        .gap(px(appearance.spacing.lg))
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
                        .child("Authenticated devices in the current Cross-Lab runtime"),
                ),
        );
    if let Some(actions) = actions {
        header = header.child(actions);
    }

    let mut content = div()
        .flex()
        .flex_col()
        .size_full()
        .p(px(appearance.spacing.xl))
        .gap(px(appearance.spacing.xl))
        .child(header);

    if let Some(notice) = notice {
        content = content.child(
            div()
                .border_1()
                .border_color(theme.border)
                .bg(theme.muted)
                .px(px(appearance.spacing.lg))
                .py(px(appearance.spacing.md))
                .text_size(px(appearance.typography.scales.caption.size))
                .text_color(theme.muted_foreground)
                .child(notice.to_owned()),
        );
    }

    if let Some(pairing) = pairing {
        content = content.child(pairing);
    }

    content = content.child(match state.current() {
        Some(device) => device_panel(device, state, clipboard, file_transfer, cx),
        None => empty_state(state.verified_session_display(), cx),
    });

    if !saved_peers.is_empty() {
        content = content.child(saved_trusted_devices_panel(
            saved_peers,
            additional_saved_peers,
            cx,
        ));
    }
    content
}

fn saved_trusted_devices_panel(
    peers: &[SavedTrustedPeerPresentation],
    additional_count: usize,
    cx: &App,
) -> Div {
    let theme = cx.theme();
    let appearance = active_theme(cx);
    let mut panel = div()
        .flex()
        .flex_col()
        .border_1()
        .border_color(theme.border)
        .bg(theme.popover)
        .p(px(appearance.spacing.lg))
        .gap(px(appearance.spacing.md))
        .child(
            div()
                .text_size(px(appearance.typography.scales.label.size))
                .font_weight(font_weight(appearance.typography.scales.label.weight))
                .child("Saved trusted devices"),
        )
        .child(
            div()
                .text_size(px(appearance.typography.scales.caption.size))
                .text_color(theme.muted_foreground)
                .child("Stored trust does not imply online presence; only a fresh authenticated session is active."),
        );

    for (index, peer) in peers.iter().enumerate() {
        panel = panel.child(detail_row(
            &format!("Device {} · {}", index + 1, peer.short_id()),
            if peer.in_authenticated_session() {
                "Session active"
            } else {
                "Not in session"
            },
            cx,
        ));
    }

    if additional_count > 0 {
        panel = panel.child(
            div()
                .text_size(px(appearance.typography.scales.caption.size))
                .text_color(theme.muted_foreground)
                .child(format!("{additional_count} more saved devices not shown")),
        );
    }

    panel
}

fn empty_state(presence: VerifiedSessionDisplay, cx: &App) -> Div {
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
                .child(status_badge(
                    presence.label(),
                    verified_tone(presence),
                    cx,
                ))
                .child(
                    div()
                        .text_size(px(appearance.typography.scales.body.size))
                        .text_color(theme.muted_foreground)
                        .child(presence.detail()),
                ),
        )
        .child(
            div()
                .text_size(px(appearance.typography.scales.caption.size))
                .text_color(theme.muted_foreground)
                .child(
                    "Cross-Lab has no mandatory cloud account. Start an authenticated local                      device session to manage the peer here.",
                ),
        )
}

fn device_panel(
    device: &DevicePresentation,
    state: &DevicesFeatureState,
    clipboard: Option<Div>,
    file_transfer: Option<Div>,
    cx: &App,
) -> Div {
    let theme = cx.theme();
    let presence = state.verified_session_display();
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

    let mut panel = div()
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
                                .child(if state.has_authenticated_session() {
                                    "Authenticated Cross-Lab peer"
                                } else {
                                    "Peer presence does not grant session authority"
                                }),
                        ),
                )
                .child(
                    div()
                        .flex()
                        .items_center()
                        .gap(px(appearance.spacing.sm))
                        .child(status_badge(presence.label(), verified_tone(presence), cx))
                        .child(status_badge(
                            device.connectivity().label(),
                            connectivity_tone,
                            cx,
                        ))
                        .child(status_badge(device.trust().label(), trust_tone, cx)),
                ),
        )
        .child(detail_row(
            "Owner",
            device.owner_id().unwrap_or("Unavailable"),
            cx,
        ))
        .child(detail_row(
            "This device",
            device.local_device_id().unwrap_or("Unavailable"),
            cx,
        ))
        .child(detail_row(
            "Peer device",
            device.peer_id().unwrap_or("Unavailable"),
            cx,
        ))
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
        .child(permission_panel(
            device,
            &state.current_permissions(),
            state.policy_revision(),
            state.has_authenticated_session(),
            cx,
        ));

    if let Some(clipboard) = clipboard {
        panel = panel.child(clipboard);
    }
    if let Some(file_transfer) = file_transfer {
        panel = panel.child(file_transfer);
    }

    panel
}

fn permission_panel(
    device: &DevicePresentation,
    permissions: &[PermissionPresentation],
    policy_revision: u64,
    authenticated: bool,
    cx: &App,
) -> Div {
    let theme = cx.theme();
    let appearance = active_theme(cx);
    let mut content = div()
        .flex()
        .flex_col()
        .border_1()
        .border_color(theme.border)
        .p(px(appearance.spacing.xl))
        .gap(px(appearance.spacing.md))
        .child(
            div()
                .text_size(px(appearance.typography.scales.label.size))
                .font_weight(font_weight(appearance.typography.scales.label.weight))
                .child("Permissions"),
        )
        .child(
            div()
                .text_size(px(appearance.typography.scales.caption.size))
                .text_color(theme.muted_foreground)
                .child(
                    "Capability support does not grant permission. Operations without an exact rule are denied.",
                ),
        );

    if device.capability_ids().is_empty() {
        content = content.child(
            div()
                .text_size(px(appearance.typography.scales.body.size))
                .text_color(theme.muted_foreground)
                .child(if authenticated {
                    "No capabilities negotiated in this authenticated session."
                } else {
                    "No authenticated session. Peer capabilities are unavailable."
                }),
        );
    } else {
        for capability_id in device.capability_ids() {
            let matching = permissions
                .iter()
                .filter(|rule| rule.capability_id() == capability_id)
                .map(|rule| format!("{}: {}", rule.operation(), rule.effect().label()))
                .collect::<Vec<_>>();
            let value = if matching.is_empty() {
                "Default deny".to_owned()
            } else {
                matching.join(" · ")
            };
            content = content.child(detail_row(capability_id, &value, cx));
        }
    }

    content.child(
        div()
            .text_size(px(appearance.typography.scales.caption.size))
            .text_color(theme.muted_foreground)
            .child(format!("Policy revision {policy_revision}")),
    )
}

fn verified_tone(presence: VerifiedSessionDisplay) -> StatusTone {
    match presence {
        VerifiedSessionDisplay::Authenticated => StatusTone::Accent,
        VerifiedSessionDisplay::Failed
        | VerifiedSessionDisplay::Unverified
        | VerifiedSessionDisplay::Revoked => StatusTone::Critical,
        VerifiedSessionDisplay::Unavailable
        | VerifiedSessionDisplay::Discovering
        | VerifiedSessionDisplay::Connecting
        | VerifiedSessionDisplay::Reconnecting
        | VerifiedSessionDisplay::Paused => StatusTone::Neutral,
    }
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
