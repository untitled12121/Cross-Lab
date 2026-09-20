package dev.crosslab.android.components.ui

import androidx.compose.foundation.BorderStroke
import androidx.compose.foundation.background
import androidx.compose.foundation.border
import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.defaultMinSize
import androidx.compose.foundation.layout.padding
import androidx.compose.ui.graphics.RectangleShape
import androidx.compose.foundation.text.BasicText
import androidx.compose.runtime.Composable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.unit.dp
import dev.crosslab.android.features.appearance.ThemeDocument
import dev.crosslab.android.features.appearance.toComposeColor
import dev.crosslab.android.features.appearance.toTextStyle

@Composable
fun ControlButton(
    theme: ThemeDocument,
    label: String,
    onClick: () -> Unit,
    modifier: Modifier = Modifier,
    active: Boolean = false,
    destructive: Boolean = false,
) {
    val colors = theme.colors
    val background =
        when {
            destructive -> colors.destructive.toComposeColor()
            active -> colors.accent.toComposeColor()
            else -> colors.secondary.toComposeColor()
        }
    val foreground =
        when {
            destructive -> colors.destructiveForeground.toComposeColor()
            active -> colors.accentForeground.toComposeColor()
            else -> colors.secondaryForeground.toComposeColor()
        }

    Box(
        modifier =
            modifier
                .defaultMinSize(minHeight = theme.metrics.controlHeightDefault.toFloat().dp)
                .background(background, RectangleShape)
                .border(
                    BorderStroke(theme.metrics.borderWidth.toFloat().dp, colors.border.toComposeColor()),
                    RectangleShape,
                )
                .clickable(onClick = onClick)
                .padding(horizontal = theme.spacing.lg.toFloat().dp),
        contentAlignment = Alignment.Center,
    ) {
        BasicText(
            text = label,
            style =
                theme.typography.scales.body
                    .toTextStyle()
                    .copy(color = foreground),
        )
    }
}
