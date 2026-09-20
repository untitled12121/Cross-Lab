use crate::features::appearance::{active_theme, font_weight};
use gpui_kit::{
    App, Div, ParentElement as _, Styled as _, component::theme::ActiveTheme as _, div, px,
};

pub(super) fn control_center_layout(
    navigation: impl gpui_kit::IntoElement,
    content: impl gpui_kit::IntoElement,
    cx: &App,
) -> Div {
    let theme = cx.theme();
    let appearance = active_theme(cx);

    div()
        .flex()
        .size_full()
        .bg(theme.background)
        .text_color(theme.foreground)
        .child(
            div()
                .flex()
                .flex_col()
                .flex_shrink_0()
                .w(px(184.))
                .h_full()
                .border_r_1()
                .border_color(theme.border)
                .bg(theme.popover)
                .p(px(appearance.spacing.lg))
                .gap(px(appearance.spacing.lg))
                .child(
                    div()
                        .text_size(px(appearance.typography.scales.label.size))
                        .font_weight(font_weight(appearance.typography.scales.label.weight))
                        .child("Cross-Lab"),
                )
                .child(navigation),
        )
        .child(div().flex_1().min_w_0().h_full().child(content))
}
