use crate::features::devices::{DesktopRuntimeController, DevicesFeatureState};
use gpui_kit::{Context, IntoElement, Render, Window};

use super::{_components::device_content, layout::devices_layout};

pub struct DevicesPage {
    state: DevicesFeatureState,
    _runtime: DesktopRuntimeController,
}

impl DevicesPage {
    pub fn new(cx: &mut Context<Self>) -> Self {
        let runtime = DesktopRuntimeController::from_environment();
        let mut status = runtime.subscribe_status();
        let state = status.borrow().as_ref().map_or_else(
            DevicesFeatureState::empty,
            DevicesFeatureState::from_runtime,
        );

        cx.spawn(async move |this, cx| {
            while status.changed().await.is_ok() {
                let next = status.borrow_and_update().clone();
                if this
                    .update(cx, |page, cx| {
                        match next.as_ref() {
                            Some(status) => page.state.update_runtime(status),
                            None => page.state.clear(),
                        }
                        cx.notify();
                    })
                    .is_err()
                {
                    return;
                }
            }
        })
        .detach();

        Self {
            state,
            _runtime: runtime,
        }
    }
}

impl Render for DevicesPage {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        devices_layout(device_content(&self.state, cx), cx)
    }
}
