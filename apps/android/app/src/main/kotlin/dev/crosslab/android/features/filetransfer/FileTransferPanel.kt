package dev.crosslab.android.features.filetransfer

import androidx.compose.foundation.BorderStroke
import androidx.compose.foundation.border
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.text.BasicText
import androidx.compose.runtime.Composable
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

private const val CAPABILITY = "files.transfer"
private const val RECEIVE = "receive"

@Composable
fun FileTransferPanel(
    theme: ThemeDocument,
    device: DevicePresentation,
    state: FileTransferState,
    retainAccess: Boolean,
    onToggleRetainAccess: () -> Unit,
    onSend: () -> Unit,
    onAccept: (String, String) -> Unit,
    onResume: (String) -> Unit,
    onReauthorize: (String) -> Unit,
    onDecline: () -> Unit,
    onCancel: () -> Unit,
    onRetry: () -> Unit,
) {
    val colors = theme.colors
    val connected =
        device.connectivity == ConnectivityDisplay.CONNECTED &&
            device.session == SessionDisplay.ACTIVE
    val negotiated = CAPABILITY in device.capabilityIds
    val pending = state.stage == FileTransferStage.WAITING_DESTINATION &&
        state.direction == FileTransferDirection.RECEIVE
    val busy = state.canCancel || pending

    Column(
        modifier = Modifier.fillMaxWidth().border(
            BorderStroke(theme.metrics.borderWidth.toFloat().dp, colors.border.toComposeColor()),
            RectangleShape,
        ).padding(theme.spacing.xl.toFloat().dp),
    ) {
        BasicText(
            text = "File transfer",
            style = theme.typography.scales.label.toTextStyle()
                .copy(color = colors.foreground.toComposeColor()),
        )
        Spacer(Modifier.height(theme.spacing.xs.toFloat().dp))
        BasicText(
            text = "Single file · explicit approval · no preview or remote filesystem access",
            style = theme.typography.scales.caption.toTextStyle()
                .copy(color = colors.mutedForeground.toComposeColor()),
        )
        Spacer(Modifier.height(theme.spacing.sm.toFloat().dp))
        BasicText(
            text = "Peer receive permission: " +
                (device.permissionRules.firstOrNull {
                    it.capabilityId == CAPABILITY && it.operation == RECEIVE
                }?.effect?.label ?: "Default deny"),
            style = theme.typography.scales.caption.toTextStyle()
                .copy(color = colors.mutedForeground.toComposeColor()),
        )
        if (!state.available || !negotiated || !connected) {
            Spacer(Modifier.height(theme.spacing.sm.toFloat().dp))
            BasicText(
                text = when {
                    !state.available -> "File transfer is unavailable in this runtime."
                    !connected -> "Connect to an authenticated device to transfer files."
                    else -> "File transfer v2 is not negotiated with this device."
                },
                style = theme.typography.scales.caption.toTextStyle()
                    .copy(color = colors.mutedForeground.toComposeColor()),
            )
        }

        Spacer(Modifier.height(theme.spacing.md.toFloat().dp))
        Column(verticalArrangement = Arrangement.spacedBy(theme.spacing.sm.toFloat().dp)) {
            ControlButton(
                theme = theme,
                label = "Send file",
                enabled = state.available && negotiated && connected && !busy,
                onClick = onSend,
            )
            if (state.canRetry && !busy) {
                ControlButton(
                    theme = theme,
                    label = "Retry / resume",
                    enabled = connected && negotiated,
                    onClick = onRetry,
                )
            }
            if (state.canCancel || pending) {
                ControlButton(
                    theme = theme,
                    label = "Cancel",
                    destructive = true,
                    onClick = onCancel,
                )
            }
        }
        if (pending) {
            Spacer(Modifier.height(theme.spacing.md.toFloat().dp))
            BasicText(
                text = state.displayName ?: "Incoming file",
                style = theme.typography.scales.body.toTextStyle()
                    .copy(color = colors.foreground.toComposeColor()),
            )
            Spacer(Modifier.height(theme.spacing.xs.toFloat().dp))
            Column(verticalArrangement = Arrangement.spacedBy(theme.spacing.sm.toFloat().dp)) {
                if (state.resumeAvailable) {
                    ControlButton(
                        theme = theme,
                        label = "Resume saved destination",
                        onClick = {
                            state.pendingRequestId?.let(onResume)
                        },
                    )
                    ControlButton(
                        theme = theme,
                        label = "Reauthorize document",
                        onClick = {
                            state.pendingRequestId?.let(onReauthorize)
                        },
                    )
                } else {
                    ControlButton(
                        theme = theme,
                        label = "Choose destination",
                        onClick = {
                            state.pendingRequestId?.let { id ->
                                onAccept(state.displayName ?: "received-file", id)
                            }
                        },
                    )
                }
                ControlButton(
                    theme = theme,
                    label = "Decline",
                    onClick = onDecline,
                )
            }
        }
        Spacer(Modifier.height(theme.spacing.sm.toFloat().dp))
        ControlButton(
            theme = theme,
            label = if (retainAccess) "Retain document access: on" else "Retain document access: off",
            enabled = !busy,
            active = retainAccess,
            onClick = onToggleRetainAccess,
        )
        BasicText(
            text = "Optional persistent document access supports retry after app restart.",
            style = theme.typography.scales.caption.toTextStyle()
                .copy(color = colors.mutedForeground.toComposeColor()),
        )

        if (state.stage != FileTransferStage.IDLE) {
            Spacer(Modifier.height(theme.spacing.md.toFloat().dp))
            StatusBadge(
                label = statusLabel(state),
                tone = when (state.stage) {
                    FileTransferStage.COMPLETED -> StatusTone.ACCENT
                    FileTransferStage.FAILED -> StatusTone.CRITICAL
                    FileTransferStage.CANCELLED -> StatusTone.NEUTRAL
                    else -> StatusTone.ACCENT
                },
                textScale = theme.typography.scales.caption,
                border = colors.border.toComposeColor(),
                neutralBackground = colors.muted.toComposeColor(),
                neutralForeground = colors.mutedForeground.toComposeColor(),
                accentBackground = colors.accent.toComposeColor(),
                accentForeground = colors.accentForeground.toComposeColor(),
                criticalBackground = colors.destructive.toComposeColor(),
                criticalForeground = colors.destructiveForeground.toComposeColor(),
            )
            Spacer(Modifier.height(theme.spacing.xs.toFloat().dp))
            BasicText(
                text = progressLabel(state),
                style = theme.typography.scales.caption.toTextStyle()
                    .copy(color = colors.mutedForeground.toComposeColor()),
            )
        }
    }
}

