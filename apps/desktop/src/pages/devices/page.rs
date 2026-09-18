use crate::features::devices::DevicesFeatureState;
use gpui_kit::{Context, IntoElement, Render, Window};

use super::{_components::device_content, layout::devices_layout};

pub struct DevicesPage {
    state: DevicesFeatureState,
}

impl DevicesPage {
    pub const fn new() -> Self {
        Self {
            state: DevicesFeatureState::empty(),
        }
    }
}

impl Default for DevicesPage {
    fn default() -> Self {
        Self::new()
    }
}

impl Render for DevicesPage {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        devices_layout(device_content(&self.state, cx), cx)
    }
}
