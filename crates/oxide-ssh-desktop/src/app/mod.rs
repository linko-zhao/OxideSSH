use std::{collections::HashMap, path::PathBuf, sync::Arc};

use gpui::{
    AnyElement, App, Bounds, ClipboardItem, Context, Element, ElementId, ElementInputHandler,
    Entity, EntityInputHandler, FocusHandle, Focusable, GlobalElementId, InspectorElementId,
    KeyBinding, KeyDownEvent, LayoutId, MouseButton, MouseDownEvent, MouseMoveEvent, MouseUpEvent,
    PaintQuad, Pixels, Point, Render, ScrollWheelEvent, ShapedLine, SharedString,
    StrikethroughStyle, Style, TextRun, UnderlineStyle, WeakEntity, Window, WindowAppearance,
    actions, div, fill, point, prelude::*, px, relative, rgb, size,
};
use gpui_component::{
    ActiveTheme as _, Disableable as _, Icon, IconName, IndexPath, Root, Sizable as _,
    WindowExt as _,
    alert::Alert,
    button::{Button, ButtonVariants as _},
    checkbox::Checkbox,
    form::{field, v_form},
    h_flex,
    input::{Input, InputEvent, InputState},
    radio::{Radio, RadioGroup},
    select::{SearchableVec, SelectEvent, SelectState},
};
// The custom title bar only renders where the system title bar is transparent.
#[cfg(target_os = "macos")]
use gpui_component::TitleBar;
// Windows routes caption interactions through gpui's native WM_NCHITTEST
// path; see render_tab_strip.
#[cfg(target_os = "windows")]
use gpui::WindowControlArea;
use oxide_ssh_core::{
    credentials::{CredentialError, CredentialStore},
    model::{
        AuthConfig, ConnectionProfile, Endpoint, LocaleSetting, NewTabAction, Osc52Clipboard,
        ProfileId, SessionId, Tag, TagColor, TagId, TerminalBell, TerminalSize as CoreTerminalSize,
        ThemeSetting,
    },
    session::{
        ConnectRequest, HostKeyDecision, SessionError, SessionEvent, SshService,
        private_key_requires_passphrase,
    },
    storage::StorageError,
};
use oxide_ssh_terminal::{CellRenderStyle, CellSide, Key, KeyInput, TerminalSize};
use secrecy::SecretString;
use uuid::Uuid;

use crate::{
    credentials::{
        CredentialTransactionError, ProfileCredentialCoordinator, SystemCredentialStore,
        credential_error_message_id,
    },
    i18n::{Catalog, MessageId, ResolvedLocale},
    state::{
        AppLoadOutcome, AppState,
        connection_form::{AuthMethod, ConnectionForm, FormError},
        theme::{CARD_GAP, CARD_PADDING, ResolvedTheme, apply_theme},
    },
};
use oxide_ssh_app::tabs::{
    DisconnectReason, ModalRequest, TabCollection, TabId, TabKind, TabLocalError, TabNotification,
    TabState,
};

mod messages;

use self::messages::{
    form_error_message_id, is_dark, local_error_message_id, session_error_message_id,
    system_locale, tag_storage_error_message_id, transaction_error_message_id,
};

mod editor;

use self::editor::{ConnectionEditor, EditorId};

mod dialog;

use self::dialog::{ConfirmAction, DialogPresenter};

mod tags;

use self::tags::TagManager;

mod session;

mod terminal;

use self::terminal::{RgbColorExt, TerminalGeometry};

mod views;

actions!(
    oxide_ssh,
    [
        AddConnection,
        LocalTerminal,
        OpenSettings,
        NextTab,
        PreviousTab,
        CloseTab,
        Copy,
        Paste,
        TerminalTab,
        TerminalShiftTab,
        DialogFocusNext,
        DialogFocusPrev,
        DialogSubmit,
        DialogCancel,
        QuitApp,
        NewWindow,
    ]
);

