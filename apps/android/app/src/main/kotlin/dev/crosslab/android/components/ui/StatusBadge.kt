package dev.crosslab.android.components.ui

import androidx.compose.foundation.BorderStroke
import androidx.compose.foundation.background
import androidx.compose.foundation.border
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.heightIn
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.text.BasicText
import androidx.compose.runtime.Composable
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.RectangleShape
import androidx.compose.ui.semantics.contentDescription
import androidx.compose.ui.semantics.semantics
import androidx.compose.ui.unit.dp
import dev.crosslab.android.features.appearance.TextScale
import dev.crosslab.android.features.appearance.toTextStyle

enum class StatusTone {
    NEUTRAL,
    ACCENT,
    CRITICAL,
}

@Composable
fun StatusBadge(
    label: String,
    tone: StatusTone,
    textScale: TextScale,
    border: Color,
    neutralBackground: Color,
    neutralForeground: Color,
    accentBackground: Color,
    accentForeground: Color,
    criticalBackground: Color,
    criticalForeground: Color,
    modifier: Modifier = Modifier,
) {
    val background =
        when (tone) {
            StatusTone.NEUTRAL -> neutralBackground
            StatusTone.ACCENT -> accentBackground
            StatusTone.CRITICAL -> criticalBackground
        }
    val foreground =
        when (tone) {
            StatusTone.NEUTRAL -> neutralForeground
            StatusTone.ACCENT -> accentForeground
            StatusTone.CRITICAL -> criticalForeground
        }

    Box(
        modifier =
            modifier
                .heightIn(min = 28.dp)
                .border(BorderStroke(1.dp, border), RectangleShape)
                .background(background, RectangleShape)
                .padding(horizontal = 8.dp, vertical = 4.dp)
                .semantics { contentDescription = label },
    ) {
        BasicText(
            text = label,
            style = textScale.toTextStyle().copy(color = foreground),
        )
    }
}
