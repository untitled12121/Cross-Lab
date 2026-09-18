package dev.crosslab.android.features.appearance

import org.json.JSONArray
import org.json.JSONException
import org.json.JSONObject

object ThemeParser {
    fun parse(content: String): ThemeDocument {
        try {
            val root = JSONObject(content)
            root.requireExactKeys(
                "schema_version",
                "id",
                "display_name",
                "appearance",
                "colors",
                "typography",
                "radius",
                "spacing",
                "metrics",
                "elevation",
                "motion",
            )

            val schemaVersion = root.getInt("schema_version")
            if (schemaVersion != 1) {
                throw UnsupportedThemeSchemaException(schemaVersion)
            }

            return ThemeDocument(
                schemaVersion = schemaVersion,
                id = root.getString("id"),
                displayName = root.getString("display_name"),
                appearance = parseAppearance(root.getString("appearance")),
                colors = parseColors(root.getJSONObject("colors")),
                typography = parseTypography(root.getJSONObject("typography")),
                radius = parseRadius(root.getJSONObject("radius")),
                spacing = parseSpacing(root.getJSONObject("spacing")),
                metrics = parseMetrics(root.getJSONObject("metrics")),
                elevation = parseElevation(root.getJSONObject("elevation")),
                motion = parseMotion(root.getJSONObject("motion")),
            ).also(::validate)
        } catch (error: UnsupportedThemeSchemaException) {
            throw error
        } catch (error: ThemeParseException) {
            throw error
        } catch (error: JSONException) {
            throw ThemeParseException("invalid theme document", error)
        }
    }

    private fun parseAppearance(value: String): ThemeAppearance =
        when (value) {
            "light" -> ThemeAppearance.LIGHT
            "dark" -> ThemeAppearance.DARK
            else -> invalid("theme appearance is invalid")
        }

    private fun parseColors(json: JSONObject): ThemeColors {
        json.requireExactKeys(
            "background",
            "foreground",
            "surface",
            "surface_foreground",
            "primary",
            "primary_foreground",
            "secondary",
            "secondary_foreground",
            "muted",
            "muted_foreground",
            "accent",
            "accent_foreground",
            "destructive",
            "destructive_foreground",
            "border",
            "input",
            "ring",
            "selection",
        )
        return ThemeColors(
            background = parseColor(json.getJSONObject("background")),
            foreground = parseColor(json.getJSONObject("foreground")),
            surface = parseColor(json.getJSONObject("surface")),
            surfaceForeground = parseColor(json.getJSONObject("surface_foreground")),
            primary = parseColor(json.getJSONObject("primary")),
            primaryForeground = parseColor(json.getJSONObject("primary_foreground")),
            secondary = parseColor(json.getJSONObject("secondary")),
            secondaryForeground = parseColor(json.getJSONObject("secondary_foreground")),
            muted = parseColor(json.getJSONObject("muted")),
            mutedForeground = parseColor(json.getJSONObject("muted_foreground")),
            accent = parseColor(json.getJSONObject("accent")),
            accentForeground = parseColor(json.getJSONObject("accent_foreground")),
            destructive = parseColor(json.getJSONObject("destructive")),
            destructiveForeground = parseColor(json.getJSONObject("destructive_foreground")),
            border = parseColor(json.getJSONObject("border")),
            input = parseColor(json.getJSONObject("input")),
            ring = parseColor(json.getJSONObject("ring")),
            selection = parseColor(json.getJSONObject("selection")),
        )
    }

    private fun parseColor(json: JSONObject): OklchColor {
        json.requireOnly("l", "c", "h", "alpha")
        for (required in listOf("l", "c", "h")) {
            if (!json.has(required)) invalid("OKLCH value is missing $required")
        }
        return OklchColor(
            l = json.getDouble("l"),
            c = json.getDouble("c"),
            h = json.getDouble("h"),
            alpha = if (json.has("alpha")) json.getDouble("alpha") else null,
        )
    }