pub fn init(cx: &mut App) {
    let mut bindings = vec![
        KeyBinding::new("tab", TerminalTab, Some("Terminal")),
        KeyBinding::new("shift-tab", TerminalShiftTab, Some("Terminal")),
        // Registered after gpui_component::init so these override the Input
        // bindings only inside dialogs (deepest-context precedence).
        KeyBinding::new("tab", DialogFocusNext, Some("OxideSSHDialog")),
        KeyBinding::new("shift-tab", DialogFocusPrev, Some("OxideSSHDialog")),
        KeyBinding::new("tab", DialogFocusNext, Some("OxideSSHDialog > Input")),
        KeyBinding::new("shift-tab", DialogFocusPrev, Some("OxideSSHDialog > Input")),
        KeyBinding::new("enter", DialogSubmit, Some("OxideSSHDialog")),
        KeyBinding::new("enter", DialogSubmit, Some("OxideSSHDialog > Input")),
        KeyBinding::new("escape", DialogCancel, Some("OxideSSHDialog")),
        KeyBinding::new("escape", DialogCancel, Some("OxideSSHDialog > Input")),
        KeyBinding::new("ctrl-tab", NextTab, None),
        KeyBinding::new("ctrl-shift-tab", PreviousTab, None),
    ];
    #[cfg(target_os = "macos")]
    {
        bindings.extend([
            KeyBinding::new("cmd-n", AddConnection, None),
            KeyBinding::new("cmd-shift-n", LocalTerminal, None),
            KeyBinding::new("cmd-,", OpenSettings, None),
            KeyBinding::new("cmd-w", CloseTab, None),
            KeyBinding::new("cmd-q", QuitApp, None),
            KeyBinding::new("cmd-c", Copy, None),
            KeyBinding::new("cmd-v", Paste, None),
            KeyBinding::new("cmd-shift-[", PreviousTab, None),
            KeyBinding::new("cmd-shift-]", NextTab, None),
        ]);
    }
    #[cfg(not(target_os = "macos"))]
    {
        bindings.extend([
            KeyBinding::new("ctrl-n", AddConnection, None),
            KeyBinding::new("ctrl-shift-n", LocalTerminal, None),
            KeyBinding::new("ctrl-,", OpenSettings, None),
            KeyBinding::new("ctrl-shift-w", CloseTab, None),
            KeyBinding::new("ctrl-shift-c", Copy, None),
            KeyBinding::new("ctrl-shift-v", Paste, None),
        ]);
    }
    cx.bind_keys(bindings);
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum MainView {
    Workspace,
    Sessions,
    Settings,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum SettingsCategory {
    General,
    Appearance,
    Security,
}

fn new_input(
    window: &mut Window,
    cx: &mut Context<AppView>,
    placeholder: MessageId,
    value: &str,
    masked: bool,
    locale: ResolvedLocale,
) -> Entity<InputState> {
    let placeholder = Catalog::text(locale, placeholder);
    let value = value.to_owned();
    cx.new(|cx| {
        InputState::new(window, cx)
            .placeholder(placeholder)
            .default_value(value)
            .masked(masked)
    })
}

type SettingSelect = Entity<SelectState<SearchableVec<SharedString>>>;

fn setting_select_items(locale: ResolvedLocale, ids: &[MessageId]) -> SearchableVec<SharedString> {
    SearchableVec::new(
        ids.iter()
            .map(|id| SharedString::from(Catalog::text(locale, *id)))
            .collect::<Vec<_>>(),
    )
}

fn new_setting_select(
    window: &mut Window,
    cx: &mut Context<AppView>,
    locale: ResolvedLocale,
    ids: &[MessageId],
    selected_index: usize,
) -> SettingSelect {
    let items = setting_select_items(locale, ids);
    cx.new(|cx| SelectState::new(items, Some(IndexPath::new(selected_index)), window, cx))
}

/// Muted, theme-agnostic hues for tag colors; mirrors the host-card avatar
/// palette so tags sit comfortably on light and dark surfaces.
fn tag_color_rgb(color: TagColor) -> u32 {
    match color {
        TagColor::Red => 0x00b5_7e7e,
        TagColor::Orange => 0x00b5_936b,
        TagColor::Yellow => 0x00b5_a86b,
        TagColor::Green => 0x007b_a389,
        TagColor::Blue => 0x006b_8cae,
        TagColor::Purple => 0x009a_7bb5,
        TagColor::Pink => 0x00b5_7ea0,
        TagColor::Gray => 0x008a_9099,
    }
}

pub struct AppView {
    root: PathBuf,
    state: Option<AppState>,
    credentials: Option<Arc<SystemCredentialStore>>,
    coordinator: Option<Arc<ProfileCredentialCoordinator<SystemCredentialStore>>>,
    ssh: Option<SshService>,
    tabs: TabCollection,
    search: Entity<InputState>,
    secret: Entity<InputState>,
    connect_secrets: HashMap<ProfileId, SecretString>,
    connecting_profiles: std::collections::HashSet<ProfileId>,
    editor: Option<ConnectionEditor>,
    tag_manager: Option<TagManager>,
    confirm: Option<ConfirmAction>,
    saving_editor: Option<EditorId>,
    dialog_presenter: Option<Entity<DialogPresenter>>,
    main_view: MainView,
    settings_category: SettingsCategory,
    locale_select: SettingSelect,
    theme_select: SettingSelect,
    ssh_connect_timeout_select: SettingSelect,
    new_tab_action_select: SettingSelect,
    ansi_palette_input: Entity<InputState>,
    ssh_keepalive_interval: Entity<InputState>,
    ssh_keepalive_max_count: Entity<InputState>,
    terminal_bell_select: SettingSelect,
    paste_threshold_select: SettingSelect,
    local_working_directory: Entity<InputState>,
    osc52_clipboard_select: SettingSelect,
    ssh_term_input: Entity<InputState>,
    locale: ResolvedLocale,
    system_locale: Option<String>,
    theme: ResolvedTheme,
    focus_handle: FocusHandle,
    terminal_focus: FocusHandle,
    pending_terminal_focus: bool,
    composing: bool,
    compose_tab: Option<TabId>,
    scroll_accumulator: f32,
    bell: bool,
    last_bell_sound: Option<std::time::Instant>,
    bell_generation: u64,
    status_message: Option<MessageId>,
    credential_clear_failures: Vec<String>,
    status_rendered: Option<MessageId>,
    terminal_geometry: Option<TerminalGeometry>,
    terminal_selecting: bool,
    terminal_selection_dragged: bool,
    quitting: bool,
    _search_subscription: gpui::Subscription,
    _settings_subscriptions: Vec<gpui::Subscription>,
    _bounds_subscription: gpui::Subscription,
}

impl AppView {
    pub(super) fn terminal_colors(&self) -> oxide_ssh_terminal::TerminalColors {
        self.theme.terminal_colors(
            self.state
                .as_ref()
                .and_then(|state| state.settings().ansi_palette),
        )
    }
    pub fn new(
        root: PathBuf,
        outcome: AppLoadOutcome,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Self {
        let system_locale = system_locale();
        let system_is_dark = is_dark(window.appearance());
        let (state, credentials, coordinator, ssh, status_message) = match outcome {
            AppLoadOutcome::Ready(state) => {
                let store = state.store().clone();
                let credentials = Arc::new(SystemCredentialStore::new());
                let coordinator = Arc::new(ProfileCredentialCoordinator::new(
                    store.clone(),
                    credentials.clone(),
                ));
                let ssh = SshService::new(store);
                let status_message = ssh
                    .as_ref()
                    .err()
                    .map(|error| session_error_message_id(*error));
                (
                    Some(state),
                    Some(credentials),
                    Some(coordinator),
                    ssh.ok(),
                    status_message,
                )
            }
            AppLoadOutcome::Recovery(_) => (None, None, None, None, None),
        };
        let locale = state
            .as_ref()
            .map(|state| state.resolved_locale(system_locale.as_deref()))
            .unwrap_or_else(|| {
                ResolvedLocale::resolve(LocaleSetting::System, system_locale.as_deref())
            });
        let theme = state
            .as_ref()
            .map(|state| state.resolved_theme(system_is_dark))
            .unwrap_or(ResolvedTheme::Dark);
        apply_theme(theme, window, cx);
        gpui_component::set_locale(match locale {
            ResolvedLocale::EnUs => "en",
            ResolvedLocale::ZhCn => "zh-CN",
        });
        // shortcut: dock label snapshots the startup locale; a mid-session
        // language switch only updates it after restart.
        #[cfg(target_os = "macos")]
        cx.set_dock_menu(vec![gpui::MenuItem::action(
            Catalog::text(locale, MessageId::NewWindow),
            NewWindow,
        )]);
        let search = new_input(window, cx, MessageId::SearchConnections, "", false, locale);
        let secret = new_input(window, cx, MessageId::CredentialRequired, "", true, locale);
        let search_subscription = cx.observe(&search, |this, _, cx| {
            let _ = this;
            cx.notify();
        });
        let preferences = state
            .as_ref()
            .map(|state| state.settings().clone())
            .unwrap_or_default();
        let ansi_palette_input = cx.new(|cx| {
            InputState::new(window, cx).default_value(
                theme
                    .terminal_colors(preferences.ansi_palette)
                    .ansi
                    .iter()
                    .map(|color| format!("{:06x}", color.to_hex()))
                    .collect::<Vec<_>>()
                    .join(","),
            )
        });
        let ssh_term_input = new_input(
            window,
            cx,
            MessageId::SshTerm,
            &preferences.ssh_term,
            false,
            locale,
        );
        let locale_select = new_setting_select(
            window,
            cx,
            locale,
            &[
                MessageId::System,
                MessageId::English,
                MessageId::SimplifiedChinese,
            ],
            match preferences.locale {
                LocaleSetting::System => 0,
                LocaleSetting::EnUs => 1,
                LocaleSetting::ZhCn => 2,
            },
        );
        let theme_select = new_setting_select(
            window,
            cx,
            locale,
            &[MessageId::System, MessageId::Light, MessageId::Dark],
            match preferences.theme {
                ThemeSetting::System => 0,
                ThemeSetting::Light => 1,
                ThemeSetting::Dark => 2,
            },
        );
        let ssh_connect_timeout_select = new_setting_select(
            window,
            cx,
            locale,
            &[
                MessageId::Timeout15Seconds,
                MessageId::Timeout30Seconds,
                MessageId::Timeout60Seconds,
                MessageId::Timeout120Seconds,
            ],
            match preferences.ssh_connect_timeout_seconds {
                30 => 1,
                60 => 2,
                120 => 3,
                _ => 0,
            },
        );
        let new_tab_action_select = new_setting_select(
            window,
            cx,
            locale,
            &[MessageId::AddConnection, MessageId::LocalTerminal],
            match preferences.new_tab_action {
                NewTabAction::AddConnection => 0,
                NewTabAction::LocalTerminal => 1,
            },
        );
        let ssh_keepalive_interval = new_input(
            window,
            cx,
            MessageId::SshKeepaliveInterval,
            &preferences.ssh_keepalive_interval_seconds.to_string(),
            false,
            locale,
        );
        let ssh_keepalive_max_count = new_input(
            window,
            cx,
            MessageId::SshKeepaliveMaxCount,
            &preferences.ssh_keepalive_max_count.to_string(),
            false,
            locale,
        );
        let terminal_bell_select = new_setting_select(
            window,
            cx,
            locale,
            &[
                MessageId::BellVisual,
                MessageId::BellSound,
                MessageId::BellOff,
            ],
            match preferences.terminal_bell {
                TerminalBell::Visual => 0,
                TerminalBell::Sound => 1,
                TerminalBell::Off => 2,
            },
        );
        let paste_threshold_select = new_setting_select(
            window,
            cx,
            locale,
            &[
                MessageId::PasteThreshold1KiB,
                MessageId::PasteThreshold4KiB,
                MessageId::PasteThreshold16KiB,
            ],
            match preferences.paste_confirm_threshold {
                0..=1024 => 0,
                1025..=4096 => 1,
                _ => 2,
            },
        );
        let local_working_directory = new_input(
            window,
            cx,
            MessageId::LocalWorkingDirectory,
            &preferences.local_working_directory,
            false,
            locale,
        );
        let osc52_clipboard_select = new_setting_select(
            window,
            cx,
            locale,
            &[
                MessageId::Osc52ClipboardOff,
                MessageId::Osc52ClipboardAsk,
                MessageId::Osc52ClipboardOn,
            ],
            match preferences.osc52_clipboard {
                Osc52Clipboard::Off => 0,
                Osc52Clipboard::Ask => 1,
                Osc52Clipboard::On => 2,
            },
        );
        let settings_subscriptions =
            vec![
                cx.subscribe_in(
                    &locale_select,
                    window,
                    |this, _, event: &SelectEvent<SearchableVec<SharedString>>, window, cx| {
                        let SelectEvent::Confirm(Some(value)) = event else {
                            return;
                        };
                        let setting = if value.as_str() == this.text(MessageId::English) {
                            LocaleSetting::EnUs
                        } else if value.as_str() == this.text(MessageId::SimplifiedChinese) {
                            LocaleSetting::ZhCn
                        } else {
                            LocaleSetting::System
                        };
                        this.set_locale(setting, window, cx);
                    },
                ),
                cx.subscribe_in(
                    &theme_select,
                    window,
                    |this, _, event: &SelectEvent<SearchableVec<SharedString>>, window, cx| {
                        let SelectEvent::Confirm(Some(value)) = event else {
                            return;
                        };
                        let setting = if value.as_str() == this.text(MessageId::Light) {
                            ThemeSetting::Light
                        } else if value.as_str() == this.text(MessageId::Dark) {
                            ThemeSetting::Dark
                        } else {
                            ThemeSetting::System
                        };
                        this.set_theme(setting, window, cx);
                    },
                ),
                cx.subscribe_in(
                    &ssh_connect_timeout_select,
                    window,
                    |this, _, event: &SelectEvent<SearchableVec<SharedString>>, _, cx| {
                        let SelectEvent::Confirm(Some(value)) = event else {
                            return;
                        };
                        let seconds = if value.as_str() == this.text(MessageId::Timeout30Seconds) {
                            30
                        } else if value.as_str() == this.text(MessageId::Timeout60Seconds) {
                            60
                        } else if value.as_str() == this.text(MessageId::Timeout120Seconds) {
                            120
                        } else {
                            15
                        };
                        this.set_ssh_connect_timeout_seconds(seconds, cx);
                    },
                ),
                cx.subscribe_in(
                    &new_tab_action_select,
                    window,
                    |this, _, event: &SelectEvent<SearchableVec<SharedString>>, _, cx| {
                        let SelectEvent::Confirm(Some(value)) = event else {
                            return;
                        };
                        let setting = if value.as_str() == this.text(MessageId::LocalTerminal) {
                            NewTabAction::LocalTerminal
                        } else {
                            NewTabAction::AddConnection
                        };
                        this.set_new_tab_action(setting, cx);
                    },
                ),
                cx.subscribe_in(
                    &ssh_keepalive_interval,
                    window,
                    |this, _, event: &InputEvent, _, cx| {
                        if matches!(event, InputEvent::Blur) {
                            this.commit_ssh_keepalive(cx);
                        }
                    },
                ),
                cx.subscribe_in(
                    &ssh_keepalive_max_count,
                    window,
                    |this, _, event: &InputEvent, _, cx| {
                        if matches!(event, InputEvent::Blur) {
                            this.commit_ssh_keepalive(cx);
                        }
                    },
                ),
                cx.subscribe_in(
                    &terminal_bell_select,
                    window,
                    |this, _, event: &SelectEvent<SearchableVec<SharedString>>, _, cx| {
                        let SelectEvent::Confirm(Some(value)) = event else {
                            return;
                        };
                        let setting = if value.as_str() == this.text(MessageId::BellSound) {
                            TerminalBell::Sound
                        } else if value.as_str() == this.text(MessageId::BellOff) {
                            TerminalBell::Off
                        } else {
                            TerminalBell::Visual
                        };
                        this.set_terminal_bell(setting, cx);
                    },
                ),
                cx.subscribe_in(
                    &paste_threshold_select,
                    window,
                    |this, _, event: &SelectEvent<SearchableVec<SharedString>>, _, cx| {
                        let SelectEvent::Confirm(Some(value)) = event else {
                            return;
                        };
                        let threshold =
                            if value.as_str() == this.text(MessageId::PasteThreshold1KiB) {
                                1024
                            } else if value.as_str() == this.text(MessageId::PasteThreshold16KiB) {
                                16 * 1024
                            } else {
                                4 * 1024
                            };
                        if this.state.as_mut().is_some_and(|state| {
                            state.set_paste_confirm_threshold(threshold).is_err()
                        }) {
                            this.status_message = Some(MessageId::StorageCorrupt);
                        }
                        cx.notify();
                    },
                ),
                cx.subscribe_in(
                    &local_working_directory,
                    window,
                    |this, _, event: &InputEvent, _window, cx| {
                        if matches!(event, InputEvent::Blur) {
                            let value = this.local_working_directory.read(cx).value().to_string();
                            if this.state.as_mut().is_some_and(|state| {
                                state.set_local_working_directory(value).is_err()
                            }) {
                                this.status_message = Some(MessageId::StorageCorrupt);
                            }
                            cx.notify();
                        }
                    },
                ),
                cx.subscribe_in(
                    &osc52_clipboard_select,
                    window,
                    |this, _, event: &SelectEvent<SearchableVec<SharedString>>, _, cx| {
                        let SelectEvent::Confirm(Some(value)) = event else {
                            return;
                        };
                        let setting = if value.as_str() == this.text(MessageId::Osc52ClipboardAsk) {
                            Osc52Clipboard::Ask
                        } else if value.as_str() == this.text(MessageId::Osc52ClipboardOn) {
                            Osc52Clipboard::On
                        } else {
                            Osc52Clipboard::Off
                        };
                        this.set_osc52_clipboard(setting, cx);
                    },
                ),
                cx.subscribe_in(
                    &ssh_term_input,
                    window,
                    |this, input, event: &InputEvent, _, cx| {
                        if matches!(event, InputEvent::Blur) {
                            let term = input.read(cx).value().to_string();
                            this.set_ssh_term(&term, cx);
                        }
                    },
                ),
            ];
        let bounds_subscription = cx.observe_window_bounds(window, |this, window, cx| {
            if let Some(state) = &mut this.state {
                let bounds = window.bounds();
                if state
                    .set_window_size(u32::from(bounds.size.width), u32::from(bounds.size.height))
                    .is_err()
                {
                    this.status_message = Some(MessageId::StorageCorrupt);
                    cx.notify();
                }
            }
        });
        Self {
            root,
            state,
            credentials,
            coordinator,
            ssh,
            tabs: TabCollection::new(),
            search,
            secret,
            connect_secrets: HashMap::new(),
            connecting_profiles: std::collections::HashSet::new(),
            editor: None,
            tag_manager: None,
            confirm: None,
            saving_editor: None,
            dialog_presenter: None,
            main_view: MainView::Workspace,
            settings_category: SettingsCategory::General,
            locale_select,
            paste_threshold_select,
            theme_select,
            ssh_connect_timeout_select,
            new_tab_action_select,
            ansi_palette_input,
            ssh_keepalive_interval,
            ssh_keepalive_max_count,
            terminal_bell_select,
            local_working_directory,
            osc52_clipboard_select,
            locale,
            system_locale,
            theme,
            focus_handle: cx.focus_handle(),
            terminal_focus: cx.focus_handle(),
            pending_terminal_focus: false,
            composing: false,
            compose_tab: None,
            scroll_accumulator: 0.0,
            bell: false,
            last_bell_sound: None,
            bell_generation: 0,
            ssh_term_input,
            status_message,
            credential_clear_failures: Vec::new(),
            status_rendered: None,
            terminal_geometry: None,
            terminal_selecting: false,
            terminal_selection_dragged: false,
            quitting: false,
            _search_subscription: search_subscription,
            _settings_subscriptions: settings_subscriptions,
            _bounds_subscription: bounds_subscription,
        }
    }

    pub fn has_live_tabs(&self) -> bool {
        !self.quitting && self.tabs.tabs().iter().any(|tab| !tab.is_disconnected())
    }

    pub fn request_quit(&mut self, cx: &mut Context<Self>) {
        if self.confirm_before_close_with_sessions() && self.has_live_tabs() {
            self.confirm = Some(ConfirmAction::Quit);
            cx.notify();
        }
    }

    pub fn confirm_before_close_with_sessions(&self) -> bool {
        self.state
            .as_ref()
            .is_none_or(|state| state.settings().confirm_before_close_with_sessions)
    }

    fn on_quit_app(&mut self, _: &QuitApp, _: &mut Window, cx: &mut Context<Self>) {
        if self.confirm_before_close_with_sessions() && self.has_live_tabs() {
            self.confirm = Some(ConfirmAction::QuitApp);
            cx.notify();
        } else {
            cx.quit();
        }
    }

    fn text(&self, id: MessageId) -> &'static str {
        Catalog::text(self.locale, id)
    }

    fn refresh_preferences(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some((locale, theme, has_custom_ansi_palette)) = self.state.as_ref().map(|state| {
            (
                state.resolved_locale(self.system_locale.as_deref()),
                state.resolved_theme(is_dark(window.appearance())),
                state.settings().ansi_palette.is_some(),
            )
        }) else {
            return;
        };
        if self.locale != locale {
            self.locale = locale;
            gpui_component::set_locale(match locale {
                ResolvedLocale::EnUs => "en",
                ResolvedLocale::ZhCn => "zh-CN",
            });
            self.update_input_placeholders(window, cx);
            self.update_setting_selects(window, cx);
        }
        if self.theme != theme {
            self.theme = theme;
            apply_theme(theme, window, cx);
            self.tabs.set_terminal_colors(self.terminal_colors());
            if !has_custom_ansi_palette {
                let value = self
                    .terminal_colors()
                    .ansi
                    .iter()
                    .map(|color| format!("{:06x}", color.to_hex()))
                    .collect::<Vec<_>>()
                    .join(",");
                self.ansi_palette_input.update(cx, |input, cx| {
                    input.set_value(value, window, cx);
                });
            }
        }
    }

    fn update_setting_selects(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(state) = &self.state else {
            return;
        };
        let settings = state.settings().clone();
        let locale = self.locale;
        self.locale_select.update(cx, |select, cx| {
            select.set_items(
                setting_select_items(
                    locale,
                    &[
                        MessageId::System,
                        MessageId::English,
                        MessageId::SimplifiedChinese,
                    ],
                ),
                window,
                cx,
            );
            select.set_selected_index(
                Some(IndexPath::new(match settings.locale {
                    LocaleSetting::System => 0,
                    LocaleSetting::EnUs => 1,
                    LocaleSetting::ZhCn => 2,
                })),
                window,
                cx,
            );
        });
        self.theme_select.update(cx, |select, cx| {
            select.set_items(
                setting_select_items(
                    locale,
                    &[MessageId::System, MessageId::Light, MessageId::Dark],
                ),
                window,
                cx,
            );
            select.set_selected_index(
                Some(IndexPath::new(match settings.theme {
                    ThemeSetting::System => 0,
                    ThemeSetting::Light => 1,
                    ThemeSetting::Dark => 2,
                })),
                window,
                cx,
            );
        });
        self.ssh_connect_timeout_select.update(cx, |select, cx| {
            select.set_items(
                setting_select_items(
                    locale,
                    &[
                        MessageId::Timeout15Seconds,
                        MessageId::Timeout30Seconds,
                        MessageId::Timeout60Seconds,
                        MessageId::Timeout120Seconds,
                    ],
                ),
                window,
                cx,
            );
            select.set_selected_index(
                Some(IndexPath::new(match settings.ssh_connect_timeout_seconds {
                    30 => 1,
                    60 => 2,
                    120 => 3,
                    _ => 0,
                })),
                window,
                cx,
            );
        });
        self.new_tab_action_select.update(cx, |select, cx| {
            select.set_items(
                setting_select_items(
                    locale,
                    &[MessageId::AddConnection, MessageId::LocalTerminal],
                ),
                window,
                cx,
            );
            select.set_selected_index(
                Some(IndexPath::new(match settings.new_tab_action {
                    NewTabAction::AddConnection => 0,
                    NewTabAction::LocalTerminal => 1,
                })),
                window,
                cx,
            );
        });
        self.terminal_bell_select.update(cx, |select, cx| {
            select.set_items(
                setting_select_items(
                    locale,
                    &[
                        MessageId::BellVisual,
                        MessageId::BellSound,
                        MessageId::BellOff,
                    ],
                ),
                window,
                cx,
            );
            select.set_selected_index(
                Some(IndexPath::new(match settings.terminal_bell {
                    TerminalBell::Visual => 0,
                    TerminalBell::Sound => 1,
                    TerminalBell::Off => 2,
                })),
                window,
                cx,
            );
        });
        self.paste_threshold_select.update(cx, |select, cx| {
            select.set_items(
                setting_select_items(
                    locale,
                    &[
                        MessageId::PasteThreshold1KiB,
                        MessageId::PasteThreshold4KiB,
                        MessageId::PasteThreshold16KiB,
                    ],
                ),
                window,
                cx,
            );
            select.set_selected_index(
                Some(IndexPath::new(match settings.paste_confirm_threshold {
                    0..=1024 => 0,
                    1025..=4096 => 1,
                    _ => 2,
                })),
                window,
                cx,
            );
        });
        self.osc52_clipboard_select.update(cx, |select, cx| {
            select.set_items(
                setting_select_items(
                    locale,
                    &[
                        MessageId::Osc52ClipboardOff,
                        MessageId::Osc52ClipboardAsk,
                        MessageId::Osc52ClipboardOn,
                    ],
                ),
                window,
                cx,
            );
            select.set_selected_index(
                Some(IndexPath::new(match settings.osc52_clipboard {
                    Osc52Clipboard::Off => 0,
                    Osc52Clipboard::Ask => 1,
                    Osc52Clipboard::On => 2,
                })),
                window,
                cx,
            );
        });
    }

    fn update_input_placeholders(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let locale = self.locale;
        self.search.update(cx, |field, cx| {
            field.set_placeholder(
                Catalog::text(locale, MessageId::SearchConnections),
                window,
                cx,
            );
        });
        self.secret.update(cx, |field, cx| {
            field.set_placeholder(
                Catalog::text(locale, MessageId::CredentialRequired),
                window,
                cx,
            );
        });
        if let Some(editor) = &self.editor {
            let fields = [
                (&editor.name, MessageId::Name),
                (&editor.host, MessageId::Host),
                (&editor.port, MessageId::Port),
                (&editor.username, MessageId::Username),
                (&editor.private_key_path, MessageId::PrivateKey),
                (
                    &editor.ssh_keepalive_interval,
                    MessageId::ProfileKeepaliveInterval,
                ),
                (
                    &editor.ssh_keepalive_max_count,
                    MessageId::ProfileKeepaliveMaxCount,
                ),
                (
                    &editor.secret,
                    if editor.form.auth_method == AuthMethod::PrivateKey {
                        MessageId::Passphrase
                    } else {
                        MessageId::Password
                    },
                ),
                (&editor.tag_name, MessageId::NewTag),
            ];
            for (entity, placeholder) in fields {
                entity.update(cx, |field, cx| {
                    field.set_placeholder(Catalog::text(locale, placeholder), window, cx);
                });
            }
        }
        if let Some(manager) = &self.tag_manager {
            for entry in &manager.entries {
                entry.name.update(cx, |field, cx| {
                    field.set_placeholder(Catalog::text(locale, MessageId::Name), window, cx);
                });
            }
        }
    }

    fn set_locale(&mut self, setting: LocaleSetting, window: &mut Window, cx: &mut Context<Self>) {
        if self
            .state
            .as_mut()
            .is_some_and(|state| state.set_locale(setting).is_err())
        {
            self.status_message = Some(MessageId::StorageCorrupt);
        }
        self.refresh_preferences(window, cx);
        cx.notify();
    }

    fn set_theme(&mut self, setting: ThemeSetting, window: &mut Window, cx: &mut Context<Self>) {
        if self
            .state
            .as_mut()
            .is_some_and(|state| state.set_theme(setting).is_err())
        {
            self.status_message = Some(MessageId::StorageCorrupt);
        }
        self.refresh_preferences(window, cx);
        cx.notify();
    }
    fn set_new_tab_action(&mut self, setting: NewTabAction, cx: &mut Context<Self>) {
        if self
            .state
            .as_mut()
            .is_some_and(|state| state.set_new_tab_action(setting).is_err())
        {
            self.status_message = Some(MessageId::StorageCorrupt);
        }
        cx.notify();
    }
    fn set_ssh_connect_timeout_seconds(&mut self, seconds: u64, cx: &mut Context<Self>) {
        if self
            .state
            .as_mut()
            .is_some_and(|state| state.set_ssh_connect_timeout_seconds(seconds).is_err())
        {
            self.status_message = Some(MessageId::StorageCorrupt);
        }
        cx.notify();
    }
    fn set_ssh_term(&mut self, term: &str, cx: &mut Context<Self>) {
        if self
            .state
            .as_mut()
            .is_some_and(|state| state.set_ssh_term(term).is_err())
        {
            self.status_message = Some(MessageId::StorageCorrupt);
        }
        cx.notify();
    }

    fn set_ssh_compression(&mut self, enabled: bool, cx: &mut Context<Self>) {
        if self
            .state
            .as_mut()
            .is_some_and(|state| state.set_ssh_compression(enabled).is_err())
        {
            self.status_message = Some(MessageId::StorageCorrupt);
        }
        cx.notify();
    }

    fn set_confirm_before_close_with_sessions(&mut self, confirm: bool, cx: &mut Context<Self>) {
        if self.state.as_mut().is_some_and(|state| {
            state
                .set_confirm_before_close_with_sessions(confirm)
                .is_err()
        }) {
            self.status_message = Some(MessageId::StorageCorrupt);
        }
    }

    fn save_ansi_palette(&mut self, _: &mut Window, cx: &mut Context<Self>) {
        let parsed = self
            .ansi_palette_input
            .read(cx)
            .value()
            .split(',')
            .map(|entry| {
                let value = entry.trim().trim_start_matches('#');
                (value.len() == 6)
                    .then(|| u32::from_str_radix(value, 16).ok())
                    .flatten()
            })
            .collect::<Option<Vec<_>>>();
        let Some(values) = parsed.filter(|values| values.len() == 16) else {
            return;
        };
        let palette: [u32; 16] = values.try_into().expect("length checked");
        if self
            .state
            .as_mut()
            .is_some_and(|state| state.set_ansi_palette(Some(palette)).is_err())
        {
            self.status_message = Some(MessageId::StorageCorrupt);
            return;
        }
        self.tabs.set_terminal_colors(self.terminal_colors());
        cx.notify();
    }

    fn reset_ansi_palette(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let value = self
            .theme
            .terminal_colors(None)
            .ansi
            .iter()
            .map(|color| format!("{:06x}", color.to_hex()))
            .collect::<Vec<_>>()
            .join(",");
        if self
            .state
            .as_mut()
            .is_some_and(|state| state.set_ansi_palette(None).is_err())
        {
            self.status_message = Some(MessageId::StorageCorrupt);
            return;
        }
        self.ansi_palette_input
            .update(cx, |input, cx| input.set_value(value, window, cx));
        self.tabs.set_terminal_colors(self.terminal_colors());
        cx.notify();
    }

    fn commit_ssh_keepalive(&mut self, cx: &mut Context<Self>) {
        let interval = self
            .ssh_keepalive_interval
            .read(cx)
            .value()
            .trim()
            .parse::<u64>()
            .ok()
            .filter(|value| *value > 0);
        let max_count = self
            .ssh_keepalive_max_count
            .read(cx)
            .value()
            .trim()
            .parse::<usize>()
            .ok()
            .filter(|value| *value > 0);
        if let (Some(interval), Some(max_count)) = (interval, max_count)
            && self
                .state
                .as_mut()
                .is_some_and(|state| state.set_ssh_keepalive(interval, max_count).is_err())
        {
            self.status_message = Some(MessageId::StorageCorrupt);
        } else if interval.is_none() || max_count.is_none() {
            self.status_message = Some(MessageId::InvalidKeepalive);
        }
        cx.notify();
    }

    fn set_terminal_bell(&mut self, setting: TerminalBell, cx: &mut Context<Self>) {
        if self
            .state
            .as_mut()
            .is_some_and(|state| state.set_terminal_bell(setting).is_err())
        {
            self.status_message = Some(MessageId::StorageCorrupt);
        }
        if setting != TerminalBell::Visual {
            self.bell_generation = self.bell_generation.wrapping_add(1);
            self.bell = false;
        }
        cx.notify();
    }

    fn handle_terminal_bell(&mut self, cx: &mut Context<Self>) {
        match self
            .state
            .as_ref()
            .map(|state| state.settings().terminal_bell)
            .unwrap_or_default()
        {
            TerminalBell::Visual => {
                self.bell = true;
                self.bell_generation = self.bell_generation.wrapping_add(1);
                let generation = self.bell_generation;
                cx.spawn(async move |weak, cx| {
                    cx.background_executor()
                        .timer(std::time::Duration::from_secs(1))
                        .await;
                    weak.update(cx, |this, cx| {
                        if this.bell_generation == generation {
                            this.bell = false;
                            cx.notify();
                        }
                    })
                    .ok();
                })
                .detach();
            }
            TerminalBell::Sound => {
                let now = std::time::Instant::now();
                if self.last_bell_sound.is_none_or(|last| {
                    now.duration_since(last) >= std::time::Duration::from_millis(200)
                }) {
                    self.last_bell_sound = Some(now);
                    play_terminal_bell();
                }
            }
            TerminalBell::Off => {
                self.bell_generation = self.bell_generation.wrapping_add(1);
                self.bell = false;
            }
        }
    }

    fn set_osc52_clipboard(&mut self, setting: Osc52Clipboard, cx: &mut Context<Self>) {
        if self
            .state
            .as_mut()
            .is_some_and(|state| state.set_osc52_clipboard(setting).is_err())
        {
            self.status_message = Some(MessageId::StorageCorrupt);
        }
        cx.notify();
    }

    fn set_remember_window_size(
        &mut self,
        remember: bool,
        window: &Window,
        cx: &mut Context<Self>,
    ) {
        if self
            .state
            .as_mut()
            .is_some_and(|state| state.set_remember_window_size(remember).is_err())
        {
            self.status_message = Some(MessageId::StorageCorrupt);
        }
        if remember {
            let bounds = window.bounds();
            if self.state.as_mut().is_some_and(|state| {
                state
                    .set_window_size(u32::from(bounds.size.width), u32::from(bounds.size.height))
                    .is_err()
            }) {
                self.status_message = Some(MessageId::StorageCorrupt);
            }
        }
        cx.notify();
    }

    fn estimated_terminal_size(&self, window: &Window) -> TerminalSize {
        let bounds = window.bounds();
        // Chrome: sidebar+strip estimate (strip = 28px pill + 2 * CARD_GAP)
        // plus card gap and terminal padding.
        let chrome = 2. * (CARD_GAP + CARD_PADDING);
        let width = (bounds.size.width - px(260. + chrome)).max(px(80.));
        let height = (bounds.size.height - px(28. + 2. * CARD_GAP + chrome)).max(px(40.));
        let text_style = window.text_style();
        let font_size = text_style.font_size.to_pixels(window.rem_size());
        let cell_width = (font_size * 0.6).max(px(1.));
        let cell_height = window.line_height().max(px(1.));
        TerminalSize {
            columns: (f32::from(width) / f32::from(cell_width)).floor() as usize,
            rows: (f32::from(height) / f32::from(cell_height)).floor() as usize,
            pixel_width: u32::from(width),
            pixel_height: u32::from(height),
        }
    }

    fn retry_storage(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let outcome = AppState::load(self.root.clone());
        let AppLoadOutcome::Ready(state) = outcome else {
            return;
        };
        let store = state.store().clone();
        let credentials = Arc::new(SystemCredentialStore::new());
        let coordinator = Arc::new(ProfileCredentialCoordinator::new(
            store.clone(),
            credentials.clone(),
        ));
        match SshService::new(store) {
            Ok(ssh) => {
                self.state = Some(state);
                self.credentials = Some(credentials);
                self.coordinator = Some(coordinator);
                self.ssh = Some(ssh);
                self.refresh_preferences(window, cx);
                self.status_message = None;
            }
            Err(error) => self.status_message = Some(session_error_message_id(error)),
        }
        cx.notify();
    }

    fn open_config_directory(&self, cx: &mut Context<Self>) {
        cx.open_with_system(&self.root);
    }

    fn cycle_tab(&mut self, backwards: bool, cx: &mut Context<Self>) {
        let tabs = self.tabs.tabs();
        if tabs.len() < 2 {
            return;
        }
        let current = self
            .tabs
            .active()
            .and_then(|active| tabs.iter().position(|tab| tab.id() == active))
            .unwrap_or(0);
        let next = if backwards {
            current.checked_sub(1).unwrap_or(tabs.len() - 1)
        } else {
            (current + 1) % tabs.len()
        };
        let id = tabs[next].id();
        self.tabs.set_active(id);
        self.main_view = MainView::Sessions;
        cx.notify();
    }

    fn dismiss_status(&mut self, cx: &mut Context<Self>) {
        self.status_message = None;
        cx.notify();
    }

    fn sync_status(&mut self, cx: &mut Context<Self>) {
        if self.status_message == self.status_rendered {
            return;
        }
        self.status_rendered = self.status_message;
        let Some(message) = self.status_message else {
            return;
        };
        // Transient banner: clear it automatically unless a newer message
        // replaced it in the meantime.
        cx.spawn(async move |weak, cx| {
            cx.background_executor().timer(STATUS_TIMEOUT).await;
            weak.update(cx, |this, cx| {
                if this.status_message == Some(message) {
                    this.status_message = None;
                    cx.notify();
                }
            })
            .ok();
        })
        .detach();
    }

    fn on_add_connection(
        &mut self,
        _: &AddConnection,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.editor.is_none() {
            self.show_add_connection(window, cx);
        }
        cx.stop_propagation();
    }

    fn on_local_terminal(
        &mut self,
        _: &LocalTerminal,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.editor.is_none() {
            self.connect_local(window, cx);
        }
        cx.stop_propagation();
    }

    fn on_open_settings(&mut self, _: &OpenSettings, _: &mut Window, cx: &mut Context<Self>) {
        self.main_view = MainView::Settings;
        cx.notify();
        cx.stop_propagation();
    }

    fn on_next_tab(&mut self, _: &NextTab, _: &mut Window, cx: &mut Context<Self>) {
        self.cycle_tab(false, cx);
        cx.stop_propagation();
    }

    fn on_previous_tab(&mut self, _: &PreviousTab, _: &mut Window, cx: &mut Context<Self>) {
        self.cycle_tab(true, cx);
        cx.stop_propagation();
    }

    fn on_close_tab(&mut self, _: &CloseTab, _: &mut Window, cx: &mut Context<Self>) {
        if let Some(id) = self.tabs.active() {
            self.close_tab(id, cx);
        }
        cx.stop_propagation();
    }

    fn on_terminal_tab(&mut self, _: &TerminalTab, _: &mut Window, cx: &mut Context<Self>) {
        self.send_terminal_input(KeyInput::new(Key::Tab), cx);
        cx.stop_propagation();
    }

    fn on_terminal_shift_tab(
        &mut self,
        _: &TerminalShiftTab,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.send_terminal_input(KeyInput::new(Key::Tab).shift(), cx);
        cx.stop_propagation();
    }

    fn send_terminal_input(&mut self, input: KeyInput<'_>, cx: &mut Context<Self>) {
        let result = self
            .tabs
            .active()
            .and_then(|id| self.tabs.tab_mut(id))
            .map(|tab| tab.send_key(input));
        match result {
            Some(Ok(())) => {
                if self.status_message == Some(MessageId::InputQueueFull) {
                    self.status_message = None;
                }
            }
            Some(Err(error)) => {
                self.status_message = Some(local_error_message_id(error));
            }
            None => {}
        }
        cx.notify();
    }

    fn on_copy(&mut self, _: &Copy, window: &mut Window, cx: &mut Context<Self>) {
        if self.focus_handle.is_focused(window) {
            self.copy_terminal_selection(cx);
            cx.stop_propagation();
        }
    }

    fn toggle_copy_on_select(&mut self, cx: &mut Context<Self>) {
        if self.state.as_mut().is_some_and(|state| {
            let enabled = !state.settings().copy_on_select;
            state.set_copy_on_select(enabled).is_err()
        }) {
            self.status_message = Some(MessageId::StorageCorrupt);
        }
        cx.notify();
    }

    fn on_paste(&mut self, _: &Paste, window: &mut Window, cx: &mut Context<Self>) {
        if self.focus_handle.is_focused(window) {
            self.paste_terminal(cx);
            cx.stop_propagation();
        }
    }
}

impl Render for AppView {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        // The dialog layer must never be mutated while AppView is on the
        // render stack; sync it on the next deferred pass.
        cx.defer_in(window, |this, window, cx| {
            this.sync_dialog_layer(window, cx)
        });
        self.refresh_preferences(window, cx);
        self.sync_status(cx);
        let sidebar_visible = self.main_view == MainView::Workspace
            || (self.main_view == MainView::Sessions && self.tabs.tabs().is_empty());
        let body = if self.state.is_none() {
            self.render_recovery(cx)
        } else {
            let main = match self.main_view {
                MainView::Workspace => self.render_workspace(cx),
                MainView::Sessions if self.tabs.tabs().is_empty() => self.render_workspace(cx),
                MainView::Sessions => self.render_sessions(cx),
                MainView::Settings => self.render_settings(cx),
            };
            div()
                .size_full()
                .flex()
                .flex_col()
                .child(self.render_tab_strip(window, cx))
                .child(
                    div()
                        .flex_1()
                        .flex()
                        .gap(px(CARD_GAP))
                        .p(px(CARD_GAP))
                        .when(sidebar_visible, |element| {
                            element.child(self.render_sidebar(cx))
                        })
                        .child(div().flex_1().h_full().child(main)),
                )
                .into_any_element()
        };
        let status = self.status_message.map(|message| {
            div()
                .absolute()
                .left(px(if sidebar_visible { 216. } else { 16. }))
                .bottom(px(14.))
                .max_w(px(620.))
                .child(
                    Alert::error("status-alert", self.text(message))
                        .on_close(cx.listener(|this, _, _, cx| this.dismiss_status(cx))),
                )
        });
        let bell = self.bell.then(|| {
            div()
                .absolute()
                .right(px(16.))
                .bottom(px(14.))
                .child(Alert::info("bell-alert", self.text(MessageId::Bell)))
        });
        div()
            .id("oxide-ssh-root")
            .size_full()
            .relative()
            .key_context("OxideSSH")
            .track_focus(&self.focus_handle)
            .bg(cx.theme().background)
            .text_color(cx.theme().foreground)
            .font_family(".SystemUIFont")
            .on_action(cx.listener(Self::on_add_connection))
            .on_action(cx.listener(Self::on_local_terminal))
            .on_action(cx.listener(Self::on_open_settings))
            .on_action(cx.listener(Self::on_next_tab))
            .on_action(cx.listener(Self::on_previous_tab))
            .on_action(cx.listener(Self::on_close_tab))
            .on_action(cx.listener(Self::on_quit_app))
            .on_action(cx.listener(Self::on_copy))
            .on_action(cx.listener(Self::on_paste))
            .on_action(cx.listener(Self::on_terminal_tab))
            .on_action(cx.listener(Self::on_terminal_shift_tab))
            .child(body)
            .children(status)
            .children(bell)
            .children(Root::render_dialog_layer(window, cx))
    }
}

impl Focusable for AppView {
    fn focus_handle(&self, _: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

impl Drop for AppView {
    fn drop(&mut self) {
        self.tabs.disconnect_all();
    }
}

const STATUS_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(8);

fn play_terminal_bell() {
    #[cfg(target_os = "windows")]
    {
        #[link(name = "user32")]
        unsafe extern "system" {
            fn MessageBeep(kind: u32) -> i32;
        }
        // Windows' default system notification sound.
        unsafe {
            MessageBeep(0xFFFFFFFF);
        }
    }
    #[cfg(target_os = "macos")]
    {
        let _ = std::process::Command::new("afplay")
            .arg("/System/Library/Sounds/Glass.aiff")
            .spawn();
    }
    #[cfg(target_os = "linux")]
    {
        if std::process::Command::new("canberra-gtk-play")
            .args(["-i", "bell"])
            .spawn()
            .is_err()
        {
            let _ = std::process::Command::new("paplay")
                .arg("/usr/share/sounds/freedesktop/stereo/bell.oga")
                .spawn();
        }
    }
}
