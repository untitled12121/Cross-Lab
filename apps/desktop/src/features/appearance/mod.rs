mod theme;

pub use theme::{
    ThemeAppearance, ThemeDocument, ThemeError, ThemeId, ThemeSelection, SystemAppearance,
    load_builtin_theme, parse_theme, resolve_theme,
};

#[cfg(test)]
mod tests;
