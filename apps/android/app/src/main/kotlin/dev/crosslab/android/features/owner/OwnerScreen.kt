package dev.crosslab.android.features.owner

import androidx.compose.foundation.BorderStroke
import androidx.compose.foundation.border
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.verticalScroll
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
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
import dev.crosslab.android.features.appearance.ThemeDocument
import dev.crosslab.android.features.appearance.toComposeColor
import dev.crosslab.android.features.appearance.toTextStyle
import dev.crosslab.android.features.devices.RuntimeControllerState
import dev.crosslab.android.components.ui.ControlButton

@Composable
fun OwnerScreen(
    theme: ThemeDocument,
    runtime: RuntimeControllerState,
    trustedPeerIds: List<String>,
    inventoryGeneration: Int,
    revokedPeerIds: List<String>,
    canRevokePeers: Boolean,
    revocationNotice: String?,
    onRevokePeer: (Int, Int) -> Unit,
) {
    val colors = theme.colors
    val snapshot = runtime.snapshot
    var pendingRevoke by remember { mutableStateOf<Pair<Int, Int>?>(null) }

    Column(
        modifier =
            Modifier
                .fillMaxSize()
                .verticalScroll(rememberScrollState())
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
            InfoRow(theme, "Paired devices", trustedPeerIds.size.toString())
            trustedPeerIds.take(24).forEachIndexed { index, peerId ->
                InfoRow(theme, "Trusted device ${index + 1}", peerId)
                if (canRevokePeers) {
                    if (pendingRevoke == (index to inventoryGeneration)) {
                        BasicText(
                            text = "Revoke this device permanently? Its existing credentials will no longer grant access.",
                            style = theme.typography.scales.caption.toTextStyle()
                                .copy(color = colors.destructive.toComposeColor()),
                        )
                        ControlButton(
                            theme = theme,
                            label = "Confirm revocation",
                            destructive = true,
                            onClick = {
                                pendingRevoke = null
                                onRevokePeer(index, inventoryGeneration)
                            },
                        )
                        ControlButton(
                            theme = theme,
                            label = "Cancel",
                            onClick = { pendingRevoke = null },
                        )
                    } else {
                        ControlButton(
                            theme = theme,
                            label = "Revoke ${peerId}",
                            destructive = true,
                            onClick = { pendingRevoke = index to inventoryGeneration },
                        )
                    }
                }
            }
            if (trustedPeerIds.size > 24) {
                InfoRow(theme, "More paired devices", "${trustedPeerIds.size - 24} not shown")
            }
            if (trustedPeerIds.isNotEmpty() && !canRevokePeers) {
                InfoRow(theme, "Revocation", "Owner signing authority unavailable")
            }
            InfoRow(theme, "Revoked devices", revokedPeerIds.size.toString())
            revokedPeerIds.take(24).forEachIndexed { index, peerId ->
                InfoRow(theme, "Revoked ${index + 1}", peerId)
            }
            revocationNotice?.let { InfoRow(theme, "Revocation result", it) }
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
