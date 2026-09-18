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

pub fn parse_theme(_content: &str) -> Result<ThemeDocument, ThemeError> {
    unimplemented!("RED: implement canonical theme parsing")
}

pub fn resolve_theme(
    _selection: ThemeSelection,
    _system: SystemAppearance,
) -> ThemeId {
    unimplemented!("RED: implement pure appearance resolution")
}

pub fn load_builtin_theme(_theme: ThemeId) -> Result<ThemeDocument, ThemeError> {
    unimplemented!("RED: implement built-in theme loading")
}
