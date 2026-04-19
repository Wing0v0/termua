use std::collections::HashMap;

use gpui::{
    AnyElement, App, AppContext, Bounds, Context, Entity, EventEmitter, FocusHandle, Focusable,
    InteractiveElement, IntoElement, ParentElement, ScrollHandle, Styled, StyledImage,
    Subscription, Window, WindowBounds, WindowDecorations, WindowOptions, div, img, px, size,
};
use gpui_common::TermuaIcon;
use gpui_component::{
    IndexPath,
    input::{InputEvent, InputState},
    select::{SearchableVec, SelectEvent, SelectItem, SelectState},
    tree::{TreeItem, TreeState},
};
use gpui_term::SshBackend;
use rust_i18n::t;

use super::SettingsNavSection;
use crate::settings::{Language, SettingsFile, TerminalBackend, load_settings_from_disk};

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
#[repr(u8)]
pub(super) enum TerminalKeybinding {
    Copy = 0,
    Paste = 1,
    SelectAll = 2,
    Clear = 3,
    Search = 4,
    SearchNext = 5,
    SearchPrevious = 6,
    IncreaseFontSize = 7,
    DecreaseFontSize = 8,
    ResetFontSize = 9,
}

impl TerminalKeybinding {
    const ALL: [Self; 10] = [
        Self::Copy,
        Self::Paste,
        Self::SelectAll,
        Self::Clear,
        Self::Search,
        Self::SearchNext,
        Self::SearchPrevious,
        Self::IncreaseFontSize,
        Self::DecreaseFontSize,
        Self::ResetFontSize,
    ];

    pub(super) fn all() -> &'static [Self; 10] {
        &Self::ALL
    }

    pub(super) fn from_id(id: &str) -> Option<Self> {
        Some(match id {
            "terminal.keybindings.copy" => Self::Copy,
            "terminal.keybindings.paste" => Self::Paste,
            "terminal.keybindings.select_all" => Self::SelectAll,
            "terminal.keybindings.clear" => Self::Clear,
            "terminal.keybindings.search" => Self::Search,
            "terminal.keybindings.search_next" => Self::SearchNext,
            "terminal.keybindings.search_previous" => Self::SearchPrevious,
            "terminal.keybindings.increase_font_size" => Self::IncreaseFontSize,
            "terminal.keybindings.decrease_font_size" => Self::DecreaseFontSize,
            "terminal.keybindings.reset_font_size" => Self::ResetFontSize,
            _ => return None,
        })
    }

    pub(super) fn id(&self) -> &'static str {
        match self {
            Self::Copy => "terminal.keybindings.copy",
            Self::Paste => "terminal.keybindings.paste",
            Self::SelectAll => "terminal.keybindings.select_all",
            Self::Clear => "terminal.keybindings.clear",
            Self::Search => "terminal.keybindings.search",
            Self::SearchNext => "terminal.keybindings.search_next",
            Self::SearchPrevious => "terminal.keybindings.search_previous",
            Self::IncreaseFontSize => "terminal.keybindings.increase_font_size",
            Self::DecreaseFontSize => "terminal.keybindings.decrease_font_size",
            Self::ResetFontSize => "terminal.keybindings.reset_font_size",
        }
    }

    pub(super) fn default_label(&self) -> &'static str {
        #[cfg(target_os = "macos")]
        match self {
            Self::Copy => "cmd-c",
            Self::Paste => "cmd-v",
            Self::SelectAll => "cmd-a",
            Self::Clear => "cmd-k",
            Self::Search => "cmd-f",
            Self::SearchNext => "cmd-g",
            Self::SearchPrevious => "cmd-shift-g",
            Self::IncreaseFontSize => "cmd-+ (also cmd-=)",
            Self::DecreaseFontSize => "cmd--",
            Self::ResetFontSize => "cmd-0",
        }

        #[cfg(not(target_os = "macos"))]
        match self {
            Self::Copy => "ctrl-shift-c",
            Self::Paste => "ctrl-shift-v",
            Self::SelectAll => "ctrl-shift-a",
            Self::Clear => "ctrl-shift-k",
            Self::Search => "ctrl-shift-f",
            Self::SearchNext => "ctrl-g",
            Self::SearchPrevious => "ctrl-shift-g",
            Self::IncreaseFontSize => "ctrl-+ (also ctrl-=)",
            Self::DecreaseFontSize => "ctrl--",
            Self::ResetFontSize => "ctrl-0",
        }
    }

    pub(super) fn index(&self) -> usize {
        *self as usize
    }
}

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum SettingsPage {
    AppearanceTheme,
    AppearanceLanguage,
    Terminal,
    TerminalFont,
    TerminalKeyBindings,
    TerminalCursor,
    TerminalRendering,
    TerminalBehavior,
    TerminalSftp,
    TerminalSuggestions,
    TerminalSharing,
    RecordingCast,
    Logging,
    Assistant,
    SecurityLockScreen,
}