    private fun parseTypography(json: JSONObject): Typography {
        json.requireExactKeys("families", "scales")
        val families = json.getJSONObject("families").also {
            it.requireExactKeys("sans", "mono")
        }
        val scales = json.getJSONObject("scales").also {
            it.requireExactKeys("caption", "body", "label", "title")
        }
        return Typography(
            families = FontFamilies(
                sans = families.getJSONArray("sans").strings(),
                mono = families.getJSONArray("mono").strings(),
            ),
            scales = TextScales(
                caption = parseTextScale(scales.getJSONObject("caption")),
                body = parseTextScale(scales.getJSONObject("body")),
                label = parseTextScale(scales.getJSONObject("label")),
                title = parseTextScale(scales.getJSONObject("title")),
            ),
        )
    }

    private fun parseTextScale(json: JSONObject): TextScale {
        json.requireExactKeys("size", "line_height", "weight")
        return TextScale(
            size = json.getDouble("size"),
            lineHeight = json.getDouble("line_height"),
            weight = json.getInt("weight"),
        )
    }

    private fun parseRadius(json: JSONObject): RadiusScale {
        json.requireExactKeys("none", "sm", "md", "lg", "xl", "full")
        return RadiusScale(
            none = json.getDouble("none"),
            sm = json.getDouble("sm"),
            md = json.getDouble("md"),
            lg = json.getDouble("lg"),
            xl = json.getDouble("xl"),
            full = json.getDouble("full"),
        )
    }

    private fun parseSpacing(json: JSONObject): SpacingScale {
        json.requireExactKeys("xxs", "xs", "sm", "md", "lg", "xl", "xxl")
        return SpacingScale(
            xxs = json.getDouble("xxs"),
            xs = json.getDouble("xs"),
            sm = json.getDouble("sm"),
            md = json.getDouble("md"),
            lg = json.getDouble("lg"),
            xl = json.getDouble("xl"),
            xxl = json.getDouble("xxl"),
        )
    }

    private fun parseMetrics(json: JSONObject): ThemeMetrics {
        json.requireExactKeys(
            "density",
            "border_width",
            "control_height_compact",
            "control_height_default",
            "row_height",
            "icon_size",
            "icon_stroke",
        )
        return ThemeMetrics(
            density = json.getDouble("density"),
            borderWidth = json.getDouble("border_width"),
            controlHeightCompact = json.getDouble("control_height_compact"),
            controlHeightDefault = json.getDouble("control_height_default"),
            rowHeight = json.getDouble("row_height"),
            iconSize = json.getDouble("icon_size"),
            iconStroke = json.getDouble("icon_stroke"),
        )
    }

    private fun parseElevation(json: JSONObject): Elevation {
        json.requireExactKeys("none", "raised")
        return Elevation(
            none = parseShadow(json.getJSONObject("none")),
            raised = parseShadow(json.getJSONObject("raised")),
        )
    }

    private fun parseShadow(json: JSONObject): Shadow {
        json.requireExactKeys("offset_x", "offset_y", "blur", "spread", "color")
        return Shadow(
            offsetX = json.getDouble("offset_x"),
            offsetY = json.getDouble("offset_y"),
            blur = json.getDouble("blur"),
            spread = json.getDouble("spread"),
            color = parseColor(json.getJSONObject("color")),
        )
    }

    private fun parseMotion(json: JSONObject): Motion {
        json.requireExactKeys("fast_ms", "normal_ms", "easing_standard")
        val easing = json.getJSONArray("easing_standard")
        if (easing.length() != 4) invalid("motion easing must contain four values")
        return Motion(
            fastMs = json.getInt("fast_ms"),
            normalMs = json.getInt("normal_ms"),
            easingStandard = List(easing.length()) { easing.getDouble(it) },
        )
    }

