use crate::{
    components::ui::{StatusTone, status_badge},
    features::{
        appearance::{active_theme, font_weight},
        devices::{DevicePresentation, PermissionPresentation},
        file_transfer::{
            FileTransferFailure, FileTransferFeatureState, FileTransferOperationState,
            FileTransferStage,
        },
    },
};
use gpui_kit::{
    App, Div, ParentElement as _, Styled as _, component::theme::ActiveTheme as _, div, px,
};

const FILE_TRANSFER: &str = "files.transfer";
const OP_RECEIVE: &str = "receive";

pub(crate) fn file_transfer_panel(
    device: &DevicePresentation,
    permissions: &[PermissionPresentation],
    state: &FileTransferFeatureState,
    actions: Div,
    cx: &App,
) -> Div {
    let theme = cx.theme();
    let appearance = active_theme(cx);
    let negotiated = device
        .capability_ids()
        .iter()
        .any(|capability| capability == FILE_TRANSFER);

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
                .child("File transfer"),
        )
        .child(
            div()
                .text_size(px(appearance.typography.scales.caption.size))
                .text_color(theme.muted_foreground)
                .child("Single file · explicit local selection · resumable verified transfer"),
        )
        .child(detail_row(
            "Peer may send files here",
            permission_label(permissions, FILE_TRANSFER, OP_RECEIVE),
            cx,
        ))
        .child(actions)
        .child(operation_panel("Send", state.send(), cx))
        .child(operation_panel("Receive", state.receive(), cx));

    if !state.available() {
        panel = panel.child(note(
            "File transfer is unavailable in this runtime.",
            cx,
        ));
    } else if !negotiated {
        panel = panel.child(note(
            "This authenticated session did not negotiate files.transfer v2.",
            cx,
        ));
    }

    panel
}

fn operation_panel(
    label: &str,
    operation: &FileTransferOperationState,
    cx: &App,
) -> Div {
    let theme = cx.theme();
    let appearance = active_theme(cx);
    let (status_label, tone) = stage_status(operation);

    let mut panel = div()
        .flex()
        .flex_col()
        .border_1()
        .border_color(theme.border)
        .p(px(appearance.spacing.lg))
        .gap(px(appearance.spacing.sm))
        .child(
            div()
                .flex()
                .items_center()
                .justify_between()
                .gap(px(appearance.spacing.md))
                .child(
                    div()
                        .text_size(px(appearance.typography.scales.body.size))
                        .font_weight(font_weight(appearance.typography.scales.body.weight))
                        .child(label.to_owned()),
                )
                .child(status_badge(status_label, tone, cx)),
        );

    if let Some(name) = operation.display_name() {
        panel = panel.child(detail_row("File", name, cx));
    }
    if operation.total_bytes() > 0 {
        panel = panel.child(detail_row(
            "Progress",
            &progress_label(operation.transferred_bytes(), operation.total_bytes()),
            cx,
        ));
    }
    if let Some(failure) = operation.failure() {
        panel = panel.child(note(failure_label(failure), cx));
    }

    panel
}

fn stage_status(operation: &FileTransferOperationState) -> (&'static str, StatusTone) {
    match operation.stage() {
        FileTransferStage::Idle => ("Idle", StatusTone::Neutral),
        FileTransferStage::AwaitingDestination => ("Choose destination", StatusTone::Accent),
        FileTransferStage::Preparing => ("Preparing", StatusTone::Accent),
        FileTransferStage::WaitingForPeer => ("Waiting for peer", StatusTone::Accent),
        FileTransferStage::Ready => ("Ready to receive", StatusTone::Accent),
        FileTransferStage::Transferring => ("Transferring", StatusTone::Accent),
        FileTransferStage::Completed => ("Completed", StatusTone::Accent),
        FileTransferStage::AlreadyComplete => ("Already complete", StatusTone::Accent),
        FileTransferStage::Cancelled => ("Cancelled", StatusTone::Neutral),
        FileTransferStage::Failed => ("Failed", StatusTone::Critical),
    }
}

fn failure_label(failure: FileTransferFailure) -> &'static str {
    match failure {
        FileTransferFailure::Source => "The selected local source is unavailable or changed.",
        FileTransferFailure::NotConnected => "The peer is not connected.",
        FileTransferFailure::NotNegotiated => "File transfer is not negotiated with this peer.",
        FileTransferFailure::Denied => "The peer denied the file transfer.",
        FileTransferFailure::ResourceLimit => "File-transfer capacity is currently busy.",
        FileTransferFailure::TimedOut => "The file-transfer request timed out.",
        FileTransferFailure::Integrity => "File verification failed. The final file was not published.",
        FileTransferFailure::Storage => "Local or remote storage could not complete the transfer.",
        FileTransferFailure::Connection => "The authenticated session ended during the transfer.",
        FileTransferFailure::Failed => "The file transfer failed.",
    }
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

fn progress_label(done: u64, total: u64) -> String {
    let percent = if total == 0 {
        0
    } else {
        done.saturating_mul(100).checked_div(total).unwrap_or(0).min(100)
    };
    format!(
        "{} / {} · {percent}%",
        format_bytes(done),
        format_bytes(total)
    )
}

fn format_bytes(bytes: u64) -> String {
    const KIB: u64 = 1024;
    const MIB: u64 = 1024 * KIB;
    const GIB: u64 = 1024 * MIB;

    if bytes >= GIB {
        format!("{:.1} GiB", bytes as f64 / GIB as f64)
    } else if bytes >= MIB {
        format!("{:.1} MiB", bytes as f64 / MIB as f64)
    } else if bytes >= KIB {
        format!("{:.1} KiB", bytes as f64 / KIB as f64)
    } else {
        format!("{bytes} B")
    }
}

fn note(text: &str, cx: &App) -> Div {
    let theme = cx.theme();
    let appearance = active_theme(cx);

    div()
        .text_size(px(appearance.typography.scales.caption.size))
        .text_color(theme.muted_foreground)
        .child(text.to_owned())
}

fn detail_row(label: &str, value: &str, cx: &App) -> Div {
    let theme = cx.theme();
    let appearance = active_theme(cx);

    div()
        .flex()
        .items_center()
        .justify_between()
        .gap(px(appearance.spacing.md))
        .min_h(px(appearance.metrics.row_height))
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn progress_is_bounded_at_one_hundred_percent() {
        assert_eq!(progress_label(150, 100), "150 B / 100 B · 100%");
    }

    #[test]
    fn byte_format_uses_binary_units() {
        assert_eq!(format_bytes(1024), "1.0 KiB");
        assert_eq!(format_bytes(1024 * 1024), "1.0 MiB");
    }
}