const KEYBINDINGS_PAGE_HINT_KEY: &str = "Settings.Hint.KeyBindings";
const LOGGING_PAGE_HINT_KEY: &str = "Settings.Hint.Logging";
const ASSISTANT_PAGE_HINT_KEY: &str = "Settings.Hint.Assistant";
const LOCK_SCREEN_PAGE_HINT_KEY: &str = "Settings.Hint.LockScreen";

#[derive(Clone, Copy, Debug)]
pub(super) struct SettingsPageSpec {
    pub(super) section: SettingsNavSection,
    pub(super) item_label_key: &'static str,
    pub(super) page: SettingsPage,
    pub(super) nav_item_id: &'static str,
    pub(super) heading_key: &'static str,
    pub(super) hint_key: Option<&'static str>,
    pub(super) is_sidebar_item: bool,
}

const SETTINGS_PAGE_SPECS: &[SettingsPageSpec] = &[
    SettingsPageSpec {
        section: SettingsNavSection::Appearance,
        item_label_key: "Settings.Appearance.Theme",
        page: SettingsPage::AppearanceTheme,
        nav_item_id: "nav.page.appearance.theme",
        heading_key: "Settings.Appearance.Theme",
        hint_key: None,
        is_sidebar_item: true,
    },
    SettingsPageSpec {
        section: SettingsNavSection::Appearance,
        item_label_key: "Settings.Appearance.Language",
        page: SettingsPage::AppearanceLanguage,
        nav_item_id: "nav.page.appearance.language",
        heading_key: "Settings.Appearance.Language",
        hint_key: None,
        is_sidebar_item: true,
    },
    // `SettingsPage::Terminal` maps to the group row (`nav.group.terminal`) and should not show as
    // a child item in the sidebar.
    SettingsPageSpec {
        section: SettingsNavSection::Terminal,
        item_label_key: "Settings.Terminal.Terminal",
        page: SettingsPage::Terminal,
        nav_item_id: "nav.group.terminal",
        heading_key: "Settings.Terminal.Terminal",
        hint_key: None,
        is_sidebar_item: false,
    },
    SettingsPageSpec {
        section: SettingsNavSection::Terminal,
        item_label_key: "Settings.Terminal.Behavior",
        page: SettingsPage::TerminalBehavior,
        nav_item_id: "nav.page.terminal.behavior",
        heading_key: "Settings.Terminal.Behavior",
        hint_key: None,
        is_sidebar_item: true,
    },
    SettingsPageSpec {
        section: SettingsNavSection::Terminal,
        item_label_key: "Settings.Terminal.Sftp",
        page: SettingsPage::TerminalSftp,
        nav_item_id: "nav.page.terminal.sftp",
        heading_key: "Settings.Terminal.Sftp",
        hint_key: None,
        is_sidebar_item: true,
    },
    SettingsPageSpec {
        section: SettingsNavSection::Terminal,
        item_label_key: "Settings.Terminal.Suggestions",
        page: SettingsPage::TerminalSuggestions,
        nav_item_id: "nav.page.terminal.suggestions",
        heading_key: "Settings.Terminal.Suggestions",
        hint_key: None,
        is_sidebar_item: true,
    },
    SettingsPageSpec {
        section: SettingsNavSection::Terminal,
        item_label_key: "Settings.Terminal.Cursor",
        page: SettingsPage::TerminalCursor,
        nav_item_id: "nav.page.terminal.cursor",
        heading_key: "Settings.Terminal.Cursor",
        hint_key: None,
        is_sidebar_item: true,
    },
    SettingsPageSpec {
        section: SettingsNavSection::Terminal,
        item_label_key: "Settings.Terminal.Font",
        page: SettingsPage::TerminalFont,
        nav_item_id: "nav.page.terminal.font",
        heading_key: "Settings.Terminal.Font",
        hint_key: None,
        is_sidebar_item: true,
    },
    SettingsPageSpec {
        section: SettingsNavSection::Terminal,
        item_label_key: "Settings.Terminal.KeyBindings",
        page: SettingsPage::TerminalKeyBindings,
        nav_item_id: "nav.page.terminal.key_bindings",
        heading_key: "Settings.Terminal.KeyBindings",
        hint_key: Some(KEYBINDINGS_PAGE_HINT_KEY),
        is_sidebar_item: true,
    },
    SettingsPageSpec {
        section: SettingsNavSection::Terminal,
        item_label_key: "Settings.Terminal.Rendering",
        page: SettingsPage::TerminalRendering,
        nav_item_id: "nav.page.terminal.rendering",
        heading_key: "Settings.Terminal.Rendering",
        hint_key: None,
        is_sidebar_item: true,
    },
    SettingsPageSpec {
        section: SettingsNavSection::Logging,
        item_label_key: "Settings.Logging.General",
        page: SettingsPage::Logging,
        nav_item_id: "nav.page.logging.general",
        heading_key: "Settings.Logging.Logging",
        hint_key: Some(LOGGING_PAGE_HINT_KEY),
        is_sidebar_item: true,
    },
    SettingsPageSpec {
        section: SettingsNavSection::Security,
        item_label_key: "Settings.Security.LockScreen",
        page: SettingsPage::SecurityLockScreen,
        nav_item_id: "nav.page.security.lock_screen",
        heading_key: "Settings.Security.LockScreen",
        hint_key: Some(LOCK_SCREEN_PAGE_HINT_KEY),
        is_sidebar_item: true,
    },
];