    private fun validate(theme: ThemeDocument) {
        if (!THEME_ID.matches(theme.id)) invalid("theme id does not satisfy the canonical id format")
        if (theme.displayName.length !in 1..80) invalid("display name length is outside the canonical range")

        theme.colors.all()
            .plus(theme.elevation.none.color)
            .plus(theme.elevation.raised.color)
            .forEach(::validateColor)

        val families = theme.typography.families
        if (families.sans.isEmpty() || families.mono.isEmpty() ||
            (families.sans + families.mono).any(String::isEmpty)
        ) {
            invalid("font family lists must contain non-empty names")
        }

        listOf(
            theme.typography.scales.caption,
            theme.typography.scales.body,
            theme.typography.scales.label,
            theme.typography.scales.title,
        ).forEach { scale ->
            if (!scale.size.positive() || !scale.lineHeight.positive() ||
                scale.weight !in 100..900 || scale.weight % 100 != 0
            ) {
                invalid("typography scale is outside the canonical range")
            }
        }

        listOf(
            theme.radius.none,
            theme.radius.sm,
            theme.radius.md,
            theme.radius.lg,
            theme.radius.xl,
            theme.radius.full,
            theme.spacing.xxs,
            theme.spacing.xs,
            theme.spacing.sm,
            theme.spacing.md,
            theme.spacing.lg,
            theme.spacing.xl,
            theme.spacing.xxl,
            theme.metrics.borderWidth,
            theme.elevation.none.blur,
            theme.elevation.raised.blur,
        ).forEach { value ->
            if (!value.nonNegative()) invalid("non-negative theme metric is outside the canonical range")
        }

        val metrics = theme.metrics
        if (!metrics.density.finiteIn(0.0, 2.0) || metrics.density == 0.0 ||
            !metrics.controlHeightCompact.positive() ||
            !metrics.controlHeightDefault.positive() ||
            !metrics.rowHeight.positive() ||
            !metrics.iconSize.positive() ||
            !metrics.iconStroke.positive()
        ) {
            invalid("theme metrics are outside the canonical range")
        }

        listOf(
            theme.elevation.none.offsetX,
            theme.elevation.none.offsetY,
            theme.elevation.none.spread,
            theme.elevation.raised.offsetX,
            theme.elevation.raised.offsetY,
            theme.elevation.raised.spread,
        ).forEach { value ->
            if (!value.isFinite()) invalid("shadow metric must be finite")
        }

        if (theme.motion.fastMs !in 0..1000 ||
            theme.motion.normalMs !in 0..2000 ||
            theme.motion.easingStandard.any { !it.finiteIn(0.0, 1.0) }
        ) {
            invalid("motion values are outside the canonical range")
        }
    }

    private fun validateColor(color: OklchColor) {
        if (!color.l.finiteIn(0.0, 1.0) ||
            !color.c.finiteIn(0.0, 0.5) ||
            !color.h.finiteIn(0.0, 360.0) ||
            color.alpha?.let { !it.finiteIn(0.0, 1.0) } == true
        ) {
            invalid("OKLCH value is outside the canonical range")
        }
    }

    private fun JSONObject.requireExactKeys(vararg expectedKeys: String) {
        val expected = expectedKeys.toSet()
        val actual = this.keys().asSequence().toSet()
        if (actual != expected) invalid("theme document fields do not match the canonical contract")
    }

    private fun JSONObject.requireOnly(vararg allowedKeys: String) {
        val allowed = allowedKeys.toSet()
        if (this.keys().asSequence().any { it !in allowed }) {
            invalid("theme document contains an unknown field")
        }
    }

    private fun JSONArray.strings(): List<String> =
        List(length()) { getString(it) }

    private fun ThemeColors.all(): List<OklchColor> =
        listOf(
            background,
            foreground,
            surface,
            surfaceForeground,
            primary,
            primaryForeground,
            secondary,
            secondaryForeground,
            muted,
            mutedForeground,
            accent,
            accentForeground,
            destructive,
            destructiveForeground,
            border,
            input,
            ring,
            selection,
        )

    private fun Double.positive(): Boolean = isFinite() && this > 0.0

    private fun Double.nonNegative(): Boolean = isFinite() && this >= 0.0

    private fun Double.finiteIn(minimum: Double, maximum: Double): Boolean =
        isFinite() && this in minimum..maximum

    private fun invalid(message: String): Nothing = throw ThemeParseException(message)

    private val THEME_ID = Regex("^[a-z0-9]+(?:-[a-z0-9]+)*$")
}
