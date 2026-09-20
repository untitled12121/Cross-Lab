package dev.crosslab.android.features.owner

import androidx.compose.foundation.BorderStroke
import androidx.compose.foundation.border
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.text.BasicText
import androidx.compose.runtime.Composable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.RectangleShape
import androidx.compose.ui.unit.dp
import dev.crosslab.android.features.appearance.ThemeDocument
import dev.crosslab.android.features.appearance.toComposeColor
import dev.crosslab.android.features.appearance.toTextStyle
import dev.crosslab.android.features.devices.RuntimeControllerState

@Composable
fun OwnerScreen(
    theme: ThemeDocument,
    runtime: RuntimeControllerState,
) {
    val colors = theme.colors
    val snapshot = runtime.snapshot

    Column(
        modifier =
            Modifier
                .fillMaxSize()
                .padding(theme.spacing.xl.toFloat().dp),
    ) {
        BasicText(
            text = "Owner",
            style =
                theme.typography.scales.title
                    .toTextStyle()
                    .copy(color = colors.foreground.toComposeColor()),
        )
        BasicText(
            text = "Local-first owner identity and this device",
            style =
                theme.typography.scales.caption
                    .toTextStyle()
                    .copy(color = colors.mutedForeground.toComposeColor()),
        )

        Column(
            modifier =
                Modifier
                    .fillMaxWidth()
                    .padding(top = theme.spacing.xl.toFloat().dp)
                    .border(
                        BorderStroke(
                            theme.metrics.borderWidth.toFloat().dp,
                            colors.border.toComposeColor(),
                        ),
                        RectangleShape,
                    ),
        ) {
            InfoRow(theme, "Account model", "Owner-controlled local identity")
            InfoRow(theme, "Cloud sign-in", "Not required")
            InfoRow(theme, "Owner ID", snapshot.ownerId?.take(16) ?: "Available after authentication")
            InfoRow(
                theme,
                "This device",
                snapshot.localDeviceId?.take(16) ?: "Available after authentication",
            )
        }

        BasicText(
            modifier =
                Modifier
                    .fillMaxWidth()
                    .padding(top = theme.spacing.xl.toFloat().dp)
                    .border(
                        BorderStroke(
                            theme.metrics.borderWidth.toFloat().dp,
                            colors.border.toComposeColor(),
                        ),
                        RectangleShape,
                    )
                    .padding(theme.spacing.lg.toFloat().dp),
            text =
                "Cross-Lab does not require an email/password account or vendor cloud. " +
                    "Development provisioning is temporary test identity; production authority " +
                    "persistence and recovery remain separate platform-security work.",
            style =
                theme.typography.scales.caption
                    .toTextStyle()
                    .copy(color = colors.mutedForeground.toComposeColor()),
        )
    }
}

@Composable
private fun InfoRow(
    theme: ThemeDocument,
    label: String,
    value: String,
) {
    val colors = theme.colors
    Row(
        modifier =
            Modifier
                .fillMaxWidth()
                .border(
                    BorderStroke(theme.metrics.borderWidth.toFloat().dp, colors.border.toComposeColor()),
                    RectangleShape,
                )
                .padding(
                    horizontal = theme.spacing.xl.toFloat().dp,
                    vertical = theme.spacing.md.toFloat().dp,
                ),
        verticalAlignment = Alignment.CenterVertically,
    ) {
        BasicText(
            modifier = Modifier.weight(1f),
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
