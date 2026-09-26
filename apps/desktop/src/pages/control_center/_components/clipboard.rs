use crate::{
    components::ui::{StatusTone, status_badge},
    features::{
        appearance::{active_theme, font_weight},
        clipboard::{ClipboardAction, ClipboardFeatureState, ClipboardResult},
        devices::{DevicePresentation, PermissionPresentation},
    },
};
use gpui_kit::{
    App, Div, ParentElement as _, Styled as _, component::theme::ActiveTheme as _, div, px,
};

const CLIPBOARD_READ: &str = "clipboard.read";
const CLIPBOARD_WRITE: &str = "clipboard.write";
const OP_GET: &str = "get";
const OP_SET: &str = "set";

pub(crate) fn clipboard_panel(
    device: &DevicePresentation,
    permissions: &[PermissionPresentation],
    state: &ClipboardFeatureState,
    actions: Div,
    cx: &App,
) -> Div {
    let theme = cx.theme();
    let appearance = active_theme(cx);
    let mut panel = div()
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
                .child("Clipboard"),
        )
        .child(
            div()
                .text_size(px(appearance.typography.scales.caption.size))
                .text_color(theme.muted_foreground)
                .child("Text only · explicit actions · no clipboard preview or history"),
        )
        .child(detail_row(
            "Peer may read this clipboard",
            permission_label(permissions, CLIPBOARD_READ, OP_GET),
            cx,
        ))
        .child(detail_row(
            "Peer may write this clipboard",
            permission_label(permissions, CLIPBOARD_WRITE, OP_SET),
            cx,
        ))
        .child(actions);

    if let Some((label, tone)) = status(state) {
        panel = panel.child(status_badge(label, tone, cx));
    } else if !state.available() {
        panel = panel.child(
            div()
                .text_size(px(appearance.typography.scales.caption.size))
                .text_color(theme.muted_foreground)
                .child("Clipboard actions are unavailable in this runtime."),
        );
    } else {
        let send_negotiated = device
            .capability_ids()
            .iter()
            .any(|id| id == CLIPBOARD_WRITE);
        let fetch_negotiated = device
            .capability_ids()
            .iter()
            .any(|id| id == CLIPBOARD_READ);
        let note = match (send_negotiated, fetch_negotiated) {
            (false, false) => Some("This session did not negotiate clipboard v1."),
            (false, true) => {
                Some("Sending is unavailable because clipboard.write is not negotiated.")
            }
            (true, false) => {
                Some("Fetching is unavailable because clipboard.read is not negotiated.")
            }
            (true, true) => None,
        };
        if let Some(note) = note {
            panel = panel.child(
                div()
                    .text_size(px(appearance.typography.scales.caption.size))
                    .text_color(theme.muted_foreground)
                    .child(note),
            );
        }
    }

    panel
}

fn permission_label<'a>(
    permissions: &'a [PermissionPresentation],
    capability_id: &str,
    operation: &str,
) -> &'a str {
    permissions
        .iter()
        .find(|rule| rule.capability_id() == capability_id && rule.operation() == operation)
        .map(|rule| rule.effect().label())
        .unwrap_or("Default deny")
}

fn status(state: &ClipboardFeatureState) -> Option<(&'static str, StatusTone)> {
    if state.busy() {
        return Some((
            match state.action() {
                Some(ClipboardAction::Send) => "Sending clipboard",
                Some(ClipboardAction::Fetch) => "Fetching clipboard",
                None => "Clipboard operation in progress",
            },
            StatusTone::Accent,
        ));
    }

    let result = state.result()?;
    let label = match result {
        ClipboardResult::Success => match state.action() {
            Some(ClipboardAction::Send) => "Clipboard sent",
            Some(ClipboardAction::Fetch) => "Clipboard fetched to this device",
            None => "Clipboard operation complete",
        },
        ClipboardResult::Unavailable => "Text clipboard is unavailable",
        ClipboardResult::NotConnected => "Clipboard peer is not connected",
        ClipboardResult::NotNegotiated => "Clipboard operation is not negotiated",
        ClipboardResult::Oversized => "Clipboard text exceeds the 64 KiB limit",
        ClipboardResult::ResourceLimit => "Clipboard operation capacity is busy",
        ClipboardResult::TimedOut => "Clipboard operation timed out",
        ClipboardResult::Cancelled => "Clipboard operation was cancelled",
        ClipboardResult::Denied => "Clipboard operation was denied by the peer",
        ClipboardResult::Failed => "Clipboard operation failed",
    };
    let tone = match result {
        ClipboardResult::Success => StatusTone::Accent,
        ClipboardResult::Unavailable
        | ClipboardResult::NotConnected
        | ClipboardResult::NotNegotiated
        | ClipboardResult::Cancelled => StatusTone::Neutral,
        ClipboardResult::Oversized
        | ClipboardResult::ResourceLimit
        | ClipboardResult::TimedOut
        | ClipboardResult::Denied
        | ClipboardResult::Failed => StatusTone::Critical,
    };
    Some((label, tone))
}

fn detail_row(label: &str, value: &str, cx: &App) -> Div {
    let theme = cx.theme();
    let appearance = active_theme(cx);

    div()
        .flex()
        .items_center()
        .justify_between()
        .min_h(px(appearance.metrics.row_height))
        .px(px(appearance.spacing.lg))
        .border_1()
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
