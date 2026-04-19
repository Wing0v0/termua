#![cfg_attr(all(windows, not(debug_assertions)), windows_subsystem = "windows")]

rust_i18n::i18n!("../locales");

mod app_state;
// mod assistant;
mod atomic_write;
mod bootstrap;
// mod cast_player;
mod command_history;
mod env;
mod footbar;
mod globals;
mod keychain;
mod locale;
mod lock_screen;
mod logging;
mod menu;
mod notification;
mod panel;
mod right_sidebar;
mod serial;
mod session;
mod settings;
// mod sharing;
mod shell_integration;
mod ssh;
mod static_suggestions;
mod theme_manager;
mod window;

pub(crate) use app_state::{PendingCommand, SerialParams, SshParams, TermuaAppState};
pub(crate) use menu::{
    NewLocalTerminal, OpenNewSession, OpenSftp, ToggleMessagesSidebar,
    ToggleSessionsSidebar,
};
pub use session::store;
pub use window::{new_session, settings as config};

use crate::settings::SettingsFile;

fn main() {
    let settings = match settings::load_settings_from_disk() {
        Ok(s) => s,
        Err(err) => {
            eprintln!("failed to load settings.json, using defaults: {err:#}");
            SettingsFile::default()
        }
    };
    logging::init_logging(&settings);

    bootstrap::run(settings);
}
