use super::{
    SystemAppearance, ThemeAppearance, ThemeError, ThemeId, ThemeSelection, load_builtin_theme,
    parse_theme, resolve_theme,
};

const VALID_THEME: &str = include_str!("../../../../../design/fixtures/theme-v1-valid.json");
const INVALID_THEME: &str = include_str!("../../../../../design/fixtures/theme-v1-invalid.json");

#[test]
fn parses_the_canonical_theme_contract() {
    let theme = parse_theme(VALID_THEME).expect("valid fixture should parse");

    assert_eq!(theme.schema_version, 1);
    assert_eq!(theme.id, "fixture-light");
    assert_eq!(theme.appearance, ThemeAppearance::Light);
    assert_eq!(theme.radius.none, 0.0);
    assert_eq!(theme.radius.full, 0.0);
    assert_eq!(theme.colors.selection.alpha, Some(0.2));
    assert_eq!(theme.metrics.row_height, 32.0);
}

#[test]
fn missing_required_theme_field_fails_closed() {
    let error = parse_theme(INVALID_THEME).expect_err("missing selection must fail");

    assert!(matches!(error, ThemeError::InvalidDocument(_)));
}

#[test]
fn rejects_unknown_schema_version() {
    let unsupported = VALID_THEME.replacen(""schema_version": 1", ""schema_version": 2", 1);

    assert_eq!(
        parse_theme(&unsupported),
        Err(ThemeError::UnsupportedSchemaVersion(2))
    );
}

#[test]
fn system_selection_resolves_from_injected_appearance() {
    assert_eq!(
        resolve_theme(ThemeSelection::System, SystemAppearance::Light),
        ThemeId::AyuLight
    );
    assert_eq!(
        resolve_theme(ThemeSelection::System, SystemAppearance::Dark),
        ThemeId::Darkmatter
    );
    assert_eq!(
        resolve_theme(ThemeSelection::AyuLight, SystemAppearance::Dark),
        ThemeId::AyuLight
    );
    assert_eq!(
        resolve_theme(ThemeSelection::Darkmatter, SystemAppearance::Light),
        ThemeId::Darkmatter
    );
}

#[test]
fn missing_darkmatter_is_explicitly_unavailable() {
    assert_eq!(
        load_builtin_theme(ThemeId::Darkmatter),
        Err(ThemeError::ThemeUnavailable(ThemeId::Darkmatter))
    );
}

#[test]
fn ayu_light_is_loadable_without_a_host_appearance_dependency() {
    let theme = load_builtin_theme(ThemeId::AyuLight).expect("Ayu Light should be available");

    assert_eq!(theme.id, "ayu-light");
    assert_eq!(theme.appearance, ThemeAppearance::Light);
}
