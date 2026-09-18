package dev.crosslab.android.features.devices

import androidx.compose.foundation.BorderStroke
import androidx.compose.foundation.border
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.safeDrawingPadding
import androidx.compose.foundation.shape.RectangleShape
import androidx.compose.foundation.text.BasicText
import androidx.compose.runtime.Composable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.background
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.unit.dp
import dev.crosslab.android.components.ui.StatusBadge
import dev.crosslab.android.components.ui.StatusTone
import dev.crosslab.android.features.appearance.ThemeDocument
import dev.crosslab.android.features.appearance.toComposeColor
import dev.crosslab.android.features.appearance.toTextStyle

@Composable
fun DevicesScreen(
    theme: ThemeDocument,
    devices: DevicesState,
    runtime: RuntimeControllerState,
) {
    val colors = theme.colors

    Column(
        modifier =
            Modifier
                .fillMaxSize()
                .background(colors.background.toComposeColor())
                .safeDrawingPadding(),
    ) {
        Row(
            modifier =
                Modifier
                    .fillMaxWidth()
                    .border(
                        BorderStroke(theme.metrics.borderWidth.toFloat().dp, colors.border.toComposeColor()),
                        RectangleShape,
                    )
                    .padding(horizontal = theme.spacing.xl.toFloat().dp, vertical = theme.spacing.lg.toFloat().dp),
            horizontalArrangement = Arrangement.SpaceBetween,
            verticalAlignment = Alignment.CenterVertically,
        ) {
            BasicText(
                text = "Cross-Lab",
                style =
                    theme.typography.scales.label
                        .toTextStyle()
                        .copy(color = colors.foreground.toComposeColor()),
            )
            StatusBadge(
                label =
                    when (runtime.lifecycle) {
                        RuntimeLifecycle.RUNNING -> "Runtime active"
                        RuntimeLifecycle.STOPPED -> "Runtime stopped"
                        RuntimeLifecycle.SHUTDOWN -> "Runtime shutdown"
                    },
                tone =
                    if (runtime.lifecycle == RuntimeLifecycle.RUNNING) {
                        StatusTone.ACCENT
                    } else {
                        StatusTone.NEUTRAL
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
        }

        Column(
            modifier =
                Modifier
                    .fillMaxWidth()
                    .padding(theme.spacing.xl.toFloat().dp),
        ) {
            BasicText(
                text = "Devices",
                style =
                    theme.typography.scales.title
                        .toTextStyle()
                        .copy(color = colors.foreground.toComposeColor()),
            )
            Spacer(Modifier.height(theme.spacing.md.toFloat().dp))

            val device = devices.current
            if (device == null) {
                DisconnectedState(theme, runtime.networkAvailable)
            } else {
                DevicePanel(theme, device)
            }
        }
    }
}

@Composable
private fun DisconnectedState(
    theme: ThemeDocument,
    networkAvailable: Boolean,
) {
    val colors = theme.colors
    Column(
        modifier =
            Modifier
                .fillMaxWidth()
                .border(
                    BorderStroke(theme.metrics.borderWidth.toFloat().dp, colors.border.toComposeColor()),
                    RectangleShape,
                )
                .padding(theme.spacing.xl.toFloat().dp),
    ) {
        BasicText(
            text = "No connected device",
            style =
                theme.typography.scales.label
                    .toTextStyle()
                    .copy(color = colors.foreground.toComposeColor()),
        )
        Spacer(Modifier.height(theme.spacing.xs.toFloat().dp))
        BasicText(
            text =
                if (networkAvailable) {
                    "The Android shell is ready for authenticated runtime wiring."
                } else {
                    "Network unavailable."
                },
            style =
                theme.typography.scales.body
                    .toTextStyle()
                    .copy(color = colors.mutedForeground.toComposeColor()),
        )
    }
}

@Composable
private fun DevicePanel(
    theme: ThemeDocument,
    device: DevicePresentation,
) {
    val colors = theme.colors
    val trustTone =
        when (device.trust) {
            TrustDisplay.TRUSTED -> StatusTone.ACCENT
            TrustDisplay.PENDING -> StatusTone.NEUTRAL
            TrustDisplay.REVOKED -> StatusTone.CRITICAL
        }
    val connectivityTone =
        if (device.connectivity == ConnectivityDisplay.CONNECTED) {
            StatusTone.ACCENT
        } else {
            StatusTone.NEUTRAL
        }

    Column(
        modifier =
            Modifier
                .fillMaxWidth()
                .border(
                    BorderStroke(theme.metrics.borderWidth.toFloat().dp, colors.border.toComposeColor()),
                    RectangleShape,
                ),
    ) {
        Row(
            modifier =
                Modifier
                    .fillMaxWidth()
                    .padding(theme.spacing.xl.toFloat().dp),
            horizontalArrangement = Arrangement.SpaceBetween,
            verticalAlignment = Alignment.CenterVertically,
        ) {
            BasicText(
                text = device.peerId,
                style =
                    theme.typography.scales.label
                        .toTextStyle()
                        .copy(color = colors.foreground.toComposeColor()),
            )
            Row(horizontalArrangement = Arrangement.spacedBy(theme.spacing.sm.toFloat().dp)) {
                DeviceBadge(theme, device.connectivity.label, connectivityTone)
                DeviceBadge(theme, device.trust.label, trustTone)
            }
        }

        DetailRow(theme, "Session", device.session.label)
        DetailRow(theme, "Protocol", device.protocol ?: "Unavailable")
        DetailRow(theme, "Network", device.network.label)
        DetailRow(theme, "Security", device.security.label)
        DetailRow(
            theme,
            "Metered",
            when (device.metered) {
                true -> "Yes"
                false -> "No"
                null -> "Unknown"
            },
        )
        DetailRow(theme, "Capabilities", device.capabilityCount.toString())
    }
}

@Composable
private fun DeviceBadge(
    theme: ThemeDocument,
    label: String,
    tone: StatusTone,
) {
    val colors = theme.colors
    StatusBadge(
        label = label,
        tone = tone,
        textScale = theme.typography.scales.caption,
        border = colors.border.toComposeColor(),
        neutralBackground = colors.muted.toComposeColor(),
        neutralForeground = colors.mutedForeground.toComposeColor(),
        accentBackground = colors.accent.toComposeColor(),
        accentForeground = colors.accentForeground.toComposeColor(),
        criticalBackground = colors.destructive.toComposeColor(),
        criticalForeground = colors.destructiveForeground.toComposeColor(),
    )
}

@Composable
private fun DetailRow(
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
                    BorderStroke(theme.metrics.borderWidth.toFloat().dp, colors.border.toComposeColor()),
                    RectangleShape,
                )
                .padding(horizontal = theme.spacing.xl.toFloat().dp),
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
