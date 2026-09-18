use gpui_kit::{
    App, Hsla, Rgba, SharedString,
    component::theme::{Theme, ThemeMode},
    px,
};

use super::theme::OklchColor;
use super::{ThemeAppearance, ThemeDocument, ThemeError, ThemeId, load_builtin_theme};

pub fn install_builtin_theme(theme_id: ThemeId, cx: &mut App) -> Result<ThemeDocument, ThemeError> {
    let theme = load_builtin_theme(theme_id)?;
    apply_theme(&theme, cx);
    Ok(theme)
}

pub fn apply_theme(source: &ThemeDocument, cx: &mut App) {
    let existing_mono = Theme::global(cx).mono_font_family.clone();
    let sans = native_font_family(
        &source.typography.families.sans,
        ".SystemUIFont",
        "system-ui",
    );
    let mono = native_font_family(
        &source.typography.families.mono,
        existing_mono.as_ref(),
        "monospace",
    );

    {
        let theme = Theme::global_mut(cx);
        theme.mode = match source.appearance {
            ThemeAppearance::Light => ThemeMode::Light,
            ThemeAppearance::Dark => ThemeMode::Dark,
        };

        theme.background = to_hsla(&source.colors.background);
        theme.foreground = to_hsla(&source.colors.foreground);
        theme.popover = to_hsla(&source.colors.surface);
        theme.popover_foreground = to_hsla(&source.colors.surface_foreground);
        theme.primary = to_hsla(&source.colors.primary);
        theme.primary_foreground = to_hsla(&source.colors.primary_foreground);
        theme.secondary = to_hsla(&source.colors.secondary);
        theme.secondary_foreground = to_hsla(&source.colors.secondary_foreground);
        theme.muted = to_hsla(&source.colors.muted);
        theme.muted_foreground = to_hsla(&source.colors.muted_foreground);
        theme.accent = to_hsla(&source.colors.accent);
        theme.accent_foreground = to_hsla(&source.colors.accent_foreground);
        theme.danger = to_hsla(&source.colors.destructive);
        theme.danger_foreground = to_hsla(&source.colors.destructive_foreground);
        theme.border = to_hsla(&source.colors.border);
        theme.input = to_hsla(&source.colors.input);
        theme.ring = to_hsla(&source.colors.ring);
        theme.selection = to_hsla(&source.colors.selection);

        theme.radius = px(source.radius.md);
        theme.radius_lg = px(source.radius.lg);
        theme.font_family = sans;
        theme.font_size = px(source.typography.scales.body.size);
        theme.mono_font_family = mono;
        theme.mono_font_size = px(source.typography.scales.body.size);
        theme.shadow = source.elevation.raised.blur > 0.0
            && source.elevation.raised.color.alpha.unwrap_or(1.0) > 0.0;
    }

    Theme::sync_base(cx);
}

fn native_font_family(
    families: &[String],
    generic_native: &str,
    generic_name: &str,
) -> SharedString {
    if families.iter().any(|family| family == generic_name) {
        return generic_native.to_owned().into();
    }

    families
        .first()
        .cloned()
        .unwrap_or_else(|| generic_native.to_owned())
        .into()
}

fn to_hsla(color: &OklchColor) -> Hsla {
    let hue = color.h.to_radians();
    let a = color.c * hue.cos();
    let b = color.c * hue.sin();

    let l_ = color.l + 0.396_337_78 * a + 0.215_803_76 * b;
    let m_ = color.l - 0.105_561_346 * a - 0.063_854_17 * b;
    let s_ = color.l - 0.089_484_18 * a - 1.291_485_5 * b;

    let l = l_ * l_ * l_;
    let m = m_ * m_ * m_;
    let s = s_ * s_ * s_;

    let red = 4.076_741_7 * l - 3.307_711_6 * m + 0.230_969_94 * s;
    let green = -1.268_438 * l + 2.609_757_4 * m - 0.341_319_4 * s;
    let blue = -0.004_196_086_3 * l - 0.703_418_6 * m + 1.707_614_7 * s;

    Rgba {
        r: linear_to_srgb(red).clamp(0.0, 1.0),
        g: linear_to_srgb(green).clamp(0.0, 1.0),
        b: linear_to_srgb(blue).clamp(0.0, 1.0),
        a: color.alpha.unwrap_or(1.0),
    }
    .into()
}

fn linear_to_srgb(channel: f32) -> f32 {
    if channel <= 0.003_130_8 {
        channel * 12.92
    } else {
        1.055 * channel.powf(1.0 / 2.4) - 0.055
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn oklch_mapping_preserves_alpha_and_neutral_extremes() {
        let transparent_black = to_hsla(&OklchColor {
            l: 0.0,
            c: 0.0,
            h: 0.0,
            alpha: Some(0.25),
        });
        let white = to_hsla(&OklchColor {
            l: 1.0,
            c: 0.0,
            h: 0.0,
            alpha: None,
        });

        assert!((transparent_black.a - 0.25).abs() < f32::EPSILON);
        assert!(transparent_black.to_rgb().r < 0.001);
        assert!(white.to_rgb().r > 0.999);
        assert!(white.to_rgb().g > 0.999);
        assert!(white.to_rgb().b > 0.999);
    }
}
