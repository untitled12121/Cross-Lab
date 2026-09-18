package dev.crosslab.android.features.appearance

import androidx.compose.ui.graphics.Color
import androidx.compose.ui.text.TextStyle
import androidx.compose.ui.text.font.FontFamily
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.sp
import kotlin.math.cos
import kotlin.math.pow
import kotlin.math.sin

fun OklchColor.toComposeColor(): Color {
    val hue = Math.toRadians(h)
    val a = c * cos(hue)
    val b = c * sin(hue)

    val lPrime = l + 0.39633778 * a + 0.21580376 * b
    val mPrime = l - 0.105561346 * a - 0.06385417 * b
    val sPrime = l - 0.08948418 * a - 1.2914855 * b

    val linearL = lPrime * lPrime * lPrime
    val linearM = mPrime * mPrime * mPrime
    val linearS = sPrime * sPrime * sPrime

    val red = 4.0767417 * linearL - 3.3077116 * linearM + 0.23096994 * linearS
    val green = -1.268438 * linearL + 2.6097574 * linearM - 0.3413194 * linearS
    val blue = -0.0041960863 * linearL - 0.7034186 * linearM + 1.7076147 * linearS

    return Color(
        red = linearToSrgb(red).coerceIn(0.0, 1.0).toFloat(),
        green = linearToSrgb(green).coerceIn(0.0, 1.0).toFloat(),
        blue = linearToSrgb(blue).coerceIn(0.0, 1.0).toFloat(),
        alpha = (alpha ?: 1.0).toFloat(),
    )
}

fun TextScale.toTextStyle(): TextStyle =
    TextStyle(
        fontFamily = FontFamily.SansSerif,
        fontSize = size.toFloat().sp,
        lineHeight = lineHeight.toFloat().sp,
        fontWeight = FontWeight(weight),
    )

private fun linearToSrgb(channel: Double): Double =
    if (channel <= 0.0031308) {
        channel * 12.92
    } else {
        1.055 * channel.pow(1.0 / 2.4) - 0.055
    }
