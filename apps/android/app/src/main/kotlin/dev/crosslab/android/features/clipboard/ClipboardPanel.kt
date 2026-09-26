package dev.crosslab.android.features.clipboard

import androidx.compose.foundation.BorderStroke
import androidx.compose.foundation.border
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.text.BasicText
import androidx.compose.runtime.Composable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.RectangleShape
import androidx.compose.ui.unit.dp
import dev.crosslab.android.components.ui.ControlButton
import dev.crosslab.android.components.ui.StatusBadge
import dev.crosslab.android.components.ui.StatusTone
import dev.crosslab.android.features.appearance.ThemeDocument
import dev.crosslab.android.features.appearance.toComposeColor
import dev.crosslab.android.features.appearance.toTextStyle
import dev.crosslab.android.features.devices.ConnectivityDisplay
import dev.crosslab.android.features.devices.DevicePresentation
import dev.crosslab.android.features.devices.SessionDisplay

private const val CLIPBOARD_READ = "clipboard.read"
private const val CLIPBOARD_WRITE = "clipboard.write"
private const val OP_GET = "get"
private const val OP_SET = "set"

@Composable
fun ClipboardPanel(
    theme: ThemeDocument,
    device: DevicePresentation,
    state: ClipboardState,
    onSend: () -> Unit,
    onFetch: () -> Unit,
) {
    val colors = theme.colors
    val sendNegotiated = CLIPBOARD_WRITE in device.capabilityIds
    val fetchNegotiated = CLIPBOARD_READ in device.capabilityIds
    val sessionReady =
        device.connectivity == ConnectivityDisplay.CONNECTED &&
            device.session == SessionDisplay.ACTIVE
    val actionsEnabled = state.available && sessionReady && !state.busy

    Column(
        modifier =
            Modifier
                .fillMaxWidth()
                .border(
                    BorderStroke(
                        theme.metrics.borderWidth.toFloat().dp,
                        colors.border.toComposeColor(),
                    ),
                    RectangleShape,
                )
                .padding(theme.spacing.xl.toFloat().dp),
    ) {
        BasicText(
            text = "Clipboard",
            style =
                theme.typography.scales.label
                    .toTextStyle()
                    .copy(color = colors.foreground.toComposeColor()),
        )
        Spacer(Modifier.height(theme.spacing.xs.toFloat().dp))
        BasicText(
            text = "Text only · explicit actions · no clipboard preview or history",
            style =
                theme.typography.scales.caption
                    .toTextStyle()
                    .copy(color = colors.mutedForeground.toComposeColor()),
        )
        Spacer(Modifier.height(theme.spacing.md.toFloat().dp))

        ClipboardDetailRow(
            theme,
            "Peer may read this clipboard",
            permissionLabel(device, CLIPBOARD_READ, OP_GET),
        )
        ClipboardDetailRow(
            theme,
            "Peer may write this clipboard",
            permissionLabel(device, CLIPBOARD_WRITE, OP_SET),
        )

        Spacer(Modifier.height(theme.spacing.md.toFloat().dp))
        Row(
            horizontalArrangement = Arrangement.spacedBy(theme.spacing.sm.toFloat().dp),
            verticalAlignment = Alignment.CenterVertically,
        ) {
            ControlButton(
                theme = theme,
                label =
                    if (state.busy && state.action == ClipboardAction.SEND) {
                        "Sending…"
                    } else {
                        "Send clipboard"
                    },
                enabled = actionsEnabled && sendNegotiated,
                active = state.busy && state.action == ClipboardAction.SEND,
                onClick = onSend,
            )
            ControlButton(
                theme = theme,
                label =
                    if (state.busy && state.action == ClipboardAction.FETCH) {
                        "Fetching…"
                    } else {
                        "Fetch clipboard"
                    },
                enabled = actionsEnabled && fetchNegotiated,
                active = state.busy && state.action == ClipboardAction.FETCH,
                onClick = onFetch,
            )
        }

        val status = clipboardStatus(state)
        if (status != null) {
            Spacer(Modifier.height(theme.spacing.md.toFloat().dp))
            StatusBadge(
                label = status.first,
                tone = status.second,
                textScale = theme.typography.scales.caption,
                border = colors.border.toComposeColor(),
                neutralBackground = colors.muted.toComposeColor(),
                neutralForeground = colors.mutedForeground.toComposeColor(),
                accentBackground = colors.accent.toComposeColor(),
                accentForeground = colors.accentForeground.toComposeColor(),
                criticalBackground = colors.destructive.toComposeColor(),
                criticalForeground = colors.destructiveForeground.toComposeColor(),
            )
        } else if (!state.available) {
            Spacer(Modifier.height(theme.spacing.md.toFloat().dp))
            BasicText(
                text = "Clipboard actions are unavailable in this runtime.",
                style =
                    theme.typography.scales.caption
                        .toTextStyle()
                        .copy(color = colors.mutedForeground.toComposeColor()),
            )
        } else if (!sendNegotiated || !fetchNegotiated) {
            Spacer(Modifier.height(theme.spacing.md.toFloat().dp))
            BasicText(
                text =
                    when {
                        !sendNegotiated && !fetchNegotiated ->
                            "This session did not negotiate clipboard v1."
                        !sendNegotiated ->
                            "Sending is unavailable because clipboard.write is not negotiated."
                        else ->
                            "Fetching is unavailable because clipboard.read is not negotiated."
                    },
                style =
                    theme.typography.scales.caption
                        .toTextStyle()
                        .copy(color = colors.mutedForeground.toComposeColor()),
            )
        }
    }
}

