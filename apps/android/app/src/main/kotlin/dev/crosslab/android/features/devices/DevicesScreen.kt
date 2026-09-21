package dev.crosslab.android.features.devices

import androidx.compose.foundation.BorderStroke
import androidx.compose.foundation.border
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
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
import dev.crosslab.android.components.ui.ControlButton
import dev.crosslab.android.components.ui.StatusBadge
import dev.crosslab.android.components.ui.StatusTone
import dev.crosslab.android.features.appearance.ThemeDocument
import dev.crosslab.android.features.appearance.toComposeColor
import dev.crosslab.android.features.appearance.toTextStyle
import dev.crosslab.android.features.pairing.PairingJoinerStage
import dev.crosslab.android.features.pairing.PairingJoinerState
import dev.crosslab.android.features.pairing.PairingScannerPanel
import uniffi.crosslab_mobile_ffi.MobilePairingBootstrap

@Composable
fun DevicesScreen(
    theme: ThemeDocument,
    devices: DevicesState,
    runtime: RuntimeControllerState,
    onDisconnect: () -> Unit,
    onReconnect: () -> Unit,
    pairing: PairingJoinerState,
    onPairingBootstrapScanned: (MobilePairingBootstrap) -> Unit,
    onCancelPairing: () -> Unit,
) {
    val colors = theme.colors
    val device = devices.current
    var scanning by remember { mutableStateOf(false) }

    Column(
        modifier =
            Modifier
                .fillMaxSize()
                .padding(theme.spacing.xl.toFloat().dp),
    ) {
        Row(
            modifier = Modifier.fillMaxWidth(),
            horizontalArrangement = Arrangement.SpaceBetween,
            verticalAlignment = Alignment.CenterVertically,
        ) {
            Column {
                BasicText(
                    text = "Devices",
                    style =
                        theme.typography.scales.title
                            .toTextStyle()
                            .copy(color = colors.foreground.toComposeColor()),
                )
                Spacer(Modifier.height(theme.spacing.xs.toFloat().dp))
                BasicText(
                    text = "Authenticated devices in the current Cross-Lab runtime",
                    style =
                        theme.typography.scales.caption
                            .toTextStyle()
                            .copy(color = colors.mutedForeground.toComposeColor()),
                )
            }

            Row(horizontalArrangement = Arrangement.spacedBy(theme.spacing.sm.toFloat().dp)) {
                ControlButton(
                    theme = theme,
                    label =
                        when {
                            scanning -> "Scanning…"
                            pairing.active -> "Pairing…"
                            else -> "Add Device"
                        },
                    active = scanning || pairing.active,
                    enabled = !pairing.active,
                    onClick = { scanning = !scanning },
                )

                if (device != null && runtime.peerControlAvailable) {
                    ControlButton(
                        theme = theme,
                        label = "Disconnect",
                        onClick = onDisconnect,
                    )
                } else if (
                    runtime.peerControlAvailable &&
                        runtime.lifecycle == RuntimeLifecycle.RUNNING &&
                        runtime.networkAvailable
                ) {
                    ControlButton(
                        theme = theme,
                        label = "Reconnect",
                        onClick = onReconnect,
                    )
                }
            }
        }

        Spacer(Modifier.height(theme.spacing.xl.toFloat().dp))

        if (scanning) {
            PairingScannerPanel(
                theme = theme,
                onScanned = { bootstrap ->
                    scanning = false
                    onPairingBootstrapScanned(bootstrap)
                },
                onCancel = { scanning = false },
            )
            Spacer(Modifier.height(theme.spacing.xl.toFloat().dp))
        } else if (pairing.stage != PairingJoinerStage.IDLE) {
            PairingProgressPanel(
                theme = theme,
                pairing = pairing,
                onCancel = onCancelPairing,
            )
            Spacer(Modifier.height(theme.spacing.xl.toFloat().dp))
        }

        if (device == null) {
            DisconnectedState(theme, runtime.networkAvailable)
        } else {
            DevicePanel(theme, device)
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
                    "Cross-Lab is ready for an authenticated local device session."
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

        DetailRow(theme, "Owner", device.ownerId ?: "Unavailable")
        DetailRow(theme, "This device", device.localDeviceId ?: "Unavailable")
        DetailRow(theme, "Peer device", device.peerId)
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


@Composable
private fun PairingProgressPanel(
    theme: ThemeDocument,
    pairing: PairingJoinerState,
    onCancel: () -> Unit,
) {
    val colors = theme.colors
    val tone =
        when (pairing.stage) {
            PairingJoinerStage.PAIRED -> StatusTone.ACCENT
            PairingJoinerStage.FAILED -> StatusTone.CRITICAL
            PairingJoinerStage.CANCELLED -> StatusTone.NEUTRAL
            else -> StatusTone.ACCENT
        }
    val label =
        when (pairing.stage) {
            PairingJoinerStage.FINDING_DEVICE -> "Finding device"
            PairingJoinerStage.CONNECTING -> "Connecting securely"
            PairingJoinerStage.SAVING_TRUST -> "Saving trust"
            PairingJoinerStage.FINALIZING -> "Finalizing"
            PairingJoinerStage.PAIRED -> "Paired"
            PairingJoinerStage.FAILED -> "Pairing failed"
            PairingJoinerStage.CANCELLED -> "Pairing cancelled"
            PairingJoinerStage.IDLE -> "Ready"
        }

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
                .padding(theme.spacing.lg.toFloat().dp),
    ) {
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

        if (pairing.ownerId != null || pairing.inviterDeviceId != null) {
            Spacer(Modifier.height(theme.spacing.md.toFloat().dp))
            BasicText(
                text =
                    buildString {
                        pairing.ownerId?.let {
                            append("Owner ")
                            append(it)
                        }
                        if (pairing.ownerId != null && pairing.inviterDeviceId != null) {
                            append(" · ")
                        }
                        pairing.inviterDeviceId?.let {
                            append("inviter ")
                            append(it)
                        }
                    },
                style =
                    theme.typography.scales.body
                        .toTextStyle()
                        .copy(color = colors.foreground.toComposeColor()),
            )
        }

        pairing.message?.let { message ->
            Spacer(Modifier.height(theme.spacing.xs.toFloat().dp))
            BasicText(
                text = message,
                style =
                    theme.typography.scales.caption
                        .toTextStyle()
                        .copy(
                            color =
                                if (pairing.stage == PairingJoinerStage.FAILED) {
                                    colors.destructive.toComposeColor()
                                } else {
                                    colors.mutedForeground.toComposeColor()
                                },
                        ),
            )
        }

        Spacer(Modifier.height(theme.spacing.md.toFloat().dp))
        ControlButton(
            theme = theme,
            label = if (pairing.active) "Cancel pairing" else "Dismiss",
            destructive = pairing.active,
            onClick = onCancel,
        )
    }
}
