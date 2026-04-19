use gpui::{
    App, AppContext, Context, FocusHandle, Focusable, InteractiveElement as _, IntoElement as _,
    ParentElement as _, Render, Styled as _, Subscription, Window, div,
};
use gpui_component::{ActiveTheme, v_flex};
use gpui_dock::{Panel, PanelControl, PanelEvent};

use crate::{
    globals::ensure_ctx_global,
    panel::message_panel::MessageCenterView,
    right_sidebar::{RightSidebarState, RightSidebarTab},
};

pub struct RightSidebarView {
    focus_handle: FocusHandle,
    notifications: gpui::Entity<MessageCenterView>,
    _subscriptions: Vec<Subscription>,
}

impl gpui::EventEmitter<PanelEvent> for RightSidebarView {}

impl Focusable for RightSidebarView {
    fn focus_handle(&self, _cx: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

impl RightSidebarView {
    pub fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        ensure_ctx_global::<RightSidebarState, _>(cx);

        let notifications = cx.new(|cx| MessageCenterView::new(window, cx));

        let subs = vec![
            cx.observe_global::<RightSidebarState>(|_, cx| cx.notify()),
            cx.observe_window_activation(window, |_, _, cx| cx.notify()),
        ];

        Self {
            focus_handle: cx.focus_handle(),
            notifications,
            _subscriptions: subs,
        }
    }

    // Intentionally no local tab bar: switching happens via the app-level toggle actions.
}

impl Panel for RightSidebarView {
    fn panel_name(&self) -> &'static str {
        "termua.right_sidebar"
    }

    fn tab_name(&self, _cx: &App) -> Option<gpui::SharedString> {
        Some("Sidebar".into())
    }

    fn title(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl gpui::IntoElement {
        "Sidebar"
    }

    fn closable(&self, _cx: &App) -> bool {
        false
    }

    fn zoomable(&self, _cx: &App) -> Option<PanelControl> {
        None
    }

    fn inner_padding(&self, _cx: &App) -> bool {
        false
    }
}

impl Render for RightSidebarView {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl gpui::IntoElement {
        let tab = cx.global::<RightSidebarState>().active_tab;

        v_flex()
            .id("termua-right-sidebar")
            .debug_selector(|| "termua-right-sidebar".to_string())
            .size_full()
            .min_h_0()
            .items_stretch()
            .bg(cx.theme().background)
            .child(match tab {
                RightSidebarTab::Notifications => div()
                    .flex_1()
                    .min_h_0()
                    .child(self.notifications.clone())
                    .into_any_element(),
            })
    }
}