const SETTINGS_PAGE_NAV_ID_ALIASES: &[(&str, SettingsPage)] =
    &[("nav.page.terminal.terminal", SettingsPage::Terminal)];

#[derive(Clone, Debug, Eq, PartialEq)]
pub(super) struct SidebarNavItem {
    pub(super) label: String,
    pub(super) page: SettingsPage,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(super) struct SidebarNavGroup {
    pub(super) label: String,
    pub(super) section: SettingsNavSection,
    pub(super) items: Vec<SidebarNavItem>,
}

fn nav_section_label_key(section: SettingsNavSection) -> &'static str {
    match section {
        SettingsNavSection::Appearance => "Settings.Section.Appearance",
        SettingsNavSection::Terminal => "Settings.Section.Terminal",
        SettingsNavSection::Recording => "Settings.Section.Recording",
        SettingsNavSection::Logging => "Settings.Section.Logging",
        SettingsNavSection::Assistant => "Settings.Section.Assistant",
        SettingsNavSection::Security => "Settings.Section.Security",
    }
}

fn nav_section_id(section: SettingsNavSection) -> &'static str {
    match section {
        SettingsNavSection::Appearance => "appearance",
        SettingsNavSection::Terminal => "terminal",
        SettingsNavSection::Recording => "recording",
        SettingsNavSection::Logging => "logging",
        SettingsNavSection::Assistant => "assistant",
        SettingsNavSection::Security => "security",
    }
}

fn nav_section_sort_key(section: SettingsNavSection) -> &'static str {
    match section {
        SettingsNavSection::Appearance => "Appearance",
        SettingsNavSection::Terminal => "Terminal",
        SettingsNavSection::Recording => "Recording",
        SettingsNavSection::Logging => "Logging",
        SettingsNavSection::Assistant => "Assistant",
        SettingsNavSection::Security => "Security",
    }
}

fn nav_item_sort_key(page: SettingsPage) -> &'static str {
    match page {
        SettingsPage::AppearanceTheme => "Theme",
        SettingsPage::AppearanceLanguage => "Language",
        SettingsPage::Terminal => "Terminal",
        SettingsPage::TerminalFont => "Font",
        SettingsPage::TerminalKeyBindings => "Key Bindings",
        SettingsPage::TerminalCursor => "Cursor",
        SettingsPage::TerminalRendering => "Rendering",
        SettingsPage::TerminalBehavior => "Behavior",
        SettingsPage::TerminalSftp => "SFTP",
        SettingsPage::TerminalSuggestions => "Suggestions",
        SettingsPage::TerminalSharing => "Sharing",
        SettingsPage::RecordingCast => "Cast Recording",
        SettingsPage::Logging => "General",
        SettingsPage::Assistant => "ZeroClaw",
        SettingsPage::SecurityLockScreen => "Lock screen",
    }
}

pub(super) fn sidebar_nav_specs() -> Vec<SidebarNavGroup> {
    let mut groups: Vec<SidebarNavGroup> = Vec::new();
    for spec in SETTINGS_PAGE_SPECS
        .iter()
        .filter(|spec| spec.is_sidebar_item)
    {
        if groups.last().is_some_and(|g| g.section == spec.section) {
            groups.last_mut().unwrap().items.push(SidebarNavItem {
                label: t!(spec.item_label_key).to_string(),
                page: spec.page,
            });
            continue;
        }

        groups.push(SidebarNavGroup {
            section: spec.section,
            label: t!(nav_section_label_key(spec.section)).to_string(),
            items: vec![SidebarNavItem {
                label: t!(spec.item_label_key).to_string(),
                page: spec.page,
            }],
        });
    }

    for group in &mut groups {
        group.items.sort_unstable_by(|a, b| {
            nav_item_sort_key(a.page)
                .cmp(nav_item_sort_key(b.page))
                .then_with(|| a.label.cmp(&b.label))
                .then_with(|| {
                    nav_tree_item_id_for_page(a.page).cmp(nav_tree_item_id_for_page(b.page))
                })
        });
    }

    groups.sort_unstable_by(|a, b| {
        nav_section_sort_key(a.section)
            .cmp(nav_section_sort_key(b.section))
            .then_with(|| a.label.cmp(&b.label))
            .then_with(|| nav_section_id(a.section).cmp(nav_section_id(b.section)))
    });

    groups
}

pub(super) fn nav_tree_item_id_for_page(page: SettingsPage) -> &'static str {
    SETTINGS_PAGE_SPECS
        .iter()
        .find(|spec| spec.page == page)
        .map(|spec| spec.nav_item_id)
        .unwrap_or_else(|| {
            debug_assert!(false, "missing SettingsPageSpec for {page:?}");
            "nav.page.appearance.theme"
        })
}

