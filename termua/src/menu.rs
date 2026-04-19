//! Application menus and top-level actions.

use std::{collections::HashMap, env, process::Command};

use gpui::{App, KeyBinding, Menu, MenuItem, actions};
use gpui_term::TerminalType;
use rust_i18n::t;

use crate::{
    PendingCommand, TermuaAppState,
    config::SettingsWindow,
    new_session::NewSessionWindow,
    right_sidebar::{RightSidebarState, RightSidebarTab},
};

actions!(
    termua,
    [
        Quit,
        OpenNewSession,
        NewLocalTerminal,
        NewWindow,
        OpenSettings,
        OpenSftp,
        ToggleSessionsSidebar,
        ToggleMessagesSidebar,
    ]
);

pub(crate) fn register(cx: &mut App) {
    cx.on_action(quit);
    cx.on_action(open_new_session);
    cx.on_action(new_local_terminal);
    cx.on_action(new_window);
    cx.on_action(open_settings);
    cx.on_action(toggle_sessions_sidebar);
    cx.on_action(toggle_messages_sidebar);
}

fn quit(_: &Quit, cx: &mut App) {
    let active_root = cx
        .active_window()
        .and_then(|window| window.downcast::<gpui_component::Root>());
    let main_window = cx
        .try_global::<TermuaAppState>()
        .and_then(|state| state.main_window);

    cx.defer(move |cx| {
        let dispatch_to_root =
            |root_handle: gpui::WindowHandle<gpui_component::Root>, cx: &mut App| -> bool {
                root_handle
                    .update(cx, |root, window, cx| {
                        let Ok(termua) = root
                            .view()
                            .clone()
                            .downcast::<crate::window::main_window::TermuaWindow>()
                        else {
                            return false;
                        };

                        termua.update(cx, |this, cx| this.request_quit(window, cx));
                        true
                    })
                    .unwrap_or(false)
            };

        if let Some(root) = active_root
            && dispatch_to_root(root, cx)
        {
            return;
        }

        if let Some(root) = main_window
            && dispatch_to_root(root, cx)
        {
            return;
        }

        cx.quit();
    });
}

fn open_new_session(_: &OpenNewSession, cx: &mut App) {
    if let Err(err) = NewSessionWindow::open(cx) {
        log::error!("OpenNewSession: failed to open window: {err:#}");
    }
}

fn new_local_terminal(_: &NewLocalTerminal, cx: &mut App) {
    if cx.global::<TermuaAppState>().main_window.is_none() {
        log::warn!("NewLocalTerminal: main window not ready yet");
        return;
    }

    cx.global_mut::<TermuaAppState>()
        .pending_command(PendingCommand::OpenLocalTerminal {
            backend_type: TerminalType::WezTerm,
            env: HashMap::new(),
        });
    cx.refresh_windows();
}

fn new_window(_: &NewWindow, cx: &mut App) {
    if cx.global::<TermuaAppState>().main_window.is_none() {
        log::warn!("NewWindow: main window not ready yet");
        return;
    };

    match env::current_exe() {
        Ok(path) => {
            let mut child = Command::new(path);
            #[cfg(windows)]
            {
                use std::os::windows::process::CommandExt;

                use windows::Win32::System::Threading::CREATE_NEW_PROCESS_GROUP;
                child.creation_flags(CREATE_NEW_PROCESS_GROUP.0);
            }

            #[cfg(unix)]
            {
                use std::os::unix::prelude::CommandExt;
                unsafe {
                    child.pre_exec(|| {
                        let _ = rustix::process::setsid();
                        Ok(())
                    });
                }
            }

            if let Err(err) = child.spawn() {
                log::error!("failed to launch new window: {err}");
            }
        }
        Err(err) => log::error!("failed to get current exe path: {err}"),
    }
}

fn open_settings(_: &OpenSettings, cx: &mut App) {
    let existing = cx.global::<TermuaAppState>().settings_window;
    if let Some(handle) = existing {
        if handle
            .update(cx, |_, window, _cx| {
                window.activate_window();
            })
            .is_ok()
        {
            return;
        }
    }

    match SettingsWindow::open(cx) {
        Ok(handle) => {
            cx.global_mut::<TermuaAppState>().settings_window = Some(handle);
        }
        Err(err) => log::error!("OpenSettings: failed to open settings window: {err:#}"),
    }
}

pub(crate) fn toggle_sessions_sidebar(_: &ToggleSessionsSidebar, cx: &mut App) {
    {
        let state = cx.global_mut::<TermuaAppState>();
        state.sessions_sidebar_visible = !state.sessions_sidebar_visible;
    }
    cx.refresh_windows();
}

