use crosslab_desktop::{
    features::appearance::{ThemeId, install_builtin_theme},
    pages::devices::DevicesPage,
};
use gpui_kit::{
    AppContext, Styled, WindowBounds, WindowOptions,
    component::{Root, theme::ActiveTheme as _},
    px, size,
};

fn main() {
    gpui_kit::application().run(|cx| {
        gpui_kit::init(cx);
        install_builtin_theme(ThemeId::AyuLight, cx)
            .expect("the built-in Ayu Light theme must remain valid");

        let mut window_size = size(px(1040.), px(680.));
        if let Some(display) = cx.primary_display() {
            let display_size = display.bounds().size;
            window_size.width = window_size.width.min(display_size.width * 0.85);
            window_size.height = window_size.height.min(display_size.height * 0.85);
        }

        let window_options = WindowOptions {
            window_bounds: Some(WindowBounds::centered(window_size, cx)),
            window_min_size: Some(gpui_kit::Size {
                width: px(640.),
                height: px(480.),
            }),
            ..Default::default()
        };

        cx.spawn(async move |cx| {
            cx.open_window(window_options, |window, cx| {
                let page = cx.new(DevicesPage::new);
                cx.new(|cx| Root::new(page, window, cx).bg(cx.theme().background))
            })
            .expect("failed to open Cross-Lab desktop window");
        })
        .detach();
    });
}