pub(super) fn page_spec(page: SettingsPage) -> SettingsPageSpec {
    SETTINGS_PAGE_SPECS
        .iter()
        .find(|spec| spec.page == page)
        .copied()
        .unwrap_or_else(|| {
            debug_assert!(false, "missing SettingsPageSpec for {page:?}");
            SETTINGS_PAGE_SPECS[0]
        })
}

pub(super) fn page_for_nav_tree_item_id(id: &str) -> Option<SettingsPage> {
    if let Some(page) = SETTINGS_PAGE_SPECS
        .iter()
        .find(|spec| spec.nav_item_id == id)
        .map(|spec| spec.page)
    {
        return Some(page);
    }

    SETTINGS_PAGE_NAV_ID_ALIASES
        .iter()
        .find(|(alias, _)| *alias == id)
        .map(|(_, page)| *page)
}

pub(super) fn build_nav_tree_items() -> Vec<TreeItem> {
    sidebar_nav_specs()
        .into_iter()
        .map(|group| {
            let group_id = format!("nav.group.{}", nav_section_id(group.section));
            TreeItem::new(group_id, group.label)
                .expanded(true)
                .children(
                    group.items.into_iter().map(|item| {
                        TreeItem::new(nav_tree_item_id_for_page(item.page), item.label)
                    }),
                )
        })
        .collect()
}

pub(super) fn find_tree_item_by_id<'a>(items: &'a [TreeItem], id: &str) -> Option<&'a TreeItem> {
    for item in items {
        if item.id.as_ref() == id {
            return Some(item);
        }
        if let Some(found) = find_tree_item_by_id(&item.children, id) {
            return Some(found);
        }
    }
    None
}

fn assistant_headers_to_text(headers: &HashMap<String, String>) -> String {
    if headers.is_empty() {
        return String::new();
    }

    let mut pairs: Vec<(&String, &String)> = headers.iter().collect();
    pairs.sort_by_key(|(ak, _)| *ak);

    let mut out = String::new();
    for (i, (k, v)) in pairs.into_iter().enumerate() {
        if i > 0 {
            out.push('\n');
        }
        out.push_str(k.trim());
        out.push_str(": ");
        out.push_str(v.trim());
    }
    out
}

fn parse_assistant_headers(text: &str) -> anyhow::Result<HashMap<String, String>> {
    let mut out: HashMap<String, String> = HashMap::new();
    for (line_ix, raw_line) in text.lines().enumerate() {
        let line = raw_line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }

        let Some((k, v)) = line.split_once(':') else {
            anyhow::bail!(
                "invalid header on line {} (expected \"Name: Value\")",
                line_ix + 1
            );
        };

        let k = k.trim();
        let v = v.trim();
        if k.is_empty() {
            anyhow::bail!("invalid header on line {} (empty name)", line_ix + 1);
        }

        out.insert(k.to_string(), v.to_string());
    }
    Ok(out)
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(super) struct AssistantProviderSelectItem {
    pub(super) name: gpui::SharedString,
    pub(super) display_name: gpui::SharedString,
}

impl AssistantProviderSelectItem {
    fn new(name: gpui::SharedString, display_name: gpui::SharedString) -> Self {
        Self { name, display_name }
    }
}

impl SelectItem for AssistantProviderSelectItem {
    type Value = gpui::SharedString;

    fn title(&self) -> gpui::SharedString {
        self.display_name.clone()
    }

    fn render(&self, _window: &mut Window, _cx: &mut App) -> impl IntoElement {
        let selector = format!(
            "termua-settings-assistant-provider-option-{}",
            debug_id_fragment(self.name.as_ref())
        );
        gpui_component::h_flex()
            .debug_selector(move || selector)
            .items_center()
            .gap_2()
            .child(div().child(self.display_name.clone()))
            .child(div().text_xs().child(self.name.clone()))
    }