pub(crate) fn toggle_messages_sidebar(_: &ToggleMessagesSidebar, cx: &mut App) {
    if cx.try_global::<RightSidebarState>().is_none() {
        cx.set_global(RightSidebarState::default());
    }
    cx.global_mut::<RightSidebarState>()
        .toggle_tab(RightSidebarTab::Notifications);
    cx.refresh_windows();
}

pub(crate) fn bind_menu_shortcuts(cx: &mut App) {
    #[cfg(not(target_os = "macos"))]
    cx.bind_keys([
        KeyBinding::new("ctrl-shift-n", OpenNewSession, None),
        KeyBinding::new("ctrl-n", NewLocalTerminal, None),
        KeyBinding::new("ctrl-q", Quit, None),
        KeyBinding::new("ctrl-,", OpenSettings, None),
        KeyBinding::new("ctrl-shift-m", ToggleMessagesSidebar, None),
    ]);

    #[cfg(target_os = "macos")]
    cx.bind_keys([
        KeyBinding::new("cmd-shift-n", OpenNewSession, None),
        KeyBinding::new("cmd-n", NewLocalTerminal, None),
        KeyBinding::new("cmd-q", Quit, None),
        KeyBinding::new("cmd-,", OpenSettings, None),
        KeyBinding::new("cmd-shift-a", ToggleAssistantSidebar, None),
        KeyBinding::new("cmd-shift-m", ToggleMessagesSidebar, None),
    ]);
}

pub(crate) fn build_menus(multi_exec_enabled: bool) -> Vec<Menu> {
    // menus[0] is the fold/app menu (menubar crate expects this).
    vec![
        Menu {
            name: t!("Menu.App.Name").to_string().into(),
            items: vec![
                MenuItem::action(t!("Menu.App.OpenSettings").to_string(), OpenSettings),
                MenuItem::separator(),
                MenuItem::action(t!("Menu.App.Quit").to_string(), Quit),
            ],
        },
        Menu {
            name: t!("Menu.Session.Name").to_string().into(),
            items: vec![
                MenuItem::action(t!("Menu.Session.NewSession").to_string(), OpenNewSession),
                MenuItem::action(t!("Menu.Session.NewWindow").to_string(), NewWindow),
                MenuItem::separator(),
            ],
        },
    ]
}

pub(crate) fn set_app_menus(cx: &mut App, menus: Vec<Menu>) {
    #[cfg(test)]
    let snapshot = snapshot_menus(&menus);

    cx.set_menus(menus);

    #[cfg(test)]
    {
        if cx.has_global::<MenuSnapshot>() {
            *cx.global_mut::<MenuSnapshot>() = snapshot;
        } else {
            cx.set_global(snapshot);
        }
    }
}

pub(crate) fn sync_app_menus(cx: &mut App) {
    let multi_exec_enabled = cx
        .try_global::<TermuaAppState>()
        .map(|state| state.multi_exec_enabled)
        .unwrap_or(false);
    set_app_menus(cx, build_menus(multi_exec_enabled));
}

#[cfg(test)]
#[derive(Clone, Debug, Default)]
pub(crate) struct MenuSnapshot {
    pub menus: Vec<MenuSnapshotMenu>,
}

#[cfg(test)]
impl gpui::Global for MenuSnapshot {}

#[cfg(test)]
#[derive(Clone, Debug)]
#[allow(dead_code)]
pub(crate) struct MenuSnapshotMenu {
    pub name: String,
    pub items: Vec<MenuSnapshotItem>,
}

#[cfg(test)]
#[derive(Clone, Debug)]
#[allow(dead_code)]
pub(crate) enum MenuSnapshotItem {
    Separator,
    Submenu(MenuSnapshotMenu),
    SystemMenu { name: String },
    Action { name: String, checked: bool },
}

#[cfg(test)]
fn snapshot_menus(menus: &[Menu]) -> MenuSnapshot {
    MenuSnapshot {
        menus: menus.iter().map(snapshot_menu).collect(),
    }
}

#[cfg(test)]
fn snapshot_menu(menu: &Menu) -> MenuSnapshotMenu {
    MenuSnapshotMenu {
        name: menu.name.to_string(),
        items: menu.items.iter().map(snapshot_item).collect(),
    }
}

#[cfg(test)]
fn snapshot_item(item: &MenuItem) -> MenuSnapshotItem {
    match item {
        MenuItem::Separator => MenuSnapshotItem::Separator,
        MenuItem::Submenu(menu) => MenuSnapshotItem::Submenu(snapshot_menu(menu)),
        MenuItem::SystemMenu(menu) => MenuSnapshotItem::SystemMenu {
            name: menu.name.to_string(),
        },
        MenuItem::Action { name, checked, .. } => MenuSnapshotItem::Action {
            name: name.to_string(),
            checked: *checked,
        },
    }
}
