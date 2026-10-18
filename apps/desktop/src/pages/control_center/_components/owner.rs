use crosslab_agent::{NotificationInboxSnapshot, NotificationInboxStatus};
use crosslab_protocol::NotificationPayload;

use crate::features::{
    appearance::{active_theme, font_weight},
    owner::OwnerFeatureState,
};
use gpui_kit::{
    App, Div, ParentElement as _, Styled as _, component::theme::ActiveTheme as _, div, px,
};

pub(crate) fn owner_content(
    state: &OwnerFeatureState,
    controls: Option<Div>,
    notice: Option<&str>,
    notifications: &NotificationInboxSnapshot,
    cx: &App,
) -> Div {
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
                })
                .child(info_row(
                    "Paired devices",
                    &state.trusted_peer_ids().len().to_string(),
                    cx,
                ))
                .children(
                    state.trusted_peer_ids().iter().take(24).enumerate().map(|(index, id)| {
                        info_row(&format!("Trusted device {}", index + 1), id, cx)
                    }),
                )
                .child(info_row(
                    "Revoked devices",
                    &state.revoked_peer_ids().len().to_string(),
                    cx,
                ))
                .children(
                    state.revoked_peer_ids().iter().take(24).enumerate().map(|(index, id)| {
                        info_row(&format!("Revoked device {}", index + 1), id, cx)
                    }),
                ),
        )
        .child(controls.unwrap_or_else(div))
        .child(
            div()
                .flex()
                .flex_col()
                .border_1()
                .border_color(theme.border)
                .child(info_row(
                    "Android notifications",
                    match notifications.phase() {
                        NotificationInboxStatus::Idle => "Not subscribed",
                        NotificationInboxStatus::AwaitingApproval => "Awaiting Android",
                        NotificationInboxStatus::Active => "Active",
                        NotificationInboxStatus::Denied => "Not authorized",
                        NotificationInboxStatus::TimedOut => "Subscription timed out",
                    },
                    cx,
                ))
                .child(info_row(
                    "Visible notifications",
                    &notifications.entries().len().to_string(),
                    cx,
                ))
                .child(info_row(
                    "Older notifications skipped",
                    &notifications.skipped_count().to_string(),
                    cx,
                ))
                .children(
                    notifications
                        .entries()
                        .iter()
                        .rev()
                        .take(8)
                        .filter_map(|notification| match notification {
                            NotificationPayload::Posted(posted) => {
                                let label = posted.title().unwrap_or(posted.app_label());
                                let preview = posted.preview().unwrap_or(if posted.redacted() {
                                    "Content hidden by owner preference"
                                } else {
                                    "No preview"
                                });
                                Some(info_row(label, preview, cx))
                            }
                            NotificationPayload::Removed(_) => None,
                        }),
                ),
        )
        .child(
            div()
                .flex()
                .flex_col()
                .border_1()
                .border_color(theme.border)
                .child(info_row(
                    "Audit events",
                    &state.audit_rows().len().to_string(),
                    cx,
                ))
                .child(info_row(
                    "Older events dropped",
                    &state.audit_dropped().to_string(),
                    cx,
                ))
                .child(info_row(
                    "Unqueued events",
                    &state.audit_queue_dropped().to_string(),
                    cx,
                ))
                .children(
                    state
                        .audit_rows()
                        .iter()
                        .rev()
                        .take(24)
                        .enumerate()
                        .map(|(index, row)| info_row(&format!("Event {}", index + 1), row, cx)),
                )
                .child(match state.audit_notice() {
                    Some(notice) => info_row("History status", notice, cx),
                    None => div(),
                }),
        )
        .child(match notice {
            Some(value) => div()
                .border_1()
                .border_color(theme.border)
                .p(px(appearance.spacing.md))
                .child(value.to_owned()),
            None => div(),
        })
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