    fn value(&self) -> &Self::Value {
        &self.name
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(super) struct AssistantModelSelectItem {
    pub(super) value: gpui::SharedString,
}

impl AssistantModelSelectItem {
    pub(super) fn default_item() -> Self {
        Self {
            value: gpui::SharedString::from(""),
        }
    }

    pub(super) fn for_model(model: gpui::SharedString) -> Self {
        Self { value: model }
    }
}

impl SelectItem for AssistantModelSelectItem {
    type Value = gpui::SharedString;

    fn title(&self) -> gpui::SharedString {
        if self.value.as_ref().trim().is_empty() {
            t!("Settings.Select.Default").to_string().into()
        } else {
            self.value.clone()
        }
    }

    fn render(&self, _window: &mut Window, _cx: &mut App) -> impl IntoElement {
        let selector = format!(
            "termua-settings-assistant-model-option-{}",
            debug_id_fragment(self.value.as_ref())
        );
        div().debug_selector(move || selector).child(self.title())
    }

    fn value(&self) -> &Self::Value {
        &self.value
    }
}

struct AssistantControlsInit {
    assistant_temperature_input: Entity<InputState>,
    assistant_api_url_input: Entity<InputState>,
    assistant_api_path_input: Entity<InputState>,
    assistant_provider_timeout_input: Entity<InputState>,
    assistant_extra_headers_input: Entity<InputState>,
    assistant_api_key_input: Entity<InputState>,
    assistant_provider_select: Entity<SelectState<SearchableVec<AssistantProviderSelectItem>>>,
    assistant_model_select: Entity<SelectState<SearchableVec<AssistantModelSelectItem>>>,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub(super) struct TerminalBackendSelectItem {
    backend: TerminalBackend,
    debug_icon_prefix: &'static str,
}

impl TerminalBackendSelectItem {
    fn new(backend: TerminalBackend, debug_icon_prefix: &'static str) -> Self {
        Self {
            backend,
            debug_icon_prefix,
        }
    }
}

impl SelectItem for TerminalBackendSelectItem {
    type Value = TerminalBackend;

    fn title(&self) -> gpui::SharedString {
        gpui::SharedString::from(terminal_backend_label(self.backend))
    }

    fn display_title(&self) -> Option<gpui::AnyElement> {
        Some(
            terminal_backend_label_with_icon(self.backend, self.debug_icon_prefix)
                .into_any_element(),
        )
    }

    fn render(&self, _window: &mut Window, _cx: &mut App) -> impl IntoElement {
        terminal_backend_label_with_icon(self.backend, self.debug_icon_prefix)
    }

    fn value(&self) -> &Self::Value {
        &self.backend
    }
}

fn terminal_backend_label(backend: TerminalBackend) -> &'static str {
    match backend {
        TerminalBackend::Alacritty => "Alacritty",
        TerminalBackend::Wezterm => "Wezterm",
    }
}

fn terminal_backend_icon_path(backend: TerminalBackend) -> TermuaIcon {
    match backend {
        TerminalBackend::Alacritty => TermuaIcon::Alacritty,
        TerminalBackend::Wezterm => TermuaIcon::Wezterm,
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) struct SshBackendSelectItem {
    backend: SshBackend,
}

impl SshBackendSelectItem {
    fn new(backend: SshBackend) -> Self {
        Self { backend }
    }
}

impl SelectItem for SshBackendSelectItem {
    type Value = SshBackend;

    fn title(&self) -> gpui::SharedString {
        gpui::SharedString::from(ssh_backend_label(self.backend))
    }

    fn render(&self, _window: &mut Window, _cx: &mut App) -> impl IntoElement {
        gpui::SharedString::from(ssh_backend_label(self.backend))
    }

    fn value(&self) -> &Self::Value {
        &self.backend
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) struct PlaybackSpeedSelectItem {
    speed: f64,
}

impl PlaybackSpeedSelectItem {
    fn new(speed: f64) -> Self {
        Self { speed }
    }
}

impl SelectItem for PlaybackSpeedSelectItem {
    type Value = f64;

    fn title(&self) -> gpui::SharedString {
        let label = if self.speed.fract() == 0.0 {
            format!("{:.0}x", self.speed)
        } else {
            format!("{:.1}x", self.speed)
        };
        label.into()
    }

    fn render(&self, _window: &mut Window, _cx: &mut App) -> impl IntoElement {
        self.title()
    }

    fn value(&self) -> &Self::Value {
        &self.speed
    }
}

fn ssh_backend_label(backend: SshBackend) -> &'static str {
    match backend {
        SshBackend::Ssh2 => "ssh2",
        SshBackend::Libssh => "libssh",
    }
}

pub(super) fn ssh_backend_docs_url(backend: SshBackend) -> &'static str {
    match backend {
        SshBackend::Ssh2 => "https://docs.rs/ssh2/latest/ssh2/",
        SshBackend::Libssh => "https://docs.rs/libssh-rs/latest/libssh_rs/",
    }
}

fn terminal_backend_id_suffix(backend: TerminalBackend) -> &'static str {
    match backend {
        TerminalBackend::Alacritty => "alacritty",
        TerminalBackend::Wezterm => "wezterm",
    }
}

fn terminal_backend_label_with_icon(
    backend: TerminalBackend,
    debug_icon_prefix: &'static str,
) -> impl IntoElement {
    let selector = format!(
        "{debug_icon_prefix}-{}",
        terminal_backend_id_suffix(backend)
    );
    let icon = div().debug_selector(move || selector).child(
        img(terminal_backend_icon_path(backend))
            .w(px(16.))
            .h(px(16.))
            .flex_shrink_0()
            .object_fit(gpui::ObjectFit::Contain),
    );

    gpui_component::h_flex()
        .items_center()
        .gap_2()
        .child(icon)
        .child(div().child(terminal_backend_label(backend)))
}

pub struct SettingsWindow {
    pub(super) focus_handle: FocusHandle,