private fun permissionLabel(
    device: DevicePresentation,
    capabilityId: String,
    operation: String,
): String =
    device.permissionRules
        .firstOrNull {
            it.capabilityId == capabilityId && it.operation == operation
        }
        ?.effect
        ?.label
        ?: "Default deny"

private fun clipboardStatus(state: ClipboardState): Pair<String, StatusTone>? {
    if (state.busy) {
        return (
            if (state.action == ClipboardAction.SEND) {
                "Sending clipboard"
            } else {
                "Fetching clipboard"
            }
        ) to StatusTone.ACCENT
    }

    val result = state.result ?: return null
    val message =
        when (result) {
            ClipboardResult.SUCCESS ->
                if (state.action == ClipboardAction.SEND) {
                    "Clipboard sent"
                } else {
                    "Clipboard fetched to this device"
                }
            ClipboardResult.UNAVAILABLE -> "Text clipboard is unavailable"
            ClipboardResult.NOT_CONNECTED -> "Clipboard peer is not connected"
            ClipboardResult.NOT_NEGOTIATED -> "Clipboard operation is not negotiated"
            ClipboardResult.OVERSIZED -> "Clipboard text exceeds the 64 KiB limit"
            ClipboardResult.RESOURCE_LIMIT -> "Clipboard operation capacity is busy"
            ClipboardResult.TIMED_OUT -> "Clipboard operation timed out"
            ClipboardResult.CANCELLED -> "Clipboard operation was cancelled"
            ClipboardResult.DENIED -> "Clipboard operation was denied by the peer"
            ClipboardResult.FAILED -> "Clipboard operation failed"
        }
    val tone =
        when (result) {
            ClipboardResult.SUCCESS -> StatusTone.ACCENT
            ClipboardResult.UNAVAILABLE,
            ClipboardResult.NOT_CONNECTED,
            ClipboardResult.NOT_NEGOTIATED,
            ClipboardResult.CANCELLED,
            -> StatusTone.NEUTRAL
            ClipboardResult.OVERSIZED,
            ClipboardResult.RESOURCE_LIMIT,
            ClipboardResult.TIMED_OUT,
            ClipboardResult.DENIED,
            ClipboardResult.FAILED,
            -> StatusTone.CRITICAL
        }
    return message to tone
}

@Composable
private fun ClipboardDetailRow(
    theme: ThemeDocument,
    label: String,
    value: String,
) {
    val colors = theme.colors
    Row(
        modifier =
            Modifier
                .fillMaxWidth()
                .height(theme.metrics.rowHeight.toFloat().dp)
                .border(
                    BorderStroke(
                        theme.metrics.borderWidth.toFloat().dp,
                        colors.border.toComposeColor(),
                    ),
                    RectangleShape,
                )
                .padding(horizontal = theme.spacing.lg.toFloat().dp),
        horizontalArrangement = Arrangement.SpaceBetween,
        verticalAlignment = Alignment.CenterVertically,
    ) {
        BasicText(
            text = label,
            style =
                theme.typography.scales.caption
                    .toTextStyle()
                    .copy(color = colors.mutedForeground.toComposeColor()),
        )
        BasicText(
            text = value,
            style =
                theme.typography.scales.body
                    .toTextStyle()
                    .copy(color = colors.foreground.toComposeColor()),
        )
    }
}
