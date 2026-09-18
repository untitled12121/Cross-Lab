use gpui_kit::{
    App, Div, InteractiveElement as _, ParentElement as _, StatefulInteractiveElement as _,
    Styled as _, div, px,
    base::Button,
    component::theme::ActiveTheme as _,
};

pub(super) fn devices_layout(content: impl gpui_kit::IntoElement, cx: &App) -> Div {
    let theme = cx.theme();

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
                .p(px(12.))
                .gap(px(12.))
                .child(
                    div()
                        .text_size(px(15.))
                        .font_semibold()
                        .child("Cross-Lab"),
                )
                .child(
                    Button::new("nav-devices")
                        .accessibility_label("Devices")
                        .on_click(|_, _, _| {})
                        .w_full()
                        .h(px(32.))
                        .px(px(8.))
                        .justify_start()
                        .border_1()
                        .border_color(theme.border)
                        .bg(theme.accent)
                        .text_color(theme.accent_foreground)
                        .hover(|style| style.bg(theme.secondary))
                        .focus_visible(|style| style.border_color(theme.ring))
                        .child("Devices"),
                ),
        )
        .child(div().flex_1().min_w_0().h_full().child(content))
}
