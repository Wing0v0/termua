use gpui::{Pixels, px};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RightSidebarTab {
    Notifications,
}

pub struct RightSidebarState {
    pub visible: bool,
    pub width: Pixels,
    pub active_tab: RightSidebarTab,
}

impl Default for RightSidebarState {
    fn default() -> Self {
        Self {
            visible: false,
            width: px(360.0),
            active_tab: RightSidebarTab::Notifications,
        }
    }
}

impl gpui::Global for RightSidebarState {}

impl RightSidebarState {
    pub fn toggle_tab(&mut self, tab: RightSidebarTab) {
        if self.visible && self.active_tab == tab {
            self.visible = false;
        } else {
            self.visible = true;
            self.active_tab = tab;
        }
    }
}
