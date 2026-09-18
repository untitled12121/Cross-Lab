package dev.crosslab.android.features.appearance

enum class ThemeAppearance {
    LIGHT,
    DARK,
}

enum class ThemeSelection {
    SYSTEM,
    AYU_LIGHT,
    DARKMATTER,
}

enum class SystemAppearance {
    LIGHT,
    DARK,
}

enum class ThemeId {
    AYU_LIGHT,
    DARKMATTER,
}

data class OklchColor(
    val l: Double,
    val c: Double,
    val h: Double,
    val alpha: Double?,
)

data class ThemeColors(
    val background: OklchColor,
    val foreground: OklchColor,
    val surface: OklchColor,
    val surfaceForeground: OklchColor,
    val primary: OklchColor,
    val primaryForeground: OklchColor,
    val secondary: OklchColor,
    val secondaryForeground: OklchColor,
    val muted: OklchColor,
    val mutedForeground: OklchColor,
    val accent: OklchColor,
    val accentForeground: OklchColor,
    val destructive: OklchColor,
    val destructiveForeground: OklchColor,
    val border: OklchColor,
    val input: OklchColor,
    val ring: OklchColor,
    val selection: OklchColor,
)

data class FontFamilies(
    val sans: List<String>,
    val mono: List<String>,
)

data class TextScale(
    val size: Double,
    val lineHeight: Double,
    val weight: Int,
)

data class TextScales(
    val caption: TextScale,
    val body: TextScale,
    val label: TextScale,
    val title: TextScale,
)

data class Typography(
    val families: FontFamilies,
    val scales: TextScales,
)

data class RadiusScale(
    val none: Double,
    val sm: Double,
    val md: Double,
    val lg: Double,
    val xl: Double,
    val full: Double,
)

data class SpacingScale(
    val xxs: Double,
    val xs: Double,
    val sm: Double,
    val md: Double,
    val lg: Double,
    val xl: Double,
    val xxl: Double,
)

data class ThemeMetrics(
    val density: Double,
    val borderWidth: Double,
    val controlHeightCompact: Double,
    val controlHeightDefault: Double,
    val rowHeight: Double,
    val iconSize: Double,
    val iconStroke: Double,
)

data class Shadow(
    val offsetX: Double,
    val offsetY: Double,
    val blur: Double,
    val spread: Double,
    val color: OklchColor,
)

data class Elevation(
    val none: Shadow,
    val raised: Shadow,
)

data class Motion(
    val fastMs: Int,
    val normalMs: Int,
    val easingStandard: List<Double>,
)

data class ThemeDocument(
    val schemaVersion: Int,
    val id: String,
    val displayName: String,
    val appearance: ThemeAppearance,
    val colors: ThemeColors,
    val typography: Typography,
    val radius: RadiusScale,
    val spacing: SpacingScale,
    val metrics: ThemeMetrics,
    val elevation: Elevation,
    val motion: Motion,
)

open class ThemeParseException(message: String, cause: Throwable? = null) :
    IllegalArgumentException(message, cause)

class UnsupportedThemeSchemaException(val version: Int) :
    ThemeParseException("unsupported theme schema version: $version")

class ThemeUnavailableException(val themeId: ThemeId) :
    IllegalStateException("theme is unavailable: $themeId")
