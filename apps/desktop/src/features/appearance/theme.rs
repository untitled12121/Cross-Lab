use core::fmt;

use serde::Deserialize;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ThemeAppearance {
    Light,
    Dark,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ThemeSelection {
    System,
    AyuLight,
    Darkmatter,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SystemAppearance {
    Light,
    Dark,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ThemeId {
    AyuLight,
    Darkmatter,
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct OklchColor {
    pub l: f32,
    pub c: f32,
    pub h: f32,
    pub alpha: Option<f32>,
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ThemeColors {
    pub background: OklchColor,
    pub foreground: OklchColor,
    pub surface: OklchColor,
    pub surface_foreground: OklchColor,
    pub primary: OklchColor,
    pub primary_foreground: OklchColor,
    pub secondary: OklchColor,
    pub secondary_foreground: OklchColor,
    pub muted: OklchColor,
    pub muted_foreground: OklchColor,
    pub accent: OklchColor,
    pub accent_foreground: OklchColor,
    pub destructive: OklchColor,
    pub destructive_foreground: OklchColor,
    pub border: OklchColor,
    pub input: OklchColor,
    pub ring: OklchColor,
    pub selection: OklchColor,
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FontFamilies {
    pub sans: Vec<String>,
    pub mono: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TextScale {
    pub size: f32,
    pub line_height: f32,
    pub weight: u16,
}

#[derive(Debug, Clone, Copy, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TextScales {
    pub caption: TextScale,
    pub body: TextScale,
    pub label: TextScale,
    pub title: TextScale,
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Typography {
    pub families: FontFamilies,
    pub scales: TextScales,
}

#[derive(Debug, Clone, Copy, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RadiusScale {
    pub none: f32,
    pub sm: f32,
    pub md: f32,
    pub lg: f32,
    pub xl: f32,
    pub full: f32,
}

#[derive(Debug, Clone, Copy, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SpacingScale {
    pub xxs: f32,
    pub xs: f32,
    pub sm: f32,
    pub md: f32,
    pub lg: f32,
    pub xl: f32,
    pub xxl: f32,
}

#[derive(Debug, Clone, Copy, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ThemeMetrics {
    pub density: f32,
    pub border_width: f32,
    pub control_height_compact: f32,
    pub control_height_default: f32,
    pub row_height: f32,
    pub icon_size: f32,
    pub icon_stroke: f32,
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Shadow {
    pub offset_x: f32,
    pub offset_y: f32,
    pub blur: f32,
    pub spread: f32,
    pub color: OklchColor,
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Elevation {
    pub none: Shadow,
    pub raised: Shadow,
}

#[derive(Debug, Clone, Copy, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Motion {
    pub fast_ms: u32,
    pub normal_ms: u32,
    pub easing_standard: [f32; 4],
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ThemeDocument {
    pub schema_version: u32,
    pub id: String,
    pub display_name: String,
    pub appearance: ThemeAppearance,
    pub colors: ThemeColors,
    pub typography: Typography,
    pub radius: RadiusScale,
    pub spacing: SpacingScale,
    pub metrics: ThemeMetrics,
    pub elevation: Elevation,
    pub motion: Motion,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ThemeError {
    InvalidDocument(String),
    UnsupportedSchemaVersion(u32),
    ThemeUnavailable(ThemeId),
}

impl fmt::Display for ThemeError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidDocument(error) => write!(formatter, "invalid theme document: {error}"),
            Self::UnsupportedSchemaVersion(version) => {
                write!(formatter, "unsupported theme schema version: {version}")
            }
            Self::ThemeUnavailable(theme) => write!(formatter, "theme is unavailable: {theme:?}"),
        }
    }
}

impl std::error::Error for ThemeError {}

pub fn parse_theme(content: &str) -> Result<ThemeDocument, ThemeError> {
    let theme: ThemeDocument =
        serde_json::from_str(content).map_err(|error| ThemeError::InvalidDocument(error.to_string()))?;

    if theme.schema_version != 1 {
        return Err(ThemeError::UnsupportedSchemaVersion(theme.schema_version));
    }

    validate_theme(&theme)?;
    Ok(theme)
}

pub fn resolve_theme(selection: ThemeSelection, system: SystemAppearance) -> ThemeId {
    match selection {
        ThemeSelection::AyuLight => ThemeId::AyuLight,
        ThemeSelection::Darkmatter => ThemeId::Darkmatter,
        ThemeSelection::System => match system {
            SystemAppearance::Light => ThemeId::AyuLight,
            SystemAppearance::Dark => ThemeId::Darkmatter,
        },
    }
}

pub fn load_builtin_theme(theme: ThemeId) -> Result<ThemeDocument, ThemeError> {
    match theme {
        ThemeId::AyuLight => parse_theme(include_str!("../../../../../design/themes/ayu-light.json")),
        ThemeId::Darkmatter => Err(ThemeError::ThemeUnavailable(ThemeId::Darkmatter)),
    }
}

fn validate_theme(theme: &ThemeDocument) -> Result<(), ThemeError> {
    if !valid_theme_id(&theme.id) {
        return invalid("theme id does not satisfy the canonical id format");
    }
    if !(1..=80).contains(&theme.display_name.chars().count()) {
        return invalid("display name length is outside the canonical range");
    }

    for color in [
        &theme.colors.background,
        &theme.colors.foreground,
        &theme.colors.surface,
        &theme.colors.surface_foreground,
        &theme.colors.primary,
        &theme.colors.primary_foreground,
        &theme.colors.secondary,
        &theme.colors.secondary_foreground,
        &theme.colors.muted,
        &theme.colors.muted_foreground,
        &theme.colors.accent,
        &theme.colors.accent_foreground,
        &theme.colors.destructive,
        &theme.colors.destructive_foreground,
        &theme.colors.border,
        &theme.colors.input,
        &theme.colors.ring,
        &theme.colors.selection,
        &theme.elevation.none.color,
        &theme.elevation.raised.color,
    ] {
        validate_color(color)?;
    }

    validate_font_families(&theme.typography.families)?;
    for scale in [
        theme.typography.scales.caption,
        theme.typography.scales.body,
        theme.typography.scales.label,
        theme.typography.scales.title,
    ] {
        if !positive(scale.size)
            || !positive(scale.line_height)
            || !(100..=900).contains(&scale.weight)
            || scale.weight % 100 != 0
        {
            return invalid("typography scale is outside the canonical range");
        }
    }

    for value in [
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
        theme.metrics.border_width,
        theme.elevation.none.blur,
        theme.elevation.raised.blur,
    ] {
        if !non_negative(value) {
            return invalid("non-negative theme metric is outside the canonical range");
        }
    }

    if !theme.metrics.density.is_finite()
        || theme.metrics.density <= 0.0
        || theme.metrics.density > 2.0
        || !positive(theme.metrics.control_height_compact)
        || !positive(theme.metrics.control_height_default)
        || !positive(theme.metrics.row_height)
        || !positive(theme.metrics.icon_size)
        || !positive(theme.metrics.icon_stroke)
    {
        return invalid("theme metrics are outside the canonical range");
    }

    for value in [
        theme.elevation.none.offset_x,
        theme.elevation.none.offset_y,
        theme.elevation.none.spread,
        theme.elevation.raised.offset_x,
        theme.elevation.raised.offset_y,
        theme.elevation.raised.spread,
    ] {
        if !value.is_finite() {
            return invalid("shadow metric must be finite");
        }
    }

    if theme.motion.fast_ms > 1000
        || theme.motion.normal_ms > 2000
        || theme
            .motion
            .easing_standard
            .iter()
            .any(|value| !in_range(*value, 0.0, 1.0))
    {
        return invalid("motion values are outside the canonical range");
    }

    Ok(())
}

fn validate_color(color: &OklchColor) -> Result<(), ThemeError> {
    if !in_range(color.l, 0.0, 1.0)
        || !in_range(color.c, 0.0, 0.5)
        || !in_range(color.h, 0.0, 360.0)
        || color
            .alpha
            .is_some_and(|alpha| !in_range(alpha, 0.0, 1.0))
    {
        return invalid("OKLCH value is outside the canonical range");
    }
    Ok(())
}

fn validate_font_families(families: &FontFamilies) -> Result<(), ThemeError> {
    if families.sans.is_empty()
        || families.mono.is_empty()
        || families
            .sans
            .iter()
            .chain(families.mono.iter())
            .any(String::is_empty)
    {
        return invalid("font family lists must contain non-empty names");
    }
    Ok(())
}

fn valid_theme_id(id: &str) -> bool {
    if !(1..=64).contains(&id.len())
        || id.starts_with('-')
        || id.ends_with('-')
        || id.contains("--")
    {
        return false;
    }

    id.bytes()
        .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'-')
}

fn positive(value: f32) -> bool {
    value.is_finite() && value > 0.0
}

fn non_negative(value: f32) -> bool {
    value.is_finite() && value >= 0.0
}

fn in_range(value: f32, minimum: f32, maximum: f32) -> bool {
    value.is_finite() && (minimum..=maximum).contains(&value)
}

fn invalid<T>(message: &str) -> Result<T, ThemeError> {
    Err(ThemeError::InvalidDocument(message.to_owned()))
}
