mod gpui;
mod theme;

pub use gpui::{active_theme, apply_theme, font_weight, install_builtin_theme};
pub use theme::{
    SystemAppearance, ThemeAppearance, ThemeDocument, ThemeError, ThemeId, ThemeSelection,
    load_builtin_theme, parse_theme, resolve_theme,
};

#[cfg(test)]
mod tests;