internal fun statusLabel(state: FileTransferState): String =
    when (state.stage) {
        FileTransferStage.IDLE -> "Ready"
        FileTransferStage.PREPARING -> "Preparing file"
        FileTransferStage.WAITING_PEER -> "Waiting for peer"
        FileTransferStage.WAITING_DESTINATION -> "Destination approval required"
        FileTransferStage.TRANSFERRING -> "Transferring"
        FileTransferStage.FINALIZING -> "Verifying and saving"
        FileTransferStage.COMPLETED -> "File transferred"
        FileTransferStage.CANCELLED -> "File transfer cancelled"
        FileTransferStage.FAILED -> when (state.failure) {
            FileTransferFailure.NOT_CONNECTED -> "Peer disconnected"
            FileTransferFailure.NOT_NEGOTIATED -> "File transfer not supported by peer"
            FileTransferFailure.DENIED -> "Peer denied transfer"
            FileTransferFailure.SOURCE_CHANGED -> "Source changed — select the file again"
            FileTransferFailure.STORAGE -> "Local storage failed"
            FileTransferFailure.INTEGRITY -> "File verification failed"
            FileTransferFailure.RESOURCE_LIMIT -> "Transfer capacity reached"
            FileTransferFailure.UNAVAILABLE -> "File transfer unavailable"
            else -> "File transfer failed"
        }
    }

internal fun progressLabel(state: FileTransferState): String {
    val total = state.totalBytes
    if (total == 0uL) return if (state.stage == FileTransferStage.COMPLETED) "Empty file" else "0 bytes"
    val done = state.transferredBytes.coerceAtMost(total)
    val percent = ((done.toDouble() / total.toDouble()) * 100.0).toInt()
    return "${done} / ${total} bytes · ${percent}%"
}
