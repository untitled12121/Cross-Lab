use crate::features::appearance::{active_theme, font_weight};
use gpui_kit::{
    App, Div, InteractiveElement as _, ParentElement as _, StatefulInteractiveElement as _,
    Styled as _, base::Button, component::theme::ActiveTheme as _, div, px,
};

pub(super) fn devices_layout(content: impl gpui_kit::IntoElement, cx: &App) -> Div {
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
                .child(
                    Button::new("nav-devices")
                        .accessibility_label("Devices")
                        .on_click(|_, _, _| {})
                        .w_full()
                        .h(px(appearance.metrics.control_height_default))
                        .px(px(appearance.spacing.md))
                        .justify_start()
                        .rounded(px(appearance.radius.sm))
                        .border_1()
                        .border_color(theme.border)
                        .bg(theme.accent)
                        .text_color(theme.accent_foreground)
                        .text_size(px(appearance.typography.scales.body.size))
                        .font_weight(font_weight(appearance.typography.scales.body.weight))
                        .hover(|style| style.bg(theme.secondary))
                        .focus_visible(|style| style.border_color(theme.ring))
                        .child("Devices"),
                ),
        )
        .child(div().flex_1().min_w_0().h_full().child(content))
}
