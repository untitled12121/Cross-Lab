mod theme;

pub use theme::{
    SystemAppearance, ThemeAppearance, ThemeDocument, ThemeError, ThemeId, ThemeSelection,
    load_builtin_theme, parse_theme, resolve_theme,
};

#[cfg(test)]
mod tests;
