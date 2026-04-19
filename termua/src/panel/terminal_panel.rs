use std::{collections::HashMap, path::PathBuf, time::Duration};

use gpui::{
    AnyElement, App, Context, ExternalPaths, FocusHandle, InteractiveElement, IntoElement,
    KeyDownEvent, MouseButton, ParentElement, Pixels, Render, SharedString, Styled, Window, div,
    prelude::FluentBuilder, px,
};
use gpui_common::TermuaIcon;
use gpui_component::{ActiveTheme, scroll::ScrollableElement};
use gpui_dock::{Panel, PanelEvent};
use gpui_term::{TerminalMode, TerminalShutdownPolicy, TerminalType, TerminalView};

use crate::notification::{self, MessageKind};

#[derive(Clone, Debug)]
struct PendingSftpUpload {
    paths: Vec<PathBuf>,
}

fn collect_dropped_upload_paths(paths: &[PathBuf]) -> Vec<PathBuf> {
    let mut upload_paths: Vec<PathBuf> = paths
        .iter()
        .filter(|path| path.is_file())
        .cloned()
        .collect();
    upload_paths.sort();
    upload_paths
}

fn supports_sftp_file_drop(kind: PanelKind, has_sftp: bool, paths: &[PathBuf]) -> bool {
    kind == PanelKind::Ssh
        && has_sftp
        && !paths.is_empty()
        && paths.iter().all(|path| path.is_file())
}

fn sftp_upload_file_count_label(count: usize) -> String {
    match count {
        1 => "1 file".to_string(),
        n => format!("{n} files"),
    }
}

