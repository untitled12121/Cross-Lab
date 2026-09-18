use crate::features::appearance::{active_theme, font_weight};
use gpui_kit::{
    App, Div, ParentElement as _, SharedString, Styled as _, component::theme::ActiveTheme as _,
    div, px,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StatusTone {
    Neutral,
    Accent,
    Critical,
}

pub fn status_badge(label: impl Into<SharedString>, tone: StatusTone, cx: &App) -> Div {
    let theme = cx.theme();
    let appearance = active_theme(cx);
    let (background, foreground, border) = match tone {
        StatusTone::Neutral => (theme.muted, theme.muted_foreground, theme.border),
        StatusTone::Accent => (theme.accent, theme.accent_foreground, theme.border),
        StatusTone::Critical => (theme.danger, theme.danger_foreground, theme.danger),
    };

    div()
        .flex()
        .items_center()
        .h(px(appearance.metrics.control_height_compact))
        .px(px(appearance.spacing.md))
        .rounded(px(appearance.radius.sm))
        .border_1()
        .border_color(border)
        .bg(background)
        .text_color(foreground)
        .text_size(px(appearance.typography.scales.caption.size))
        .font_weight(font_weight(appearance.typography.scales.caption.weight))
        .child(label.into())
}
