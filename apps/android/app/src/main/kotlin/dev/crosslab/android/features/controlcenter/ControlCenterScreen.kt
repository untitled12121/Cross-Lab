package dev.crosslab.android.features.controlcenter

import androidx.compose.foundation.BorderStroke
import androidx.compose.foundation.background
import androidx.compose.foundation.border
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.safeDrawingPadding
import androidx.compose.foundation.text.BasicText
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
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
import dev.crosslab.android.features.devices.DevicesScreen
import dev.crosslab.android.features.devices.DevicesState
import dev.crosslab.android.features.devices.RuntimeControllerState
import dev.crosslab.android.features.devices.RuntimeLifecycle
import dev.crosslab.android.features.owner.OwnerScreen

private enum class ControlCenterSection {
    DEVICES,
    OWNER,
}

@Composable
fun ControlCenterScreen(
    theme: ThemeDocument,
    runtime: RuntimeControllerState,
    onDisconnect: () -> Unit,
    onReconnect: () -> Unit,
) {
    val colors = theme.colors
    var section by remember { mutableStateOf(ControlCenterSection.DEVICES) }

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
                        BorderStroke(
                            theme.metrics.borderWidth.toFloat().dp,
                            colors.border.toComposeColor(),
                        ),
                        RectangleShape,
                    )
                    .padding(
                        horizontal = theme.spacing.xl.toFloat().dp,
                        vertical = theme.spacing.lg.toFloat().dp,
                    ),
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

        Row(
            modifier =
                Modifier
                    .fillMaxWidth()
                    .padding(
                        horizontal = theme.spacing.xl.toFloat().dp,
                        vertical = theme.spacing.md.toFloat().dp,
                    ),
            horizontalArrangement = Arrangement.spacedBy(theme.spacing.sm.toFloat().dp),
        ) {
            ControlButton(
                theme = theme,
                label = "Devices",
                active = section == ControlCenterSection.DEVICES,
                onClick = { section = ControlCenterSection.DEVICES },
            )
            ControlButton(
                theme = theme,
                label = "Owner",
                active = section == ControlCenterSection.OWNER,
                onClick = { section = ControlCenterSection.OWNER },
            )
        }

        when (section) {
            ControlCenterSection.DEVICES ->
                DevicesScreen(
                    theme = theme,
                    devices = DevicesState.from(runtime.snapshot),
                    runtime = runtime,
                    onDisconnect = onDisconnect,
                    onReconnect = onReconnect,
                )

            ControlCenterSection.OWNER ->
                OwnerScreen(
                    theme = theme,
                    runtime = runtime,
                )
        }
    }
}