    // Left pane state
    pub(super) search_input: Entity<InputState>,
    pub(super) lock_overlay: crate::lock_screen::overlay::LockOverlayState,
    pub(super) selected_page: SettingsPage,
    pub(super) nav_tree_state: Entity<TreeState>,
    pub(super) nav_tree_items: Vec<TreeItem>,

    // Settings data
    pub(super) settings: SettingsFile,
    pub(super) current_language: Language,
    pub(super) save_epoch: usize,

    // Scroll state
    pub(super) right_scroll_handle: ScrollHandle,

    // Page inputs
    pub(super) font_family_select: Entity<SelectState<SearchableVec<FontFamilySelectItem>>>,
    pub(super) terminal_default_backend_select:
        Entity<SelectState<SearchableVec<TerminalBackendSelectItem>>>,
    pub(super) terminal_ssh_backend_select:
        Entity<SelectState<SearchableVec<SshBackendSelectItem>>>,
    pub(super) terminal_keybinding_focus: [FocusHandle; 10],
    pub(super) logging_path_input: Entity<InputState>,
    pub(super) static_suggestions_reload_in_flight: bool,

    pub(super) _subscriptions: Vec<Subscription>,
}

impl EventEmitter<()> for SettingsWindow {}

impl Focusable for SettingsWindow {
    fn focus_handle(&self, _cx: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

impl SettingsWindow {
    fn set_input_value(
        input: &gpui::Entity<InputState>,
        value: &str,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let value = value.to_string();
        input.update(cx, move |state, cx| state.set_value(&value, window, cx));
    }

    fn trimmed_nonempty(value: Option<&str>) -> Option<String> {
        value
            .map(str::trim)
            .filter(|v| !v.is_empty())
            .map(ToString::to_string)
    }

    fn new_input(
        window: &mut Window,
        cx: &mut Context<Self>,
        placeholder: String,
    ) -> gpui::Entity<InputState> {
        cx.new(|cx| InputState::new(window, cx).placeholder(placeholder))
    }

    fn new_configured_input<F>(
        window: &mut Window,
        cx: &mut Context<Self>,
        placeholder: String,
        configure: F,
    ) -> gpui::Entity<InputState>
    where
        F: FnOnce(InputState) -> InputState,
    {
        cx.new(|cx| configure(InputState::new(window, cx).placeholder(placeholder)))
    }

    fn new_input_with_initial<T: ToString>(
        window: &mut Window,
        cx: &mut Context<Self>,
        placeholder: String,
        initial: Option<T>,
    ) -> gpui::Entity<InputState> {
        let input = Self::new_input(window, cx, placeholder);
        if let Some(value) = initial {
            let value = value.to_string();
            Self::set_input_value(&input, &value, window, cx);
        }
        input
    }

    fn row_index(row: usize) -> IndexPath {
        IndexPath::default().row(row)
    }

    fn new_select<I>(
        window: &mut Window,
        cx: &mut Context<Self>,
        items: Vec<I>,
        selected_row: Option<usize>,
    ) -> Entity<SelectState<SearchableVec<I>>>
    where
        I: SelectItem + 'static,
        I::Value: 'static,
    {
        cx.new(|cx| {
            SelectState::new(
                SearchableVec::new(items),
                selected_row.map(Self::row_index),
                window,
                cx,
            )
        })
    }

    fn subscribe_trimmed_input<F, G>(
        &mut self,
        input: &Entity<InputState>,
        window: &mut Window,
        cx: &mut Context<Self>,
        should_handle: G,
        apply: F,
    ) where
        F: Fn(&mut Self, String, &mut Window, &mut Context<Self>) + 'static,
        G: Fn(&InputEvent) -> bool + 'static,
    {
        self._subscriptions.push(cx.subscribe_in(
            input,
            window,
            move |this, input, ev, window, cx| {
                if !should_handle(ev) {
                    return;
                }

                let value = input.read(cx).value().trim().to_string();
                apply(this, value, window, cx);
            },
        ));
    }

    fn subscribe_change_input<F>(
        &mut self,
        input: &Entity<InputState>,
        window: &mut Window,
        cx: &mut Context<Self>,
        apply: F,
    ) where
        F: Fn(&mut Self, Option<String>, &mut Window, &mut Context<Self>) + 'static,
    {
        self.subscribe_trimmed_input(input, window, cx, |ev| matches!(ev, InputEvent::Change), {
            move |this, value, window, cx| {
                apply(this, (!value.is_empty()).then_some(value), window, cx);
            }
        });
    }

    fn subscribe_select_confirm<I, F>(
        &mut self,
        select: &Entity<SelectState<SearchableVec<I>>>,
        window: &mut Window,
        cx: &mut Context<Self>,
        apply: F,
    ) where
        I: SelectItem + 'static,
        I::Value: 'static,
        F: Fn(&mut Self, &I::Value, &mut Window, &mut Context<Self>) + 'static,
    {
        self._subscriptions.push(cx.subscribe_in(
            select,
            window,
            move |this, _select, ev: &SelectEvent<SearchableVec<I>>, window, cx| {
                if let SelectEvent::Confirm(Some(value)) = ev {
                    apply(this, value, window, cx);
                }
            },
        ));
    }

    pub fn open(app: &mut App) -> anyhow::Result<gpui::WindowHandle<gpui_component::Root>> {
        use gpui_component::{Root, TitleBar};

        let initial_size = size(px(900.), px(700.));
        let min_size = size(px(900.), px(400.));
        let initial_bounds = Bounds::centered(None, initial_size, app);

        let handle = app.open_window(
            WindowOptions {
                window_bounds: Some(WindowBounds::Windowed(initial_bounds)),
                titlebar: Some(TitleBar::title_bar_options()),
                window_decorations: cfg!(target_os = "linux").then_some(WindowDecorations::Client),
                window_min_size: Some(min_size),
                ..Default::default()
            },
            |window, cx| {
                let view = cx.new(|cx| Self::new(window, cx));
                cx.new(|cx| Root::new(view, window, cx))
            },
        )?;
        Ok(handle)
    }

    fn selected_page_from_settings(settings: &SettingsFile) -> SettingsPage {
        settings
            .ui
            .last_settings_page
            .as_deref()
            .and_then(page_for_nav_tree_item_id)
            .unwrap_or(SettingsPage::Terminal)
    }

    fn search_input(window: &mut Window, cx: &mut Context<Self>) -> gpui::Entity<InputState> {
        Self::new_input(window, cx, t!("Settings.Search.Placeholder").to_string())
    }

    fn logging_path_input(
        settings: &SettingsFile,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> gpui::Entity<InputState> {
        Self::new_input_with_initial(
            window,
            cx,
            t!("Settings.Logging.PathPlaceholder").to_string(),
            Self::trimmed_nonempty(settings.logging.path.as_deref()),
        )
    }

    fn font_family_select(
        settings: &SettingsFile,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> gpui::Entity<SelectState<SearchableVec<FontFamilySelectItem>>> {
        let current_font_family = settings.terminal.font_family.to_string();
        let mut font_names = cx.text_system().all_font_names();
        if !font_names.iter().any(|name| name == &current_font_family) {
            font_names.insert(0, current_font_family);
        }
        let font_families: Vec<FontFamilySelectItem> = font_names
            .into_iter()
            .map(|name| FontFamilySelectItem::new(gpui::SharedString::from(name)))
            .collect();
        let font_family_selected_row = font_families
            .iter()
            .position(|item| item.name.as_ref() == settings.terminal.font_family.as_ref());
        cx.new(|cx| {
            SelectState::new(
                SearchableVec::new(font_families),
                font_family_selected_row.map(Self::row_index),
                window,
                cx,
            )
            .searchable(true)
        })
    }

    fn terminal_backend_select(
        settings: &SettingsFile,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> gpui::Entity<SelectState<SearchableVec<TerminalBackendSelectItem>>> {
        let items = vec![
            TerminalBackendSelectItem::new(
                TerminalBackend::Alacritty,
                "termua-settings-terminal-default-backend-icon",
            ),
            TerminalBackendSelectItem::new(
                TerminalBackend::Wezterm,
                "termua-settings-terminal-default-backend-icon",
            ),
        ];
        let selected_row = Some(match settings.terminal.default_backend {
            TerminalBackend::Alacritty => 0,
            TerminalBackend::Wezterm => 1,
        });
        Self::new_select(window, cx, items, selected_row)
    }

    fn ssh_backend_select(
        settings: &SettingsFile,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> gpui::Entity<SelectState<SearchableVec<SshBackendSelectItem>>> {
        let items = vec![
            SshBackendSelectItem::new(SshBackend::Ssh2),
            SshBackendSelectItem::new(SshBackend::Libssh),
        ];
        let selected_row = Some(match settings.terminal.ssh_backend {
            SshBackend::Ssh2 => 0,
            SshBackend::Libssh => 1,
        });
        Self::new_select(window, cx, items, selected_row)
    }

    pub fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        window.set_window_title(t!("Settings.WindowTitle").as_ref());

        let focus_handle = cx.focus_handle();
        let settings = load_settings_from_disk().unwrap_or_default();
        let current_language = settings.appearance.language;
        crate::settings::ensure_language_state_with_default(current_language, cx);

        let selected_page = Self::selected_page_from_settings(&settings);
        let search_input = Self::search_input(window, cx);

        let lock_overlay = crate::lock_screen::overlay::LockOverlayState::new(window, cx);
        let logging_path_input = Self::logging_path_input(&settings, window, cx);

        let static_suggestions_reload_in_flight = false;

        let font_family_select = Self::font_family_select(&settings, window, cx);
        let terminal_default_backend_select = Self::terminal_backend_select(&settings, window, cx);
        let terminal_ssh_backend_select = Self::ssh_backend_select(&settings, window, cx);

        let terminal_keybinding_focus = std::array::from_fn(|_| cx.focus_handle());

        let nav_tree_items = build_nav_tree_items();
        let nav_tree_state = cx.new(|cx| TreeState::new(cx).items(nav_tree_items.clone()));

        let mut this = Self {
            focus_handle,
            search_input,
            lock_overlay,
            selected_page,
            nav_tree_state,
            nav_tree_items,
            current_language,
            settings,
            save_epoch: 0,
            right_scroll_handle: ScrollHandle::default(),
            font_family_select,
            terminal_default_backend_select,
            terminal_ssh_backend_select,
            terminal_keybinding_focus,
            logging_path_input,
            static_suggestions_reload_in_flight,
            _subscriptions: Vec::new(),
        };

        this.install_subscriptions(window, cx);
        this.sync_nav_tree_selection(cx);
        this
    }

    fn install_subscriptions(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.install_search_subscription(window, cx);
        self.install_logging_path_subscription(window, cx);
        self.install_terminal_subscriptions(window, cx);
        self.install_lock_state_subscription(window, cx);
        self.install_language_settings_subscription(window, cx);
    }

    fn install_search_subscription(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self._subscriptions
            .push(cx.subscribe_in(&self.search_input, window, {
                move |_this, _input, ev, _window, cx| {
                    if matches!(ev, InputEvent::Change) {
                        cx.notify();
                    }
                }
            }));
    }

    fn install_logging_path_subscription(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let logging_path_input = self.logging_path_input.clone();
        self.subscribe_change_input(
            &logging_path_input,
            window,
            cx,
            |this, value, window, cx| {
                this.settings.logging.path = value;
                this.save_only(window, cx);
            },
        );
    }

    fn install_terminal_subscriptions(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let font_family_select = self.font_family_select.clone();
        self.subscribe_select_confirm(
            &font_family_select,
            window,
            cx,
            |this, font_family, window, cx| {
                this.settings.terminal.font_family = font_family.clone();
                this.apply_and_save(window, cx);
            },
        );

        let terminal_default_backend_select = self.terminal_default_backend_select.clone();
        self.subscribe_select_confirm(
            &terminal_default_backend_select,
            window,
            cx,
            |this, backend, window, cx| {
                this.settings.terminal.default_backend = *backend;
                this.apply_and_save(window, cx);
            },
        );

        let terminal_ssh_backend_select = self.terminal_ssh_backend_select.clone();
        self.subscribe_select_confirm(
            &terminal_ssh_backend_select,
            window,
            cx,
            |this, backend, window, cx| {
                this.settings.terminal.ssh_backend = *backend;
                this.apply_and_save(window, cx);
            },
        );
    }

    fn install_lock_state_subscription(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self._subscriptions
            .push(cx.observe_global_in::<crate::lock_screen::LockState>(
                window,
                |this, window, cx| {
                    cx.notify();
                    window.refresh();
                    let locked = cx.global::<crate::lock_screen::LockState>().locked();
                    if locked {
                        this.lock_overlay.password_input.update(cx, |state, cx| {
                            state.set_masked(true, window, cx);
                        });
                        let focus = this.lock_overlay.password_input.read(cx).focus_handle(cx);
                        window.defer(cx, move |window, cx| window.focus(&focus, cx));
                    } else {
                        this.lock_overlay.error = None;
                    }
                },
            ));
    }

    fn install_language_settings_subscription(
        &mut self,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self._subscriptions
            .push(cx.observe_global_in::<crate::settings::LanguageSettings>(
                window,
                |this, window, cx| {
                    let next_language = cx.global::<crate::settings::LanguageSettings>().language;
                    if this.current_language == next_language {
                        return;
                    }
                    this.current_language = next_language;
                    this.sync_localized_strings(window, cx);
                    cx.notify();
                    window.refresh();
                },
            ));
    }
}

fn debug_id_fragment(name: &str) -> String {
    name.chars()
        .map(|c| if c.is_ascii_alphanumeric() { c } else { '_' })
        .collect()
}

#[derive(Clone)]
pub(super) struct FontFamilySelectItem {
    name: gpui::SharedString,
}

impl FontFamilySelectItem {
    fn new(name: gpui::SharedString) -> Self {
        Self { name }
    }
}

impl SelectItem for FontFamilySelectItem {
    type Value = gpui::SharedString;

    fn title(&self) -> gpui::SharedString {
        self.name.clone()
    }

    fn display_title(&self) -> Option<AnyElement> {
        Some(
            div()
                .whitespace_nowrap()
                .font_family(self.name.clone())
                .child(self.name.clone())
                .into_any_element(),
        )
    }

    fn render(&self, _window: &mut Window, _cx: &mut App) -> impl IntoElement {
        let selector = format!(
            "termua-settings-terminal-font-family-option-{}",
            debug_id_fragment(self.name.as_ref())
        );
        div()
            .debug_selector(move || selector)
            .whitespace_nowrap()
            .font_family(self.name.clone())
            .child(self.name.clone())
    }

    fn value(&self) -> &Self::Value {
        &self.name
    }
}