fn sftp_upload_destination_label(current_dir: Option<&str>) -> String {
    match current_dir {
        Some(path) if !path.trim().is_empty() => format!("Destination: {path}"),
        _ => "Destination: current remote directory".to_string(),
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum PanelKind {
    Local,
    Ssh,
    Serial,
    // Recorder,
}

pub(crate) fn terminal_panel_tab_name(kind: PanelKind, id: usize) -> SharedString {
    match kind {
        PanelKind::Local => format!("local {id}").into(),
        PanelKind::Ssh => format!("ssh {id}").into(),
        PanelKind::Serial => format!("serial {id}").into(),
        // PanelKind::Recorder => format!("recorder {id}").into(),
    }
}

pub(crate) fn local_terminal_panel_tab_name(
    env: &HashMap<String, String>,
    id: usize,
    counts: &mut HashMap<String, usize>,
) -> SharedString {
    let Some(base) = gpui_term::shell::pick_shell_program_from_env(env)
        .map(gpui_term::shell::shell_display_name)
        .filter(|name| !name.trim().is_empty())
    else {
        return terminal_panel_tab_name(PanelKind::Local, id);
    };

    let count = counts.entry(base.clone()).or_insert(0);
    *count += 1;

    if *count == 1 {
        base.into()
    } else {
        format!("{base} {id}").into()
    }
}

pub(crate) fn tab_icon_path_for_terminal_type(terminal_type: TerminalType) -> TermuaIcon {
    match terminal_type {
        TerminalType::Alacritty => TermuaIcon::Alacritty,
        TerminalType::WezTerm => TermuaIcon::Wezterm,
    }
}

pub(crate) fn tab_icon_for_terminal_panel(
    kind: PanelKind,
    terminal_type: TerminalType,
) -> gpui_dock::TabIcon {
    match kind {
        PanelKind::Local | PanelKind::Ssh | PanelKind::Serial => gpui_dock::TabIcon::ColoredSvg {
            path: tab_icon_path_for_terminal_type(terminal_type).into(),
        },
    }
}

pub(crate) struct TerminalPanel {
    id: usize,
    kind: PanelKind,
    tab_label: SharedString,
    tab_tooltip: Option<SharedString>,
    terminal_view: gpui::Entity<TerminalView>,
    pending_sftp_upload: Option<PendingSftpUpload>,
}

impl TerminalPanel {
    pub(crate) fn new(
        id: usize,
        kind: PanelKind,
        tab_label: SharedString,
        tab_tooltip: Option<SharedString>,
        terminal_view: gpui::Entity<TerminalView>,
    ) -> Self {
        Self {
            id,
            kind,
            tab_label,
            tab_tooltip,
            terminal_view,
            pending_sftp_upload: None,
        }
    }

    pub(crate) fn id(&self) -> usize {
        self.id
    }

    pub(crate) fn kind(&self) -> PanelKind {
        self.kind
    }

    pub(crate) fn terminal_view(&self) -> gpui::Entity<TerminalView> {
        self.terminal_view.clone()
    }

    pub(crate) fn tab_label(&self) -> SharedString {
        self.tab_label.clone()
    }

    fn terminal_has_sftp(&self, cx: &App) -> bool {
        self.terminal_view
            .read(cx)
            .terminal
            .read(cx)
            .sftp()
            .is_some()
    }

    fn current_remote_dir(&self, cx: &App) -> Option<String> {
        self.terminal_view.read(cx).terminal.read(cx).current_dir()
    }

    fn notify(
        &self,
        kind: MessageKind,
        title: &str,
        detail: Option<&str>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let message = match detail {
            Some(detail) if !detail.trim().is_empty() => format!("{title}\n{detail}"),
            _ => title.to_string(),
        };
        notification::notify_deferred(kind, message, window, cx);
    }

    fn handle_sftp_file_drop(
        &mut self,
        paths: &ExternalPaths,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let upload_paths = collect_dropped_upload_paths(paths.paths());
        if upload_paths.is_empty() {
            self.notify(
                MessageKind::Info,
                "Only files are supported",
                Some("Dropped items did not include any files."),
                window,
                cx,
            );
            return;
        }

        if self.kind != PanelKind::Ssh || !self.terminal_has_sftp(cx) {
            return;
        }

        let terminal = self.terminal_view.read(cx).terminal.clone();
        let terminal = terminal.read(cx);

        if terminal
            .last_content()
            .mode
            .contains(TerminalMode::ALT_SCREEN)
        {
            self.notify(
                MessageKind::Warning,
                "Exit the full-screen app first",
                Some("Upload requires a shell prompt (ALT_SCREEN is active)."),
                window,
                cx,
            );
            return;
        }

        if terminal.sftp_upload_is_active() {
            self.notify(
                MessageKind::Warning,
                "Transfer in progress",
                Some("Wait for the current upload to finish before starting another."),
                window,
                cx,
            );
            return;
        }
        let _ = terminal;

        self.pending_sftp_upload = Some(PendingSftpUpload {
            paths: upload_paths,
        });
        cx.notify();
    }

    fn cancel_sftp_file_drop_upload(&mut self, cx: &mut Context<Self>) {
        if self.pending_sftp_upload.take().is_some() {
            cx.notify();
        }
    }

    fn confirm_sftp_file_drop_upload(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(dialog) = self.pending_sftp_upload.take() else {
            return;
        };

        let has_sftp = self.terminal_has_sftp(cx);
        if !has_sftp {
            self.notify(
                MessageKind::Error,
                "SFTP is unavailable",
                Some("This SSH terminal no longer has an active SFTP session."),
                window,
                cx,
            );
            cx.notify();
            return;
        }

        self.terminal_view.update(cx, |terminal_view, cx| {
            terminal_view.terminal.update(cx, |terminal, cx| {
                if terminal.sftp_upload_is_active() {
                    cx.emit(gpui_term::Event::Toast {
                        level: gpui::PromptLevel::Warning,
                        title: "Transfer in progress".to_string(),
                        detail: Some(
                            "Wait for the current upload to finish before starting another."
                                .to_string(),
                        ),
                    });
                    return;
                }
                terminal.start_sftp_upload(dialog.paths, cx);
            });
        });
        cx.notify();
    }

    fn handle_key_down(
        &mut self,
        event: &KeyDownEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.pending_sftp_upload.is_none() {
            return;
        }

        match event.keystroke.key.as_str() {
            "escape" => {
                self.cancel_sftp_file_drop_upload(cx);
                cx.stop_propagation();
            }
            "enter" => {
                self.confirm_sftp_file_drop_upload(window, cx);
                cx.stop_propagation();
            }
            _ => {}
        }
    }

    fn render_pending_sftp_upload_overlay(
        &mut self,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Option<AnyElement> {
        let dialog = self.pending_sftp_upload.clone()?;
        let destination = sftp_upload_destination_label(self.current_remote_dir(cx).as_deref());
        let file_count = sftp_upload_file_count_label(dialog.paths.len());
        let theme = cx.theme();
        let viewport = window.viewport_size();
        let panel_w = px(680.0)
            .min((viewport.width - px(24.0)).max(Pixels::ZERO))
            .max(px(360.0).min(viewport.width.max(Pixels::ZERO)));
        let row_bg = theme.muted.opacity(0.2);
        let backdrop = theme.overlay.opacity(0.35);
        let panel_bg = theme.popover.opacity(0.98);
        let panel_border = theme.border.opacity(0.9);
        let hint_fg = theme.muted_foreground;
        let accent = theme.accent;
        let accent_fg = theme.accent_foreground;

        let mut list = div()
            .mt(px(10.0))
            .h(px(280.0))
            .rounded_md()
            .border_1()
            .border_color(panel_border)
            .bg(theme.background.opacity(0.05))
            .p(px(8.0))
            .overflow_y_scrollbar();

        for path in &dialog.paths {
            let name = path
                .file_name()
                .and_then(|name| name.to_str())
                .unwrap_or_default()
                .to_string();
            list = list.child(
                div()
                    .bg(row_bg)
                    .rounded_md()
                    .p(px(8.0))
                    .mb(px(6.0))
                    .child(div().text_sm().child(name))
                    .child(
                        div()
                            .mt(px(2.0))
                            .text_xs()
                            .text_color(hint_fg)
                            .whitespace_normal()
                            .child(path.display().to_string()),
                    ),
            );
        }

        Some(
            div()
                .id("termua-terminal-panel-sftp-drop")
                .absolute()
                .top_0()
                .left_0()
                .right_0()
                .bottom_0()
                .bg(backdrop)
                .size_full()
                .flex()
                .items_center()
                .justify_center()
                .on_mouse_down(
                    MouseButton::Left,
                    cx.listener(|this, _, _, cx| {
                        this.cancel_sftp_file_drop_upload(cx);
                        cx.stop_propagation();
                    }),
                )
                .child(
                    div()
                        .w(panel_w)
                        .max_w(px(720.0))
                        .bg(panel_bg)
                        .text_color(theme.popover_foreground)
                        .border_1()
                        .border_color(panel_border)
                        .rounded_lg()
                        .shadow_lg()
                        .p(px(14.0))
                        .on_mouse_down(
                            MouseButton::Left,
                            cx.listener(|_, _, _, cx| cx.stop_propagation()),
                        )
                        .child(
                            div()
                                .flex()
                                .items_center()
                                .justify_between()
                                .child(div().text_sm().child("Upload via SFTP"))
                                .child(
                                    div()
                                        .cursor_pointer()
                                        .rounded_md()
                                        .w(px(34.0))
                                        .h(px(34.0))
                                        .flex()
                                        .items_center()
                                        .justify_center()
                                        .bg(theme.muted.opacity(0.25))
                                        .on_mouse_down(
                                            MouseButton::Left,
                                            cx.listener(|this, _, _, cx| {
                                                this.cancel_sftp_file_drop_upload(cx);
                                                cx.stop_propagation();
                                            }),
                                        )
                                        .child(gpui_component::Icon::new(
                                            gpui_component::IconName::Close,
                                        )),
                                ),
                        )
                        .child(
                            div()
                                .mt(px(8.0))
                                .text_xs()
                                .text_color(hint_fg)
                                .child(destination),
                        )
                        .child(list)
                        .child(
                            div()
                                .mt(px(10.0))
                                .flex()
                                .items_center()
                                .justify_between()
                                .child(
                                    div()
                                        .text_xs()
                                        .text_color(hint_fg)
                                        .child(format!("{file_count}  Press Enter to upload")),
                                )
                                .child(
                                    div()
                                        .cursor_pointer()
                                        .rounded_md()
                                        .bg(accent)
                                        .text_color(accent_fg)
                                        .w(px(38.0))
                                        .h(px(38.0))
                                        .flex()
                                        .items_center()
                                        .justify_center()
                                        .on_mouse_down(
                                            MouseButton::Left,
                                            cx.listener(|this, _, window, cx| {
                                                this.confirm_sftp_file_drop_upload(window, cx);
                                                cx.stop_propagation();
                                            }),
                                        )
                                        .child(gpui_component::Icon::new(
                                            gpui_component::IconName::ArrowUp,
                                        )),
                                ),
                        ),
                )
                .into_any_element(),
        )
    }
}

impl Drop for TerminalPanel {
    fn drop(&mut self) {
        log::debug!("termua: TerminalPanel drop (id={})", self.id);
    }
}

impl gpui::EventEmitter<PanelEvent> for TerminalPanel {}

impl gpui::Focusable for TerminalPanel {
    fn focus_handle(&self, cx: &App) -> FocusHandle {
        self.terminal_view.read(cx).focus_handle.clone()
    }
}

impl Panel for TerminalPanel {
    fn panel_name(&self) -> &'static str {
        "TerminalPanel"
    }

    fn tab_icon(&self, cx: &App) -> Option<gpui_dock::TabIcon> {
        let backend_type = self.terminal_view.read(cx).terminal.read(cx).backend_type();
        Some(tab_icon_for_terminal_panel(self.kind, backend_type))
    }

    fn on_removed(&mut self, _window: &mut Window, _cx: &mut Context<Self>) {
        // This may run during tab drag/drop (detach/attach), so it must not terminate the session.
        log::debug!("termua: TerminalPanel on_removed (id={})", self.id);
    }

    fn on_close(&mut self, _window: &mut Window, cx: &mut Context<Self>) {
        log::debug!(
            "termua: TerminalPanel on_close (id={}), requesting terminal shutdown",
            self.id
        );

        // crate::assistant::unregister_terminal_target(cx, self.id);
        // crate::sharing::disconnect_terminal_sharing(self.terminal_view.entity_id(), cx);

        // Ensure the backend releases its PTY/process resources when the tab is explicitly closed.
        self.terminal_view.update(cx, |terminal_view, cx| {
            terminal_view.terminal.update(cx, |terminal, cx| {
                terminal.shutdown(
                    TerminalShutdownPolicy::GracefulThenKill(Duration::from_secs(3)),
                    cx,
                );
            });
        });
    }

    fn tab_name(&self, _cx: &App) -> Option<SharedString> {
        Some(self.tab_label.clone())
    }

    fn tab_tooltip(
        &mut self,
        _window: &mut Window,
        _cx: &mut Context<Self>,
    ) -> Option<impl IntoElement> {
        let tooltip = self.tab_tooltip.clone()?;
        Some(div().child(tooltip))
    }

    fn title(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        self.tab_name(cx).unwrap_or_else(|| "local".into())
    }
}

impl Render for TerminalPanel {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let kind = self.kind;
        let terminal_view = self.terminal_view.clone();
        div()
            .id("termua-terminal-panel")
            .size_full()
            .relative()
            .can_drop(move |any, _window, cx| {
                let has_sftp = terminal_view.read(cx).terminal.read(cx).sftp().is_some();
                any.downcast_ref::<ExternalPaths>()
                    .is_some_and(|paths| supports_sftp_file_drop(kind, has_sftp, paths.paths()))
            })
            .on_drop(cx.listener(|this, paths: &ExternalPaths, window, cx| {
                this.handle_sftp_file_drop(paths, window, cx);
            }))
            .on_key_down(cx.listener(Self::handle_key_down))
            .child(self.terminal_view.clone())
            .when_some(
                self.render_pending_sftp_upload_overlay(window, cx),
                |this, overlay| this.child(overlay),
            )
    }
}