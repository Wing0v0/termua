use std::collections::HashMap;

use gpui::px;
use gpui_term::{SshOptions, TerminalType};

use crate::store::{SerialFlowControl, SerialParity, SerialStopBits};

#[derive(Clone, Debug)]
pub(crate) struct SshParams {
    pub(crate) env: HashMap<String, String>,
    pub(crate) name: String,
    pub(crate) opts: SshOptions,
}

#[derive(Clone, Debug)]
pub(crate) struct SerialParams {
    pub(crate) name: String,
    pub(crate) port: String,
    pub(crate) baud: u32,
    pub(crate) data_bits: u8,
    pub(crate) parity: SerialParity,
    pub(crate) stop_bits: SerialStopBits,
    pub(crate) flow_control: SerialFlowControl,
}

impl SerialParams {
    pub(crate) fn to_options(&self) -> gpui_term::SerialOptions {
        gpui_term::SerialOptions {
            port: self.port.clone(),
            baud: self.baud,
            data_bits: self.data_bits,
            parity: match self.parity {
                SerialParity::None => gpui_term::SerialParity::None,
                SerialParity::Even => gpui_term::SerialParity::Even,
                SerialParity::Odd => gpui_term::SerialParity::Odd,
            },
            stop_bits: match self.stop_bits {
                SerialStopBits::One => gpui_term::SerialStopBits::One,
                SerialStopBits::Two => gpui_term::SerialStopBits::Two,
            },
            flow_control: match self.flow_control {
                SerialFlowControl::None => gpui_term::SerialFlowControl::None,
                SerialFlowControl::Software => gpui_term::SerialFlowControl::Software,
                SerialFlowControl::Hardware => gpui_term::SerialFlowControl::Hardware,
            },
        }
    }
}

pub(crate) struct TermuaAppState {
    pub(crate) main_window: Option<gpui::WindowHandle<gpui_component::Root>>,
    pub(crate) settings_window: Option<gpui::WindowHandle<gpui_component::Root>>,
    pub(crate) multi_exec_enabled: bool,
    pub(crate) sessions_sidebar_visible: bool,
    pub(crate) sessions_sidebar_width: gpui::Pixels,
    pub(crate) pending_commands: Vec<PendingCommand>,
}

impl Default for TermuaAppState {
    fn default() -> Self {
        Self {
            main_window: None,
            settings_window: None,
            multi_exec_enabled: false,
            sessions_sidebar_visible: true,
            sessions_sidebar_width: px(280.0),
            pending_commands: Vec::new(),
        }
    }
}

impl gpui::Global for TermuaAppState {}

impl TermuaAppState {
    pub(crate) fn pending_command(&mut self, command: PendingCommand) {
        if self
            .pending_commands
            .iter()
            .any(|existing| existing.coalesces_with(&command))
        {
            return;
        }

        self.pending_commands.push(command);
    }
}

#[derive(Clone, Debug)]
pub(crate) enum PendingCommand {
    OpenLocalTerminal {
        backend_type: TerminalType,
        env: HashMap<String, String>,
    },
    OpenSshTerminal {
        backend_type: TerminalType,
        params: SshParams,
    },
    OpenSerialTerminal {
        backend_type: TerminalType,
        params: SerialParams,
        session_id: Option<i64>,
    },
    ReloadSessionsSidebar,
}

impl PendingCommand {
    fn coalesces_with(&self, other: &Self) -> bool {
        matches!(
            (self, other),
            (Self::ReloadSessionsSidebar, Self::ReloadSessionsSidebar)
        )
    }
}
