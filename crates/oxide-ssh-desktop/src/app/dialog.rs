use std::cell::Cell;

use super::discovery::{
    DiscoveryDialog, DiscoveryKey, LocalRenameDialog, discovered_username, skip_reason_text,
};
use super::editor::{EditorId, EditorSaveState, editor_save_state};
use super::tags::TagManagerEntry;
use super::*;

#[derive(Clone, Debug, Eq, PartialEq)]
pub(super) enum ConfirmAction {
    ImportConfig(PathBuf),
    ImportKnownHostsReplace(PathBuf),
    DeleteProfile(ProfileId),
    DeleteHost(Endpoint),
    DeleteTag(TagId),
    DeleteLocalConnection(LocalConnectionId),
    ClearAllCredentials,
    Paste {
        tab_id: TabId,
        text: String,
    },
    /// Window close: disconnects, then removes only this window.
    Osc52Clipboard {
        tab_id: TabId,
        session_id: SessionId,
        text: String,
    },
    Quit,
    /// Cmd+Q / dock Quit: disconnects, then terminates the process.
    QuitApp,
}

impl ConfirmAction {
    fn accept_variant(&self) -> ButtonVariant {
        match self {
            Self::Paste { .. } | Self::Osc52Clipboard { .. } => ButtonVariant::Primary,
            _ => ButtonVariant::Danger,
        }
    }

    fn cancel_on_submit(&self) -> bool {
        self.accept_variant() == ButtonVariant::Danger
    }
}

#[derive(Clone, Debug, PartialEq)]
struct DialogIdentity {
    locale: ResolvedLocale,
    kind: DialogIdentityKind,
}

#[derive(Clone, Debug, PartialEq)]
enum DialogIdentityKind {
    Editor {
        editor_id: EditorId,
        auth_method: AuthMethod,
        remember: bool,
        save_state: EditorSaveState,
        is_editing: bool,
    },
    Confirm(ConfirmAction),
    Secret(TabId),
    HostKey {
        tab_id: TabId,
        prompt_id: Uuid,
    },
    ChangedHostKey {
        tab_id: TabId,
        request_id: Uuid,
    },
    ConfirmClose(TabId),
    TagManager,
    AnsiPalette,
    Discovery,
    LocalRename {
        id: LocalConnectionId,
    },
}

/// Stable logical identity used to decide whether a deferred focus pass is
/// still current; excludes locale and editor view-state.
#[derive(Clone, Debug, Eq, PartialEq)]
enum DialogFocusIdentity {
    Editor(EditorId),
    Confirm(ConfirmAction),
    Secret(TabId),
    HostKey { tab_id: TabId, prompt_id: Uuid },
    ChangedHostKey { tab_id: TabId, request_id: Uuid },
    ConfirmClose(TabId),
    TagManager,
    AnsiPalette,
    Discovery,
    LocalRename(LocalConnectionId),
}

impl DialogIdentityKind {
    fn focus_identity(&self) -> DialogFocusIdentity {
        match self {
            Self::Editor { editor_id, .. } => DialogFocusIdentity::Editor(*editor_id),
            Self::Confirm(action) => DialogFocusIdentity::Confirm(action.clone()),
            Self::Secret(tab_id) => DialogFocusIdentity::Secret(*tab_id),
            Self::HostKey { tab_id, prompt_id } => DialogFocusIdentity::HostKey {
                tab_id: *tab_id,
                prompt_id: *prompt_id,
            },
            Self::ChangedHostKey { tab_id, request_id } => DialogFocusIdentity::ChangedHostKey {
                tab_id: *tab_id,
                request_id: *request_id,
            },
            Self::ConfirmClose(tab_id) => DialogFocusIdentity::ConfirmClose(*tab_id),
            Self::TagManager => DialogFocusIdentity::TagManager,
            Self::AnsiPalette => DialogFocusIdentity::AnsiPalette,
            Self::Discovery => DialogFocusIdentity::Discovery,
            Self::LocalRename { id } => DialogFocusIdentity::LocalRename(*id),
        }
    }
}

#[derive(Clone)]
enum DialogSnapshot {
    Editor(Box<EditorDialogSnapshot>),
    Confirm {
        action: ConfirmAction,
        /// Profiles referencing a tag being deleted; shown in the body.
        affected: usize,
    },
    TagManager {
        entries: Vec<TagManagerEntry>,
        /// The manager's inline "create tag" field.
        new_tag: Entity<InputState>,
    },
    AnsiPalette {
        palettes: Vec<AnsiPalette>,
        selected: usize,
    },
    Discovery(DiscoveryDialog),
    LocalRename(LocalRenameDialog),
    Modal {
        request: ModalRequest,
        /// The shared one-time credential input; rendered by the presenter for
        /// Secret requests. Carried in the payload because the AppView is
        /// leased while the dialog layer renders and cannot be read then.
        secret_input: Entity<InputState>,
    },
}

#[derive(Clone)]
struct EditorDialogSnapshot {
    editor_id: EditorId,
    is_editing: bool,
    auth_method: AuthMethod,
    remember: bool,
    advanced_open: bool,
    error: Option<FormError>,
    save_state: EditorSaveState,
    name: Entity<InputState>,
    host: Entity<InputState>,
    port: Entity<InputState>,
    username: Entity<InputState>,
    private_key_path: Entity<InputState>,
    startup_command: Entity<InputState>,
    ssh_keepalive_interval: Entity<InputState>,
    ssh_keepalive_max_count: Entity<InputState>,
    terminal_font_size: Entity<InputState>,
    ansi_palette: Entity<InputState>,
    environment_variables: Entity<InputState>,
    secret: Entity<InputState>,
    /// Registry tags available for assignment, name-sorted.
    tags: Vec<Tag>,
    tag_ids: Vec<TagId>,
    tag_name: Entity<InputState>,
}

struct EditorDialogLayout {
    rows: Vec<gpui::AnyElement>,
    footer: gpui::AnyElement,
}

struct DialogPayload {
    identity: DialogIdentity,
    snapshot: DialogSnapshot,
}

struct DialogFocusBoundary {
    scope: FocusHandle,
    start: FocusHandle,
    end: FocusHandle,
}

impl DialogFocusBoundary {
    fn new(cx: &mut Context<DialogPresenter>) -> Self {
        Self {
            scope: cx.focus_handle(),
            start: cx.focus_handle(),
            end: cx.focus_handle(),
        }
    }

    fn contains_focused(&self, window: &Window, cx: &App) -> bool {
        self.scope.contains_focused(window, cx)
    }
}

pub(super) struct DialogPresenter {
    owner: WeakEntity<AppView>,
    payload: DialogPayload,
    focus: DialogFocusBoundary,
    editor_scroll: gpui::ScrollHandle,
    pending_error_scroll: Cell<bool>,
}

impl DialogPresenter {
    fn new(owner: WeakEntity<AppView>, payload: DialogPayload, cx: &mut Context<Self>) -> Self {
        Self {
            owner,
            payload,
            focus: DialogFocusBoundary::new(cx),
            editor_scroll: gpui::ScrollHandle::new(),
            pending_error_scroll: Cell::new(false),
        }
    }

    fn text(&self, id: MessageId) -> &'static str {
        Catalog::text(self.payload.identity.locale, id)
    }

    fn dialog_width(&self) -> Pixels {
        match &self.payload.snapshot {
            DialogSnapshot::Editor(_) => px(560.),
            DialogSnapshot::Discovery(_) => px(640.),
            DialogSnapshot::LocalRename(_) => px(420.),
            _ => px(520.),
        }
    }

    fn focus_initial(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        match &self.payload.snapshot {
            DialogSnapshot::AnsiPalette { .. } => {
                self.focus.scope.focus(window);
                return;
            }
            DialogSnapshot::Editor(editor) => {
                editor.name.update(cx, |input, cx| input.focus(window, cx));
                return;
            }
            DialogSnapshot::Modal {
                request: ModalRequest::Secret { .. },
                secret_input,
            } => {
                secret_input.update(cx, |input, cx| {
                    input.set_value("", window, cx);
                    input.focus(window, cx);
                });
                return;
            }
            DialogSnapshot::TagManager { entries, new_tag } => {
                match entries.first() {
                    Some(entry) => entry.name.update(cx, |input, cx| input.focus(window, cx)),
                    None => new_tag.update(cx, |input, cx| input.focus(window, cx)),
                }
                return;
            }
            DialogSnapshot::LocalRename(dialog) => {
                dialog.name.update(cx, |input, cx| input.focus(window, cx));
                return;
            }
            _ => {}
        }
        // Non-input dialogs: land on the first/last real component by
        // focusing the boundary anchor and advancing once.
        self.focus.start.focus(window);
        window.focus_next();
    }

    fn on_focus_next(&mut self, window: &mut Window, cx: &mut App) {
        window.focus_next();
        if !self.focus.contains_focused(window, cx) {
            self.focus.start.focus(window);
            window.focus_next();
        }
        cx.stop_propagation();
    }

    fn on_focus_prev(&mut self, window: &mut Window, cx: &mut App) {
        window.focus_prev();
        if !self.focus.contains_focused(window, cx) {
            self.focus.end.focus(window);
            window.focus_prev();
        }
        cx.stop_propagation();
    }

    fn save_editor(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let result = self
            .owner
            .update(cx, |app, cx| {
                app.save_editor(window, cx);
                app.editor
                    .as_ref()
                    .map(|editor| (editor.error, editor.advanced_open))
            })
            .ok()
            .flatten();
        let Some((error, advanced_open)) = result else {
            return;
        };
        if let DialogSnapshot::Editor(editor) = &mut self.payload.snapshot {
            editor.error = error;
            editor.advanced_open = advanced_open;
            let input = match error {
                Some(FormError::Name) => Some(&editor.name),
                Some(FormError::Host) => Some(&editor.host),
                Some(FormError::Port) => Some(&editor.port),
                Some(FormError::Username) => Some(&editor.username),
                Some(FormError::PrivateKeyPath) => Some(&editor.private_key_path),
                Some(FormError::Keepalive) => Some(&editor.ssh_keepalive_interval),
                Some(FormError::KeepaliveMaxCount) => Some(&editor.ssh_keepalive_max_count),
                Some(FormError::TerminalFontSize) => Some(&editor.terminal_font_size),
                Some(FormError::AnsiPalette) => Some(&editor.ansi_palette),
                Some(FormError::EnvironmentVariable) => Some(&editor.environment_variables),
                None => None,
            };
            if let Some(input) = input {
                self.pending_error_scroll.set(true);
                input.update(cx, |input, cx| input.focus(window, cx));
            }
        }
        cx.notify();
    }

    /// Enter triggers the dialog's primary action; ignored while the action
    /// is in a disabled/in-progress state (e.g. an editor save in flight).
    fn on_submit(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        match &self.payload.snapshot {
            DialogSnapshot::Editor(editor) => {
                if editor.save_state != EditorSaveState::Idle {
                    return;
                }
                self.save_editor(window, cx);
            }
            DialogSnapshot::Confirm { action, .. } => {
                self.owner
                    .update(cx, |app, cx| {
                        if action.cancel_on_submit() {
                            app.cancel_confirm(cx);
                        } else {
                            app.perform_confirm(window, cx);
                        }
                    })
                    .ok();
            }
            DialogSnapshot::TagManager { .. } => {
                self.owner
                    .update(cx, |app, cx| app.commit_tag_names(window, cx))
                    .ok();
            }
            DialogSnapshot::AnsiPalette { .. } => {
                self.owner
                    .update(cx, |app, cx| app.confirm_ansi_palette_selection(cx))
                    .ok();
            }
            DialogSnapshot::Discovery(dialog) => {
                if !dialog.scanning {
                    self.owner
                        .update(cx, |app, cx| app.commit_discovery_import(cx))
                        .ok();
                }
            }
            DialogSnapshot::LocalRename(_) => {
                self.owner
                    .update(cx, |app, cx| app.commit_local_rename(window, cx))
                    .ok();
            }
            DialogSnapshot::Modal { request, .. } => match request {
                ModalRequest::Secret { tab_id } => {
                    let tab_id = *tab_id;
                    self.owner
                        .update(cx, |app, cx| app.submit_secret(tab_id, window, cx))
                        .ok();
                }
                ModalRequest::HostKey {
                    tab_id, prompt_id, ..
                } => {
                    let (tab_id, prompt_id) = (*tab_id, *prompt_id);
                    self.owner
                        .update(cx, |app, cx| {
                            app.decide_host_key(
                                tab_id,
                                prompt_id,
                                HostKeyDecision::AcceptAndStore,
                                cx,
                            )
                        })
                        .ok();
                }
                ModalRequest::ChangedHostKey {
                    tab_id, request_id, ..
                } => {
                    let (tab_id, request_id) = (*tab_id, *request_id);
                    self.owner
                        .update(cx, |app, cx| app.open_trusted_hosts(tab_id, request_id, cx))
                        .ok();
                }
                ModalRequest::ConfirmClose { tab_id } => {
                    let tab_id = *tab_id;
                    self.owner
                        .update(cx, |app, cx| app.cancel_tab_close(tab_id, cx))
                        .ok();
                }
            },
        }
    }

    /// Esc dismisses the dialog through the same path as its cancel/close
    /// button; ignored while an editor save is in flight.
    fn on_cancel(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        match &self.payload.snapshot {
            DialogSnapshot::Editor(editor) => {
                if editor.save_state != EditorSaveState::Idle {
                    return;
                }
                self.owner.update(cx, |app, cx| app.cancel_editor(cx)).ok();
            }
            DialogSnapshot::Confirm { .. } => {
                self.owner.update(cx, |app, cx| app.cancel_confirm(cx)).ok();
            }
            DialogSnapshot::TagManager { .. } => {
                self.owner
                    .update(cx, |app, cx| app.close_tag_manager(window, cx))
                    .ok();
            }
            DialogSnapshot::AnsiPalette { .. } => {
                self.owner
                    .update(cx, |app, cx| app.close_ansi_palette_dialog(cx))
                    .ok();
            }
            DialogSnapshot::Discovery(_) => {
                self.owner
                    .update(cx, |app, cx| app.close_discovery(cx))
                    .ok();
            }
            DialogSnapshot::LocalRename(_) => {
                self.owner
                    .update(cx, |app, cx| app.cancel_local_rename(cx))
                    .ok();
            }
            DialogSnapshot::Modal { request, .. } => match request {
                ModalRequest::Secret { tab_id } => {
                    let tab_id = *tab_id;
                    self.owner
                        .update(cx, |app, cx| app.cancel_secret(tab_id, window, cx))
                        .ok();
                }
                ModalRequest::HostKey {
                    tab_id, prompt_id, ..
                } => {
                    let (tab_id, prompt_id) = (*tab_id, *prompt_id);
                    self.owner
                        .update(cx, |app, cx| {
                            app.decide_host_key(tab_id, prompt_id, HostKeyDecision::Reject, cx)
                        })
                        .ok();
                }
                ModalRequest::ChangedHostKey {
                    tab_id, request_id, ..
                } => {
                    let (tab_id, request_id) = (*tab_id, *request_id);
                    self.owner
                        .update(cx, |app, cx| {
                            app.close_changed_host_key(tab_id, request_id, cx)
                        })
                        .ok();
                }
                ModalRequest::ConfirmClose { tab_id } => {
                    let tab_id = *tab_id;
                    self.owner
                        .update(cx, |app, cx| app.cancel_tab_close(tab_id, cx))
                        .ok();
                }
            },
        }
    }

    fn render_editor(
        &self,
        editor: &EditorDialogSnapshot,
        cx: &mut Context<Self>,
    ) -> EditorDialogLayout {
        let text = |id: MessageId| self.text(id).to_owned();
        let error =
            |target| (editor.error == Some(target)).then(|| text(form_error_message_id(target)));
        let name_label = text(MessageId::Name);
        let host_label = text(MessageId::Host);
        let port_label = text(MessageId::Port);
        let username_label = text(MessageId::Username);
        let terminal_font_size_label = text(MessageId::TerminalFontSize);
        let keepalive_interval_label = text(MessageId::ProfileKeepaliveInterval);
        let keepalive_max_count_label = text(MessageId::ProfileKeepaliveMaxCount);
        let auth_label = text(MessageId::AuthMethod);
        let private_key_label = text(MessageId::PrivateKey);
        let password_label = text(MessageId::Password);
        let passphrase_label = text(MessageId::Passphrase);
        let remember_label = text(MessageId::Remember);
        let agent_description = text(MessageId::AgentDescription);
        let tags_label = text(MessageId::Tags);
        let create_label = text(MessageId::Create);
        let tag_border = cx.theme().border;
        let tag_secondary = cx.theme().secondary;
        let tag_selected_bg = cx.theme().primary;
        let auth_index = match editor.auth_method {
            AuthMethod::Password => 0,
            AuthMethod::PrivateKey => 1,
            AuthMethod::Agent => 2,
        };
        let secret_label = if editor.auth_method == AuthMethod::PrivateKey {
            passphrase_label
        } else {
            password_label.clone()
        };
        let heading = if editor.is_editing {
            text(MessageId::Edit)
        } else {
            text(MessageId::AddConnection)
        };
        debug_assert!(matches!(
            &self.payload.identity.kind,
            DialogIdentityKind::Editor { editor_id, .. } if *editor_id == editor.editor_id
        ));
        let secret_is_visible = editor.auth_method != AuthMethod::Agent;
        let cancel_listener = cx.listener(|this, _, _, cx| {
            this.owner.update(cx, |app, cx| app.cancel_editor(cx)).ok();
        });
        let save_listener = cx.listener(|this, _, window, cx| this.save_editor(window, cx));
        let browse_listener = cx.listener(|this, _, window, cx| {
            this.owner
                .update(cx, |app, cx| app.browse_private_key(window, cx))
                .ok();
        });
        let advanced_listener = cx.listener(|this, _, _, cx| {
            let _ = this
                .owner
                .update(cx, |app, cx| app.toggle_editor_advanced(cx));
            if let DialogSnapshot::Editor(editor) = &mut this.payload.snapshot {
                editor.advanced_open = !editor.advanced_open;
            }
            cx.notify();
        });
        let auth_listener = cx.listener(|this, index, window, cx| {
            let method = match *index {
                1 => AuthMethod::PrivateKey,
                2 => AuthMethod::Agent,
                _ => AuthMethod::Password,
            };
            let _ = this
                .owner
                .update(cx, |app, cx| app.select_auth_method(method, window, cx));
            if let DialogSnapshot::Editor(editor) = &mut this.payload.snapshot {
                editor.auth_method = method;
                let locale = this.payload.identity.locale;
                let placeholder = if method == AuthMethod::PrivateKey {
                    MessageId::Passphrase
                } else {
                    MessageId::Password
                };
                editor.secret.update(cx, |input, cx| {
                    input.set_value("", window, cx);
                    input.set_placeholder(Catalog::text(locale, placeholder), window, cx);
                });
            }
            cx.notify();
        });
        let remember_listener = cx.listener(|this, checked, _, cx| {
            let _ = this.owner.update(cx, |app, cx| app.toggle_remember(cx));
            if let DialogSnapshot::Editor(editor) = &mut this.payload.snapshot {
                editor.remember = *checked;
            }
            cx.notify();
        });
        let browse_button = button(
            "browse-private-key",
            text(MessageId::Browse),
            ButtonVariant::Secondary,
        )
        .on_click(browse_listener);
        let mut tag_chips = h_flex().gap(px(6.)).flex_wrap();
        for tag in &editor.tags {
            let selected = editor.tag_ids.contains(&tag.id);
            let id = tag.id;
            let toggle_listener = cx.listener(move |this, _, _, cx| {
                let _ = this
                    .owner
                    .update(cx, |app, cx| app.toggle_editor_tag(id, cx));
                if let DialogSnapshot::Editor(editor) = &mut this.payload.snapshot {
                    match editor.tag_ids.iter().position(|tag_id| *tag_id == id) {
                        Some(index) => {
                            editor.tag_ids.remove(index);
                        }
                        None => editor.tag_ids.push(id),
                    }
                }
                cx.notify();
            });
            let chip = TextRole::Caption.apply(
                h_flex()
                    .id(SharedString::from(format!("editor-tag-{}", tag.id.0)))
                    .items_center()
                    .gap(px(6.))
                    .px(px(8.))
                    .py(px(3.))
                    .rounded_full()
                    .border_1()
                    .cursor_pointer()
                    .child(
                        div()
                            .size(px(8.))
                            .flex_none()
                            .rounded_full()
                            .bg(rgb(tag_color_rgb(tag.color.unwrap_or(TagColor::Gray)))),
                    )
                    .child(tag.name.clone())
                    .when(selected, |chip| {
                        chip.child(Icon::new(IconName::Check).size(px(12.)).flex_none())
                            .child(self.text(MessageId::Selected))
                    })
                    .on_click(toggle_listener),
            );
            tag_chips = tag_chips.child(if selected {
                TextColor::OnAccent
                    .apply(chip.bg(tag_selected_bg).border_color(tag_selected_bg), cx)
            } else {
                chip.bg(tag_secondary).border_color(tag_border)
            });
        }
        let create_listener = cx.listener(|this, _, window, cx| {
            let created = this
                .owner
                .update(cx, |app, cx| app.create_tag_from_editor(window, cx))
                .ok()
                .flatten();
            if let Some(tag) = created
                && let DialogSnapshot::Editor(editor) = &mut this.payload.snapshot
            {
                if !editor.tag_ids.contains(&tag.id) {
                    editor.tag_ids.push(tag.id);
                }
                editor.tags.push(tag);
                editor.tags.sort_by(|left, right| {
                    left.name.to_lowercase().cmp(&right.name.to_lowercase())
                });
            }
            cx.notify();
        });

        let mut rows = Vec::new();
        let mut error_row = None;
        let mut push_row = |element: gpui::AnyElement, target: Option<FormError>| {
            if target.is_some() && editor.error == target {
                error_row = Some(rows.len());
            }
            rows.push(element);
        };
        push_row(
            ui_text(TextRole::DialogTitle, TextColor::Primary, heading, cx).into_any_element(),
            None,
        );
        push_row(
            field()
                .label(name_label)
                .child(Input::new(&editor.name))
                .when_some(error(FormError::Name), |field, description| {
                    field.description_fn(move |_, cx| {
                        ui_text(TextRole::Helper, TextColor::Danger, description.clone(), cx)
                    })
                })
                .into_any_element(),
            Some(FormError::Name),
        );
        let endpoint_error = match editor.error {
            Some(FormError::Host) => Some(FormError::Host),
            Some(FormError::Port) => Some(FormError::Port),
            _ => None,
        };
        push_row(
            h_flex()
                .gap(px(8.))
                .child(
                    field()
                        .label(host_label)
                        .child(Input::new(&editor.host))
                        .when_some(error(FormError::Host), |field, description| {
                            field.description_fn(move |_, cx| {
                                ui_text(
                                    TextRole::Helper,
                                    TextColor::Danger,
                                    description.clone(),
                                    cx,
                                )
                            })
                        }),
                )
                .child(
                    div().w(px(104.)).flex_none().child(
                        field()
                            .label(port_label)
                            .child(Input::new(&editor.port))
                            .when_some(error(FormError::Port), |field, description| {
                                field.description_fn(move |_, cx| {
                                    ui_text(
                                        TextRole::Helper,
                                        TextColor::Danger,
                                        description.clone(),
                                        cx,
                                    )
                                })
                            }),
                    ),
                )
                .into_any_element(),
            endpoint_error,
        );
        push_row(
            field()
                .label(username_label)
                .child(Input::new(&editor.username))
                .when_some(error(FormError::Username), |field, description| {
                    field.description_fn(move |_, cx| {
                        ui_text(TextRole::Helper, TextColor::Danger, description.clone(), cx)
                    })
                })
                .into_any_element(),
            Some(FormError::Username),
        );
        push_row(
            field()
                .label(auth_label)
                .child(
                    RadioGroup::horizontal("editor-auth")
                        .selected_index(Some(auth_index))
                        .child(Radio::new("auth-password").label(password_label))
                        .child(Radio::new("auth-private-key").label(private_key_label.clone()))
                        .child(Radio::new("auth-agent").label(text(MessageId::Agent)))
                        .on_click(auth_listener),
                )
                .into_any_element(),
            None,
        );
        if editor.auth_method == AuthMethod::PrivateKey {
            push_row(
                field()
                    .label(private_key_label)
                    .child(
                        h_flex()
                            .gap(px(8.))
                            .child(Input::new(&editor.private_key_path))
                            .child(browse_button),
                    )
                    .when_some(error(FormError::PrivateKeyPath), |field, description| {
                        field.description_fn(move |_, cx| {
                            ui_text(TextRole::Helper, TextColor::Danger, description.clone(), cx)
                        })
                    })
                    .into_any_element(),
                Some(FormError::PrivateKeyPath),
            );
        }
        if secret_is_visible {
            push_row(
                field()
                    .label(secret_label)
                    .child(Input::new(&editor.secret))
                    .into_any_element(),
                None,
            );
            push_row(
                field()
                    .label_indent(false)
                    .child(checkbox_focus_ring(
                        "editor-remember",
                        Checkbox::new("editor-remember")
                            .label(remember_label)
                            .checked(editor.remember)
                            .on_click(remember_listener),
                    ))
                    .into_any_element(),
                None,
            );
        }
        if editor.auth_method == AuthMethod::Agent {
            push_row(helper_text(agent_description, cx).into_any_element(), None);
        }
        push_row(
            custom_button("toggle-editor-advanced", ButtonVariant::Tertiary)
                .w_full()
                .child(
                    h_flex()
                        .w_full()
                        .items_center()
                        .justify_between()
                        .child(
                            TextRole::Button
                                .apply(div())
                                .child(text(MessageId::AdvancedOptions)),
                        )
                        .child(
                            Icon::new(if editor.advanced_open {
                                IconName::ChevronDown
                            } else {
                                IconName::ChevronRight
                            })
                            .size(px(16.)),
                        ),
                )
                .on_click(advanced_listener)
                .into_any_element(),
            None,
        );
        if editor.advanced_open {
            push_row(
                ui_text(
                    TextRole::SectionTitle,
                    TextColor::Primary,
                    text(MessageId::ConnectionSettings),
                    cx,
                )
                .pt(px(4.))
                .into_any_element(),
                None,
            );
            push_row(
                field()
                    .label(text(MessageId::StartupCommand))
                    .child(Input::new(&editor.startup_command))
                    .into_any_element(),
                None,
            );
            push_row(
                field()
                    .label(keepalive_interval_label)
                    .child(Input::new(&editor.ssh_keepalive_interval))
                    .when_some(error(FormError::Keepalive), |field, description| {
                        field.description_fn(move |_, cx| {
                            ui_text(TextRole::Helper, TextColor::Danger, description.clone(), cx)
                        })
                    })
                    .into_any_element(),
                Some(FormError::Keepalive),
            );
            push_row(
                field()
                    .label(keepalive_max_count_label)
                    .child(Input::new(&editor.ssh_keepalive_max_count))
                    .when_some(error(FormError::KeepaliveMaxCount), |field, description| {
                        field.description_fn(move |_, cx| {
                            ui_text(TextRole::Helper, TextColor::Danger, description.clone(), cx)
                        })
                    })
                    .into_any_element(),
                Some(FormError::KeepaliveMaxCount),
            );
            push_row(
                field()
                    .label(text(MessageId::EnvironmentVariables))
                    .child(Input::new(&editor.environment_variables).h(px(110.)))
                    .when_some(
                        error(FormError::EnvironmentVariable),
                        |field, description| {
                            field.description_fn(move |_, cx| {
                                ui_text(
                                    TextRole::Helper,
                                    TextColor::Danger,
                                    description.clone(),
                                    cx,
                                )
                            })
                        },
                    )
                    .into_any_element(),
                Some(FormError::EnvironmentVariable),
            );
            push_row(
                ui_text(
                    TextRole::SectionTitle,
                    TextColor::Primary,
                    text(MessageId::TerminalSettings),
                    cx,
                )
                .pt(px(4.))
                .into_any_element(),
                None,
            );
            push_row(
                field()
                    .label(terminal_font_size_label)
                    .child(Input::new(&editor.terminal_font_size))
                    .when_some(error(FormError::TerminalFontSize), |field, description| {
                        field.description_fn(move |_, cx| {
                            ui_text(TextRole::Helper, TextColor::Danger, description.clone(), cx)
                        })
                    })
                    .into_any_element(),
                Some(FormError::TerminalFontSize),
            );
            push_row(
                field()
                    .label(text(MessageId::AnsiPalette))
                    .child(Input::new(&editor.ansi_palette))
                    .when_some(error(FormError::AnsiPalette), |field, description| {
                        field.description_fn(move |_, cx| {
                            ui_text(TextRole::Helper, TextColor::Danger, description.clone(), cx)
                        })
                    })
                    .into_any_element(),
                Some(FormError::AnsiPalette),
            );
            let mut tags_content = div().flex().flex_col().gap(px(8.));
            if !editor.tags.is_empty() {
                tags_content = tags_content.child(tag_chips);
            }
            tags_content = tags_content.child(
                h_flex()
                    .gap(px(8.))
                    .child(div().flex_1().child(Input::new(&editor.tag_name)))
                    .child(
                        button("create-tag", create_label, ButtonVariant::Secondary)
                            .button_size(ButtonSize::Compact)
                            .on_click(create_listener),
                    ),
            );
            push_row(
                ui_text(TextRole::SectionTitle, TextColor::Primary, tags_label, cx)
                    .pt(px(4.))
                    .into_any_element(),
                None,
            );
            push_row(tags_content.into_any_element(), None);
        }
        if self.pending_error_scroll.replace(false)
            && let Some(row) = error_row
        {
            self.editor_scroll.scroll_to_item(row);
        }
        let footer = div()
            .flex()
            .justify_end()
            .gap(px(8.))
            .pt(px(12.))
            .mt(px(4.))
            .border_t_1()
            .border_color(cx.theme().border)
            .child(
                button(
                    "cancel-editor",
                    self.text(MessageId::Cancel),
                    ButtonVariant::Secondary,
                )
                .on_click(cancel_listener),
            )
            .child(
                button(
                    "save-editor",
                    self.text(MessageId::Save),
                    ButtonVariant::Primary,
                )
                .loading(editor.save_state == EditorSaveState::SavingThis)
                .disabled(editor.save_state != EditorSaveState::Idle)
                .on_click(save_listener),
            )
            .into_any_element();
        EditorDialogLayout { rows, footer }
    }

    fn render_confirm(
        &self,
        action: &ConfirmAction,
        affected: usize,
        cx: &mut Context<Self>,
    ) -> gpui::AnyElement {
        let (message, action_label) = match action {
            ConfirmAction::ImportConfig(_) => {
                (MessageId::ConfirmConfigImport, MessageId::ConfigImport)
            }
            ConfirmAction::ImportKnownHostsReplace(_) => (
                MessageId::ConfirmKnownHostsReplace,
                MessageId::ImportReplaceExisting,
            ),
            ConfirmAction::DeleteProfile(_) => (MessageId::ConfirmDeleteProfile, MessageId::Delete),
            ConfirmAction::DeleteHost(_) => (MessageId::ConfirmDeleteHost, MessageId::Delete),
            ConfirmAction::DeleteTag(_) => (MessageId::ConfirmDeleteTag, MessageId::Delete),
            ConfirmAction::DeleteLocalConnection(_) => {
                (MessageId::ConfirmDeleteLocalConnection, MessageId::Delete)
            }
            ConfirmAction::ClearAllCredentials => (
                MessageId::ConfirmClearAllCredentials,
                MessageId::ClearAllCredentials,
            ),
            ConfirmAction::Paste { .. } => (MessageId::ConfirmLargePaste, MessageId::Paste),
            ConfirmAction::Osc52Clipboard { .. } => {
                (MessageId::Osc52ClipboardConfirm, MessageId::Allow)
            }
            ConfirmAction::Quit | ConfirmAction::QuitApp => {
                (MessageId::ConfirmQuit, MessageId::Quit)
            }
        };
        let cancel_listener = cx.listener(|this, _, _, cx| {
            this.owner.update(cx, |app, cx| app.cancel_confirm(cx)).ok();
        });
        let accept_listener = cx.listener(move |this, _, window, cx| {
            this.owner
                .update(cx, |app, cx| app.perform_confirm(window, cx))
                .ok();
        });
        let mut body = ui_text(TextRole::Body, TextColor::Primary, self.text(message), cx).w_full();
        if matches!(action, ConfirmAction::DeleteTag(_)) {
            body = body.child(ui_text(
                TextRole::Caption,
                TextColor::Secondary,
                format!("{affected}"),
                cx,
            ));
        }
        div()
            .flex()
            .flex_col()
            .gap(px(14.))
            .child(body)
            .child(
                div()
                    .flex()
                    .justify_end()
                    .gap(px(8.))
                    .child(
                        button(
                            "cancel-confirm",
                            self.text(MessageId::Cancel),
                            ButtonVariant::Secondary,
                        )
                        .on_click(cancel_listener),
                    )
                    .child(
                        button(
                            "accept-confirm",
                            self.text(action_label),
                            action.accept_variant(),
                        )
                        .on_click(accept_listener),
                    ),
            )
            .into_any_element()
    }

    fn render_tag_manager(
        &self,
        entries: &[TagManagerEntry],
        new_tag: &Entity<InputState>,
        cx: &mut Context<Self>,
    ) -> gpui::AnyElement {
        let heading = self.text(MessageId::Tags);
        let close_label = self.text(MessageId::Close);
        let create_label = self.text(MessageId::Create);
        let delete_label = self.text(MessageId::Delete).to_owned();
        let row_border = cx.theme().border;
        let row_bg = cx.theme().secondary;
        let swatch_selected = cx.theme().primary;
        let swatch_rest = cx.theme().border;
        let none_bg = cx.theme().background;
        let close_listener = cx.listener(|this, _, window, cx| {
            this.owner
                .update(cx, |app, cx| app.close_tag_manager(window, cx))
                .ok();
        });
        let create_listener = cx.listener(|this, _, window, cx| {
            let created = this
                .owner
                .update(cx, |app, cx| app.create_tag_from_manager(window, cx))
                .ok()
                .flatten();
            if let Some(entry) = created
                && let DialogSnapshot::TagManager { entries, .. } = &mut this.payload.snapshot
            {
                entries.push(entry);
                entries.sort_by(|left, right| {
                    left.name
                        .read(cx)
                        .value()
                        .to_lowercase()
                        .cmp(&right.name.read(cx).value().to_lowercase())
                });
            }
        });
        let mut rows = div().flex().flex_col().gap(px(8.));
        for entry in entries {
            let id = entry.id;
            let expanded = entry.expanded;
            let none_selected = entry.color.is_none();
            let none_listener = cx.listener(move |this, _, window, cx| {
                this.set_manager_tag_color(id, None, window, cx);
            });
            let mut swatches = h_flex().gap(px(6.)).items_center().child(
                div()
                    .id(SharedString::from(format!("tag-color-none-{}", id.0)))
                    .size(px(16.))
                    .flex_none()
                    .rounded_full()
                    .bg(none_bg)
                    .border_2()
                    .border_color(if none_selected {
                        swatch_selected
                    } else {
                        swatch_rest
                    })
                    .cursor_pointer()
                    .when(none_selected, |swatch| {
                        swatch.child(
                            Icon::new(IconName::Check)
                                .size(px(12.))
                                .text_color(cx.theme().foreground),
                        )
                    })
                    .on_click(none_listener),
            );
            if none_selected {
                swatches = swatches.child(self.text(MessageId::Selected));
            }
            for color in TagColor::ALL {
                let selected = entry.color == Some(color);
                let listener = cx.listener(move |this, _, window, cx| {
                    this.set_manager_tag_color(id, Some(color), window, cx);
                });
                swatches = swatches.child(
                    div()
                        .id(SharedString::from(format!(
                            "tag-color-{}-{}",
                            color as u8, id.0
                        )))
                        .size(px(16.))
                        .flex_none()
                        .rounded_full()
                        .bg(rgb(tag_color_rgb(color)))
                        .border_2()
                        .border_color(if selected {
                            swatch_selected
                        } else {
                            swatch_rest
                        })
                        .cursor_pointer()
                        .when(selected, |swatch| {
                            swatch.child(
                                Icon::new(IconName::Check)
                                    .size(px(12.))
                                    .text_color(contrasting_foreground(rgb(tag_color_rgb(color)))),
                            )
                        })
                        .on_click(listener),
                );
                if selected {
                    swatches = swatches.child(self.text(MessageId::Selected));
                }
            }
            let delete_listener = cx.listener(move |this, _, _, cx| {
                this.owner
                    .update(cx, |app, cx| app.request_tag_delete(id, cx))
                    .ok();
            });
            let toggle_listener = cx.listener(move |this, _, _, cx| {
                this.owner
                    .update(cx, |app, cx| app.toggle_tag_expanded(id, cx))
                    .ok();
                if let DialogSnapshot::TagManager { entries, .. } = &mut this.payload.snapshot {
                    for entry in entries.iter_mut() {
                        entry.expanded = entry.id == id && !entry.expanded;
                    }
                }
            });
            let dot_bg = match entry.color {
                Some(color) => rgb(tag_color_rgb(color)),
                None => none_bg.into(),
            };
            rows = rows.child(
                div()
                    .flex()
                    .flex_col()
                    .gap(px(8.))
                    .p(px(10.))
                    .rounded(px(8.))
                    .border_1()
                    .border_color(row_border)
                    .bg(row_bg)
                    .child(
                        h_flex()
                            .gap(px(8.))
                            .items_center()
                            .child(
                                div()
                                    .id(SharedString::from(format!("tag-dot-{}", id.0)))
                                    .size(px(16.))
                                    .flex_none()
                                    .rounded_full()
                                    .bg(dot_bg)
                                    .border_2()
                                    .border_color(swatch_rest)
                                    .cursor_pointer()
                                    .on_click(toggle_listener),
                            )
                            .child(div().flex_1().child(Input::new(&entry.name)))
                            .child(
                                button(
                                    SharedString::from(format!("delete-tag-{}", id.0)),
                                    delete_label.clone(),
                                    ButtonVariant::Tertiary,
                                )
                                .icon(IconName::Delete)
                                .button_size(ButtonSize::Compact)
                                .on_click(delete_listener),
                            ),
                    )
                    .when(expanded, |row| row.child(swatches)),
            );
        }
        div()
            .flex()
            .flex_col()
            .gap(px(12.))
            .child(ui_text(
                TextRole::DialogTitle,
                TextColor::Primary,
                heading,
                cx,
            ))
            .child(
                h_flex()
                    .gap(px(8.))
                    .child(div().flex_1().child(Input::new(new_tag)))
                    .child(
                        button(
                            "create-tag-from-manager",
                            create_label,
                            ButtonVariant::Secondary,
                        )
                        .button_size(ButtonSize::Compact)
                        .on_click(create_listener),
                    ),
            )
            .child(rows)
            .child(
                div().flex().justify_end().child(
                    button("close-tag-manager", close_label, ButtonVariant::Secondary)
                        .on_click(close_listener),
                ),
            )
            .into_any_element()
    }

    /// Applies a swatch click: persists the color and refreshes the snapshot so
    /// the selected swatch updates before the next AppView render.
    fn set_manager_tag_color(
        &mut self,
        id: TagId,
        color: Option<TagColor>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.owner
            .update(cx, |app, cx| app.set_tag_color(id, color, window, cx))
            .ok();
        if let DialogSnapshot::TagManager { entries, .. } = &mut self.payload.snapshot
            && let Some(entry) = entries.iter_mut().find(|entry| entry.id == id)
        {
            entry.color = color;
        }
        cx.notify();
    }

    fn render_ansi_palette(
        &self,
        palettes: &[AnsiPalette],
        selected: usize,
        cx: &mut Context<Self>,
    ) -> gpui::AnyElement {
        let mut rows = div().flex().flex_col().gap(px(8.));
        for (index, palette) in palettes.iter().enumerate() {
            let name = palette.name.clone();
            let label = match palette.name.as_str() {
                BUILT_IN_PALETTE => self.text(MessageId::BuiltInAnsiPalette),
                CUSTOM_PALETTE => self.text(MessageId::CustomAnsiPalette),
                name => name,
            };
            let listener = cx.listener(move |this, _, _, cx| {
                this.owner
                    .update(cx, |app, cx| app.select_ansi_palette(name.clone(), cx))
                    .ok();
            });
            rows = rows.child(
                custom_button(
                    SharedString::from(format!("ansi-palette-{index}")),
                    ButtonVariant::Tertiary,
                )
                .w_full()
                .h(px(64.))
                .selected(index == selected)
                .child(
                    h_flex()
                        .items_center()
                        .gap(px(12.))
                        .child(ansi_palette_preview(&palette.colors, px(16.), cx))
                        .child(
                            TextRole::Button
                                .apply(div())
                                .child(SharedString::from(label.to_owned())),
                        ),
                )
                .on_click(listener),
            );
        }
        let cancel_listener = cx.listener(|this, _, window, cx| this.on_cancel(window, cx));
        div()
            .flex()
            .flex_col()
            .gap(px(16.))
            .child(ui_text(
                TextRole::DialogTitle,
                TextColor::Primary,
                self.text(MessageId::SelectAnsiPalette),
                cx,
            ))
            .child(rows)
            .child(
                div().flex().justify_end().child(
                    button(
                        "cancel-ansi-palette",
                        self.text(MessageId::Cancel),
                        ButtonVariant::Secondary,
                    )
                    .on_click(cancel_listener),
                ),
            )
            .into_any_element()
    }

    fn render_discovery(
        &self,
        dialog: &DiscoveryDialog,
        cx: &mut Context<Self>,
    ) -> gpui::AnyElement {
        let border = cx.theme().input;
        let row_bg = cx.theme().secondary;
        let scanning = dialog.scanning;
        let scan_listener = cx.listener(|this, _, _, cx| {
            this.owner
                .update(cx, |app, cx| app.start_discovery_scan(cx))
                .ok();
        });
        let scan_label = if scanning {
            self.text(MessageId::DiscoveryScanning)
        } else {
            #[cfg(target_os = "windows")]
            let has_results = (dialog.include_ssh && dialog.ssh.is_some())
                || (dialog.include_wsl && dialog.wsl.is_some());
            #[cfg(not(target_os = "windows"))]
            let has_results = dialog.include_ssh && dialog.ssh.is_some();
            if has_results {
                self.text(MessageId::DiscoveryRescan)
            } else {
                self.text(MessageId::DiscoveryScan)
            }
        };
        let mut scan_row = h_flex().items_center().gap(px(8.)).child(
            button("discovery-scan", scan_label, ButtonVariant::Secondary)
                .icon(IconName::Search)
                .loading(scanning)
                .disabled(scanning)
                .on_click(scan_listener),
        );
        let ssh_enabled = dialog.include_ssh;
        let ssh_listener = cx.listener(|this, checked, _, cx| {
            let enabled = *checked;
            this.owner
                .update(cx, |app, cx| app.set_discovery_ssh_enabled(enabled, cx))
                .ok();
            if let DialogSnapshot::Discovery(dialog) = &mut this.payload.snapshot {
                dialog.include_ssh = enabled;
                if !enabled {
                    dialog.ssh = None;
                    dialog.ssh_error = None;
                }
            }
            cx.notify();
        });
        scan_row = scan_row.child(checkbox_focus_ring(
            "discovery-include-ssh",
            Checkbox::new("discovery-include-ssh")
                .label(self.text(MessageId::DiscoverySshOption))
                .checked(ssh_enabled)
                .on_click(ssh_listener),
        ));
        #[cfg(target_os = "windows")]
        {
            let enabled = dialog.include_wsl;
            let listener = cx.listener(|this, checked, _, cx| {
                let enabled = *checked;
                this.owner
                    .update(cx, |app, cx| app.set_discovery_wsl_enabled(enabled, cx))
                    .ok();
                if let DialogSnapshot::Discovery(dialog) = &mut this.payload.snapshot {
                    dialog.include_wsl = enabled;
                }
                cx.notify();
            });
            scan_row = scan_row.child(checkbox_focus_ring(
                "discovery-include-wsl",
                Checkbox::new("discovery-include-wsl")
                    .label(self.text(MessageId::DiscoveryWslOption))
                    .checked(enabled)
                    .on_click(listener),
            ));
        }
        let mut content = div()
            .flex()
            .flex_col()
            .gap(px(10.))
            .child(ui_text(
                TextRole::DialogTitle,
                TextColor::Primary,
                self.text(MessageId::DiscoveryTitle),
                cx,
            ))
            .child(helper_text(self.text(MessageId::DiscoveryDescription), cx))
            .child(helper_text(
                self.text(MessageId::DiscoveryExistingExcluded),
                cx,
            ))
            .child(scan_row);
        if dialog.include_ssh
            && let Some(path) = &dialog.ssh_path
        {
            content = content.child(ui_text(
                TextRole::Caption,
                TextColor::Secondary,
                format!(
                    "{} {}",
                    self.text(MessageId::DiscoverySshConfigPath),
                    path.display()
                ),
                cx,
            ));
        }
        if dialog.include_ssh
            && let Some(error) = &dialog.ssh_error
        {
            content = content.child(ui_text(
                TextRole::Helper,
                TextColor::Warning,
                format!(
                    "{} {error}",
                    self.text(MessageId::DiscoverySshConfigReadFailed)
                ),
                cx,
            ));
        }
        if dialog.include_ssh
            && let Some(ssh) = &dialog.ssh
        {
            content = content.child(ui_text(
                TextRole::SectionTitle,
                TextColor::Primary,
                format!(
                    "{} ({})",
                    self.text(MessageId::DiscoverySshSection),
                    ssh.hosts.len()
                ),
                cx,
            ));
            if ssh.hosts.is_empty() {
                content = content.child(helper_text(self.text(MessageId::DiscoveryNoSshHosts), cx));
            }
            for host in &ssh.hosts {
                let key = DiscoveryKey::Ssh(host.alias.to_lowercase());
                let selected = dialog.selected.contains(&key);
                let listener = {
                    let key = key.clone();
                    cx.listener(move |this, checked, _, cx| {
                        let selected = *checked;
                        this.owner
                            .update(cx, |app, cx| {
                                app.set_discovery_selection(key.clone(), selected, cx)
                            })
                            .ok();
                        if let DialogSnapshot::Discovery(dialog) = &mut this.payload.snapshot {
                            if selected {
                                dialog.selected.insert(key.clone());
                            } else {
                                dialog.selected.remove(&key);
                            }
                        }
                        cx.notify();
                    })
                };
                let endpoint = format!(
                    "{}@{}:{}",
                    discovered_username(host),
                    host.hostname,
                    host.port
                );
                let auth = match &host.identity_file {
                    Some(path) => {
                        format!("{}: {}", self.text(MessageId::PrivateKey), path.display())
                    }
                    None => self.text(MessageId::Agent).to_owned(),
                };
                let checkbox_id =
                    SharedString::from(format!("discovery-ssh-{}", host.alias.to_lowercase()));
                let mut row = div()
                    .flex()
                    .items_start()
                    .gap(px(10.))
                    .p(px(10.))
                    .rounded(px(8.))
                    .border_1()
                    .border_color(border)
                    .bg(row_bg)
                    .child(checkbox_focus_ring(
                        checkbox_id.clone(),
                        Checkbox::new(checkbox_id)
                            .checked(selected)
                            .on_click(listener),
                    ))
                    .child(
                        div()
                            .flex_1()
                            .min_w(px(0.))
                            .flex()
                            .flex_col()
                            .gap(px(2.))
                            .child(ui_text(
                                TextRole::Label,
                                TextColor::Primary,
                                host.alias.clone(),
                                cx,
                            ))
                            .child(ui_text(
                                TextRole::Caption,
                                TextColor::Secondary,
                                endpoint,
                                cx,
                            ))
                            .child(ui_text(TextRole::Caption, TextColor::Secondary, auth, cx)),
                    );
                if host.identity_file_missing
                    && let Some(path) = &host.identity_file
                {
                    row = row.child(ui_text(
                        TextRole::Caption,
                        TextColor::Warning,
                        format!(
                            "{} {}",
                            self.text(MessageId::DiscoveryIdentityMissing),
                            path.display()
                        ),
                        cx,
                    ));
                }
                content = content.child(row);
            }
            if !ssh.skipped.is_empty() {
                content = content.child(ui_text(
                    TextRole::SectionTitle,
                    TextColor::Primary,
                    self.text(MessageId::DiscoverySkipped),
                    cx,
                ));
                for skipped in &ssh.skipped {
                    content = content.child(
                        div()
                            .flex()
                            .gap(px(6.))
                            .child(ui_text(
                                TextRole::Caption,
                                TextColor::Primary,
                                skipped.alias.clone(),
                                cx,
                            ))
                            .child(ui_text(
                                TextRole::Caption,
                                TextColor::Secondary,
                                skip_reason_text(self.payload.identity.locale, &skipped.reason),
                                cx,
                            )),
                    );
                }
            }
        }
        #[cfg(target_os = "windows")]
        if dialog.include_wsl {
            content = content.child(ui_text(
                TextRole::SectionTitle,
                TextColor::Primary,
                self.text(MessageId::DiscoveryWslSection),
                cx,
            ));
            if let Some(error) = &dialog.wsl_error {
                content = content.child(ui_text(
                    TextRole::Helper,
                    TextColor::Warning,
                    format!("{} {error}", self.text(MessageId::DiscoveryWslReadFailed)),
                    cx,
                ));
            }
            if let Some(distributions) = &dialog.wsl {
                if distributions.is_empty() {
                    content = content.child(helper_text(
                        self.text(MessageId::DiscoveryNoWslDistributions),
                        cx,
                    ));
                }
                for distribution in distributions {
                    let key = DiscoveryKey::Wsl(distribution.to_lowercase());
                    let selected = dialog.selected.contains(&key);
                    let listener = {
                        let key = key.clone();
                        cx.listener(move |this, checked, _, cx| {
                            let selected = *checked;
                            this.owner
                                .update(cx, |app, cx| {
                                    app.set_discovery_selection(key.clone(), selected, cx)
                                })
                                .ok();
                            if let DialogSnapshot::Discovery(dialog) = &mut this.payload.snapshot {
                                if selected {
                                    dialog.selected.insert(key.clone());
                                } else {
                                    dialog.selected.remove(&key);
                                }
                            }
                            cx.notify();
                        })
                    };
                    let checkbox_id = SharedString::from(format!("discovery-wsl-{}", distribution));
                    content = content.child(
                        div()
                            .flex()
                            .items_center()
                            .gap(px(10.))
                            .p(px(10.))
                            .rounded(px(8.))
                            .border_1()
                            .border_color(border)
                            .bg(row_bg)
                            .child(checkbox_focus_ring(
                                checkbox_id.clone(),
                                Checkbox::new(checkbox_id)
                                    .checked(selected)
                                    .on_click(listener),
                            ))
                            .child(div().flex_1().min_w(px(0.)).child(ui_text(
                                TextRole::Label,
                                TextColor::Primary,
                                distribution.clone(),
                                cx,
                            ))),
                    );
                }
            }
        }
        let close_listener = cx.listener(|this, _, _, cx| {
            this.owner
                .update(cx, |app, cx| app.close_discovery(cx))
                .ok();
        });
        let import_listener = cx.listener(|this, _, _, cx| {
            this.owner
                .update(cx, |app, cx| app.commit_discovery_import(cx))
                .ok();
        });
        let manual_add_listener = cx.listener(|this, _, window, cx| {
            this.owner
                .update(cx, |app, cx| app.show_add_connection(window, cx))
                .ok();
        });
        let import_count = dialog.import_count();
        let import_label = if import_count == 0 {
            self.text(MessageId::DiscoveryImport).to_owned()
        } else {
            format!("{} ({import_count})", self.text(MessageId::DiscoveryImport))
        };
        content
            .child(
                div()
                    .flex()
                    .justify_end()
                    .gap(px(8.))
                    .child(
                        button(
                            "manual-add-connection",
                            self.text(MessageId::ManualAdd),
                            ButtonVariant::Secondary,
                        )
                        .on_click(manual_add_listener),
                    )
                    .child(
                        button(
                            "close-discovery",
                            self.text(MessageId::Close),
                            ButtonVariant::Secondary,
                        )
                        .on_click(close_listener),
                    )
                    .child(
                        button("import-discovery", import_label, ButtonVariant::Primary)
                            .disabled(scanning || import_count == 0)
                            .on_click(import_listener),
                    ),
            )
            .into_any_element()
    }

    fn render_local_rename(
        &self,
        dialog: &LocalRenameDialog,
        cx: &mut Context<Self>,
    ) -> gpui::AnyElement {
        let cancel_listener = cx.listener(|this, _, _, cx| {
            this.owner
                .update(cx, |app, cx| app.cancel_local_rename(cx))
                .ok();
        });
        let save_listener = cx.listener(|this, _, window, cx| {
            this.owner
                .update(cx, |app, cx| app.commit_local_rename(window, cx))
                .ok();
        });
        v_form()
            .w(px(420.))
            .label_width(px(130.))
            .child(field().child(ui_text(
                TextRole::DialogTitle,
                TextColor::Primary,
                self.text(MessageId::RenameLocalConnection),
                cx,
            )))
            .child(
                field()
                    .label(self.text(MessageId::Distribution))
                    .child(ui_text(
                        TextRole::Body,
                        TextColor::Primary,
                        dialog.distribution.clone(),
                        cx,
                    )),
            )
            .child(
                field()
                    .label(self.text(MessageId::Name))
                    .child(Input::new(&dialog.name)),
            )
            .child(
                field().child(
                    div()
                        .flex()
                        .justify_end()
                        .gap(px(8.))
                        .child(
                            button(
                                "cancel-local-rename",
                                self.text(MessageId::Cancel),
                                ButtonVariant::Secondary,
                            )
                            .on_click(cancel_listener),
                        )
                        .child(
                            button(
                                "save-local-rename",
                                self.text(MessageId::Rename),
                                ButtonVariant::Primary,
                            )
                            .on_click(save_listener),
                        ),
                ),
            )
            .into_any_element()
    }

    fn render_modal(
        &self,
        request: &ModalRequest,
        secret_input: &Entity<InputState>,
        cx: &mut Context<Self>,
    ) -> gpui::AnyElement {
        match request {
            ModalRequest::Secret { tab_id } => {
                let tab_id = *tab_id;
                let secret_input = secret_input.clone();
                let cancel_listener = cx.listener(move |this, _, window, cx| {
                    this.owner
                        .update(cx, |app, cx| app.cancel_secret(tab_id, window, cx))
                        .ok();
                });
                let submit_listener = cx.listener(move |this, _, window, cx| {
                    this.owner
                        .update(cx, |app, cx| app.submit_secret(tab_id, window, cx))
                        .ok();
                });
                div()
                    .flex()
                    .flex_col()
                    .gap(px(12.))
                    .child(ui_text(
                        TextRole::DialogTitle,
                        TextColor::Primary,
                        self.text(MessageId::CredentialRequired),
                        cx,
                    ))
                    .child(Input::new(&secret_input))
                    .child(
                        div()
                            .flex()
                            .justify_end()
                            .gap(px(8.))
                            .child(
                                button(
                                    "cancel-secret",
                                    self.text(MessageId::Cancel),
                                    ButtonVariant::Secondary,
                                )
                                .on_click(cancel_listener),
                            )
                            .child(
                                button(
                                    "connect-once",
                                    self.text(MessageId::ConnectOnce),
                                    ButtonVariant::Primary,
                                )
                                .on_click(submit_listener),
                            ),
                    )
                    .into_any_element()
            }
            ModalRequest::HostKey {
                tab_id,
                prompt_id,
                endpoint,
                algorithm,
                fingerprint_sha256,
            } => {
                let tab_id = *tab_id;
                let prompt_id = *prompt_id;
                let reject_listener = cx.listener(move |this, _, _, cx| {
                    this.owner
                        .update(cx, |app, cx| {
                            app.decide_host_key(tab_id, prompt_id, HostKeyDecision::Reject, cx)
                        })
                        .ok();
                });
                let accept_listener = cx.listener(move |this, _, _, cx| {
                    this.owner
                        .update(cx, |app, cx| {
                            app.decide_host_key(
                                tab_id,
                                prompt_id,
                                HostKeyDecision::AcceptAndStore,
                                cx,
                            )
                        })
                        .ok();
                });
                div()
                    .flex()
                    .flex_col()
                    .gap(px(10.))
                    .child(ui_text(
                        TextRole::DialogTitle,
                        TextColor::Primary,
                        self.text(MessageId::UnknownHostKey),
                        cx,
                    ))
                    .child(format!("{}:{}", endpoint.host, endpoint.port))
                    .child(algorithm.clone())
                    .child(fingerprint_sha256.clone())
                    .child(
                        div()
                            .flex()
                            .justify_end()
                            .gap(px(8.))
                            .child(
                                button(
                                    "reject-host-key",
                                    self.text(MessageId::Reject),
                                    ButtonVariant::Secondary,
                                )
                                .on_click(reject_listener),
                            )
                            .child(
                                button(
                                    "accept-host-key",
                                    self.text(MessageId::AcceptAndStore),
                                    ButtonVariant::Primary,
                                )
                                .on_click(accept_listener),
                            ),
                    )
                    .into_any_element()
            }
            ModalRequest::ChangedHostKey {
                tab_id,
                request_id,
                endpoint,
                expected_sha256,
                presented_sha256,
            } => {
                let tab_id = *tab_id;
                let request_id = *request_id;
                let close_listener = cx.listener(move |this, _, _, cx| {
                    this.owner
                        .update(cx, |app, cx| {
                            app.close_changed_host_key(tab_id, request_id, cx)
                        })
                        .ok();
                });
                let hosts_listener = cx.listener(move |this, _, _, cx| {
                    this.owner
                        .update(cx, |app, cx| app.open_trusted_hosts(tab_id, request_id, cx))
                        .ok();
                });
                div()
                    .flex()
                    .flex_col()
                    .gap(px(10.))
                    .child(ui_text(
                        TextRole::DialogTitle,
                        TextColor::Primary,
                        self.text(MessageId::ChangedHostKey),
                        cx,
                    ))
                    .child(format!("{}:{}", endpoint.host, endpoint.port))
                    .child(format!(
                        "{}: {}",
                        self.text(MessageId::ExpectedFingerprint),
                        expected_sha256
                    ))
                    .child(format!(
                        "{}: {}",
                        self.text(MessageId::PresentedFingerprint),
                        presented_sha256
                    ))
                    .child(
                        div()
                            .flex()
                            .justify_end()
                            .gap(px(8.))
                            .child(
                                button(
                                    "close-changed-host-key",
                                    self.text(MessageId::Close),
                                    ButtonVariant::Secondary,
                                )
                                .on_click(close_listener),
                            )
                            .child(
                                button(
                                    "open-trusted-hosts",
                                    self.text(MessageId::OpenTrustedHosts),
                                    ButtonVariant::Primary,
                                )
                                .on_click(hosts_listener),
                            ),
                    )
                    .into_any_element()
            }
            ModalRequest::ConfirmClose { tab_id } => {
                let tab_id = *tab_id;
                let cancel_listener = cx.listener(move |this, _, _, cx| {
                    this.owner
                        .update(cx, |app, cx| app.cancel_tab_close(tab_id, cx))
                        .ok();
                });
                let confirm_listener = cx.listener(move |this, _, _, cx| {
                    this.owner
                        .update(cx, |app, cx| app.confirm_tab_close(tab_id, cx))
                        .ok();
                });
                div()
                    .flex()
                    .flex_col()
                    .gap(px(14.))
                    .child(div().child(self.text(MessageId::ConfirmCloseTab)))
                    .child(
                        div()
                            .flex()
                            .justify_end()
                            .gap(px(8.))
                            .child(
                                button(
                                    "cancel-tab-close",
                                    self.text(MessageId::Cancel),
                                    ButtonVariant::Secondary,
                                )
                                .on_click(cancel_listener),
                            )
                            .child(
                                button(
                                    "confirm-tab-close",
                                    self.text(MessageId::Close),
                                    ButtonVariant::Danger,
                                )
                                .on_click(confirm_listener),
                            ),
                    )
                    .into_any_element()
            }
        }
    }
}

impl Render for DialogPresenter {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let mut footer = None;
        let content = match &self.payload.snapshot {
            DialogSnapshot::Editor(editor) => {
                let layout = self.render_editor(editor, cx);
                footer = Some(layout.footer);
                layout.rows
            }
            DialogSnapshot::Confirm { action, affected } => {
                vec![self.render_confirm(action, *affected, cx)]
            }
            DialogSnapshot::TagManager { entries, new_tag } => {
                vec![self.render_tag_manager(entries, new_tag, cx)]
            }
            DialogSnapshot::AnsiPalette { palettes, selected } => {
                vec![self.render_ansi_palette(palettes, *selected, cx)]
            }
            DialogSnapshot::Discovery(dialog) => vec![self.render_discovery(dialog, cx)],
            DialogSnapshot::LocalRename(dialog) => vec![self.render_local_rename(dialog, cx)],
            DialogSnapshot::Modal {
                request,
                secret_input,
            } => vec![self.render_modal(request, secret_input, cx)],
        };
        // The dialog panel is capped at 80% of the viewport (see
        // sync_dialog_layer); the library's own scroll region never engages
        // because its inner container is unbounded. Bound the content here
        // (minus the panel's 24px vertical paddings) so tall forms scroll
        // instead of overflowing the window.
        let max_content_h = window.viewport_size().height * 0.8 - px(48.);
        div()
            .id("oxide-ssh-dialog")
            .key_context("OxideSSHDialog")
            .flex()
            .flex_col()
            .max_h(max_content_h)
            .track_focus(&self.focus.scope)
            .on_action(
                cx.listener(|this, _: &DialogFocusNext, window, cx| this.on_focus_next(window, cx)),
            )
            .on_action(
                cx.listener(|this, _: &DialogFocusPrev, window, cx| this.on_focus_prev(window, cx)),
            )
            .on_action(cx.listener(|this, _: &DialogPrevious, _, cx| {
                this.owner
                    .update(cx, |app, cx| app.move_ansi_palette_selection(false, cx))
                    .ok();
            }))
            .on_action(cx.listener(|this, _: &DialogNext, _, cx| {
                this.owner
                    .update(cx, |app, cx| app.move_ansi_palette_selection(true, cx))
                    .ok();
            }))
            .on_action(cx.listener(|this, _: &DialogSubmit, window, cx| this.on_submit(window, cx)))
            .on_action(cx.listener(|this, _: &DialogCancel, window, cx| this.on_cancel(window, cx)))
            .child(div().track_focus(&self.focus.start))
            .child(
                div()
                    .id("oxide-ssh-dialog-content")
                    .flex_1()
                    .flex()
                    .flex_col()
                    .gap(px(8.))
                    .overflow_y_scroll()
                    .track_scroll(&self.editor_scroll)
                    .children(content),
            )
            .when_some(footer, |this, footer| this.child(footer))
            .child(div().track_focus(&self.focus.end))
    }
}

impl AppView {
    fn requested_dialog(&self) -> Option<DialogPayload> {
        let locale = self.locale;
        if let Some(dialog) = self.ansi_palette_dialog {
            return Some(DialogPayload {
                identity: DialogIdentity {
                    locale,
                    kind: DialogIdentityKind::AnsiPalette,
                },
                snapshot: DialogSnapshot::AnsiPalette {
                    palettes: self.ansi_palette_choices(),
                    selected: dialog.selected,
                },
            });
        }
        if let Some(editor) = &self.editor {
            let save_state = editor_save_state(self.saving_editor, editor.id);
            let tags: Vec<Tag> = self
                .state
                .as_ref()
                .map(|state| state.tags().into_iter().cloned().collect())
                .unwrap_or_default();
            return Some(DialogPayload {
                identity: DialogIdentity {
                    locale,
                    kind: DialogIdentityKind::Editor {
                        editor_id: editor.id,
                        auth_method: editor.form.auth_method,
                        remember: editor.form.remember,
                        save_state,
                        is_editing: editor.form.is_editing(),
                    },
                },
                snapshot: DialogSnapshot::Editor(Box::new(EditorDialogSnapshot {
                    editor_id: editor.id,
                    is_editing: editor.form.is_editing(),
                    auth_method: editor.form.auth_method,
                    remember: editor.form.remember,
                    advanced_open: editor.advanced_open,
                    error: editor.error,
                    save_state,
                    name: editor.name.clone(),
                    host: editor.host.clone(),
                    port: editor.port.clone(),
                    username: editor.username.clone(),
                    private_key_path: editor.private_key_path.clone(),
                    startup_command: editor.startup_command.clone(),
                    ssh_keepalive_interval: editor.ssh_keepalive_interval.clone(),
                    ssh_keepalive_max_count: editor.ssh_keepalive_max_count.clone(),
                    terminal_font_size: editor.terminal_font_size.clone(),
                    ansi_palette: editor.ansi_palette.clone(),
                    environment_variables: editor.environment_variables.clone(),
                    secret: editor.secret.clone(),
                    tags,
                    tag_ids: editor.form.tag_ids.clone(),
                    tag_name: editor.tag_name.clone(),
                })),
            });
        }
        if let Some(dialog) = &self.discovery {
            return Some(DialogPayload {
                identity: DialogIdentity {
                    locale,
                    kind: DialogIdentityKind::Discovery,
                },
                snapshot: DialogSnapshot::Discovery(dialog.clone()),
            });
        }
        if let Some(dialog) = &self.local_rename {
            return Some(DialogPayload {
                identity: DialogIdentity {
                    locale,
                    kind: DialogIdentityKind::LocalRename { id: dialog.id },
                },
                snapshot: DialogSnapshot::LocalRename(dialog.clone()),
            });
        }
        if let Some(action) = &self.confirm {
            let affected = match action {
                ConfirmAction::DeleteTag(id) => self
                    .state
                    .as_ref()
                    .map(|state| state.profiles_with_tag(*id).len())
                    .unwrap_or(0),
                _ => 0,
            };
            return Some(DialogPayload {
                identity: DialogIdentity {
                    locale,
                    kind: DialogIdentityKind::Confirm(action.clone()),
                },
                snapshot: DialogSnapshot::Confirm {
                    action: action.clone(),
                    affected,
                },
            });
        }
        if let Some(manager) = &self.tag_manager {
            return Some(DialogPayload {
                identity: DialogIdentity {
                    locale,
                    kind: DialogIdentityKind::TagManager,
                },
                snapshot: DialogSnapshot::TagManager {
                    entries: manager.entries.clone(),
                    new_tag: manager.new_tag.clone(),
                },
            });
        }
        let request = self.tabs.modals().current()?.clone();
        let kind = match &request {
            ModalRequest::HostKey {
                tab_id, prompt_id, ..
            } => DialogIdentityKind::HostKey {
                tab_id: *tab_id,
                prompt_id: *prompt_id,
            },
            ModalRequest::ChangedHostKey {
                tab_id, request_id, ..
            } => DialogIdentityKind::ChangedHostKey {
                tab_id: *tab_id,
                request_id: *request_id,
            },
            ModalRequest::Secret { tab_id } => DialogIdentityKind::Secret(*tab_id),
            ModalRequest::ConfirmClose { tab_id } => DialogIdentityKind::ConfirmClose(*tab_id),
        };
        Some(DialogPayload {
            identity: DialogIdentity { locale, kind },
            snapshot: DialogSnapshot::Modal {
                request,
                secret_input: self.secret.clone(),
            },
        })
    }

    fn schedule_dialog_focus(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let identity = self
            .requested_dialog()
            .map(|payload| payload.identity.kind.focus_identity());
        cx.defer_in(window, move |this, window, cx| {
            let current = this
                .requested_dialog()
                .map(|payload| payload.identity.kind.focus_identity());
            if current != identity {
                return;
            }
            if let Some(presenter) = &this.dialog_presenter {
                presenter.update(cx, |presenter, cx| presenter.focus_initial(window, cx));
            }
        });
    }

    pub(super) fn sync_dialog_layer(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let requested = self.requested_dialog();
        match (requested, &self.dialog_presenter) {
            (None, None) => {
                if self.pending_terminal_focus {
                    self.pending_terminal_focus = false;
                    self.terminal_focus.focus(window);
                }
            }
            (None, Some(_)) => {
                window.close_dialog(cx);
                self.dialog_presenter = None;
                if self.pending_terminal_focus {
                    self.pending_terminal_focus = false;
                    self.terminal_focus.focus(window);
                }
                cx.notify();
            }
            (Some(payload), None) => {
                let overlay_closable =
                    matches!(&payload.snapshot, DialogSnapshot::AnsiPalette { .. });
                let owner = cx.entity().downgrade();
                let presenter = cx.new(|cx| DialogPresenter::new(owner.clone(), payload, cx));
                let presenter_entity = presenter.clone();
                self.dialog_presenter = Some(presenter);
                window.open_dialog(cx, move |mut dialog, window, _cx| {
                    let presenter = presenter_entity.clone();
                    let width = presenter.read(_cx).dialog_width();
                    // The panel is top-anchored with natural height; without a
                    // bound, tall content (editor, tag manager) overflows the
                    // window. Cap at 80% so the built-in vertical scrollbar
                    // engages and the panel stays fully visible.
                    dialog.style().max_size.height =
                        Some((window.viewport_size().height * 0.8).into());
                    let mut dialog = dialog
                        .overlay_closable(overlay_closable)
                        .keyboard(false)
                        .close_button(false)
                        .width(width)
                        .child(presenter);
                    if overlay_closable {
                        let owner = owner.clone();
                        dialog = dialog.on_cancel(move |_, _, cx| {
                            owner
                                .update(cx, |app, cx| app.close_ansi_palette_dialog(cx))
                                .ok();
                            true
                        });
                    }
                    dialog
                });
                self.schedule_dialog_focus(window, cx);
            }
            (Some(payload), Some(existing)) => {
                let needs_focus = existing.read(cx).payload.identity.kind.focus_identity()
                    != payload.identity.kind.focus_identity();
                existing.update(cx, |presenter, cx| {
                    presenter.payload = payload;
                    cx.notify();
                });
                if needs_focus {
                    self.schedule_dialog_focus(window, cx);
                }
            }
        }
    }

    fn cancel_confirm(&mut self, cx: &mut Context<Self>) {
        self.confirm = None;
        cx.notify();
    }

    fn close_changed_host_key(&mut self, tab_id: TabId, request_id: Uuid, cx: &mut Context<Self>) {
        if self
            .tabs
            .modals_mut()
            .complete_current_if(|request| request.is_changed_host_key_for(tab_id, request_id))
            .is_none()
        {
            return;
        }
        cx.notify();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn confirmation_variant_matches_action_risk() {
        let path = PathBuf::from("backup.json");
        for action in [
            ConfirmAction::DeleteProfile(ProfileId::new()),
            ConfirmAction::DeleteHost(Endpoint {
                host: "example.com".into(),
                port: 22,
            }),
            ConfirmAction::DeleteTag(TagId::new()),
            ConfirmAction::DeleteLocalConnection(LocalConnectionId::new()),
            ConfirmAction::ClearAllCredentials,
            ConfirmAction::Quit,
            ConfirmAction::QuitApp,
            ConfirmAction::ImportConfig(path.clone()),
            ConfirmAction::ImportKnownHostsReplace(path),
        ] {
            assert_eq!(action.accept_variant(), ButtonVariant::Danger);
            assert!(action.cancel_on_submit());
        }
        for action in [
            ConfirmAction::Paste {
                tab_id: TabId::new(),
                text: "paste".into(),
            },
            ConfirmAction::Osc52Clipboard {
                tab_id: TabId::new(),
                session_id: SessionId::new(),
                text: "clipboard".into(),
            },
        ] {
            assert_eq!(action.accept_variant(), ButtonVariant::Primary);
            assert!(!action.cancel_on_submit());
        }
    }

    #[gpui::test]
    async fn configuration_import_cancel_enter_and_confirm(cx: &mut gpui::TestAppContext) {
        use tempfile::tempdir;

        cx.update(gpui_component::init);
        let source_dir = tempdir().unwrap();
        let source =
            oxide_ssh_core::storage::AppStore::open(source_dir.path().to_path_buf()).unwrap();
        source
            .update_settings(|settings| settings.terminal_font_size = 18)
            .unwrap();
        let backup = source_dir.path().join("backup.json");
        source.export_config(&backup).unwrap();

        let target_dir = tempdir().unwrap();
        let target =
            oxide_ssh_core::storage::AppStore::open(target_dir.path().to_path_buf()).unwrap();
        let before = target.load_config().unwrap();
        let root = target_dir.path().to_path_buf();
        let (view, cx) = cx.add_window_view(|window, cx| {
            AppView::new(root.clone(), AppState::load(root), window, cx)
        });

        view.update(cx, |app, _cx| {
            app.confirm = Some(ConfirmAction::ImportConfig(backup.clone()));
        });
        cx.update(|window, app_cx| {
            let payload = view.read(app_cx).requested_dialog().unwrap();
            let presenter = app_cx.new(|cx| DialogPresenter::new(view.downgrade(), payload, cx));
            presenter.update(app_cx, |presenter, cx| presenter.on_submit(window, cx));
        });

        assert!(cx.read(|app| view.read(app).confirm.is_none()));
        assert_eq!(
            cx.read(|app| view.read(app).state.as_ref().unwrap().settings().clone()),
            before.settings
        );
        assert_eq!(target.load_config().unwrap(), before);

        view.update(cx, |app, _cx| {
            app.confirm = Some(ConfirmAction::ImportConfig(backup.clone()));
        });
        cx.update(|window, app_cx| {
            view.update(app_cx, |app, cx| app.perform_confirm(window, cx));
        });

        assert_eq!(
            cx.read(|app| {
                view.read(app)
                    .state
                    .as_ref()
                    .unwrap()
                    .settings()
                    .terminal_font_size
            }),
            18
        );
        assert_eq!(
            target.load_config().unwrap().settings.terminal_font_size,
            18
        );
    }

    #[gpui::test]
    async fn known_host_replacement_cancel_and_confirm(cx: &mut gpui::TestAppContext) {
        use oxide_ssh_core::storage::{KnownHostEntry, KnownHosts};
        use tempfile::tempdir;

        cx.update(gpui_component::init);
        let target_dir = tempdir().unwrap();
        let target =
            oxide_ssh_core::storage::AppStore::open(target_dir.path().to_path_buf()).unwrap();
        let existing = KnownHostEntry {
            host: "existing.example".into(),
            port: 22,
            algorithm: "ssh-ed25519".into(),
            public_key:
                "ssh-ed25519 AAAAC3NzaC1lZDI1NTE5AAAAILagOJFgwaMNhBWQINinKOXmqS4Gh5NgxgriXwdOoINJ"
                    .into(),
            fingerprint_sha256: "SHA256:ldyiXa1JQakitNU5tErauu8DvWQ1dZ7aXu+rm7KQuog".into(),
            accepted_at_unix: 0,
        };
        target.accept_host_key(existing.clone()).unwrap();
        let replacement = KnownHostEntry {
            public_key:
                "ssh-ed25519 AAAAC3NzaC1lZDI1NTE5AAAAIBJJ1YTZOJKczfSjkCqsiGYm/rx//5Qlf5h77EhjqEnL"
                    .into(),
            fingerprint_sha256: "SHA256:2IT0bxYpFn1tl6wHZcK7fsb4T18rUGggIJUaq3ld4X8".into(),
            ..existing.clone()
        };
        let backup = target_dir.path().join("known-hosts.json");
        let mut imported = KnownHosts::default();
        imported.hosts = vec![replacement.clone()];
        std::fs::write(&backup, serde_json::to_vec(&imported).unwrap()).unwrap();

        let root = target_dir.path().to_path_buf();
        let (view, cx) = cx.add_window_view(|window, cx| {
            AppView::new(root.clone(), AppState::load(root), window, cx)
        });
        view.update(cx, |app, cx| {
            app.confirm = Some(ConfirmAction::ImportKnownHostsReplace(backup.clone()));
            app.cancel_confirm(cx);
        });
        assert_eq!(
            cx.read(|app| {
                view.read(app)
                    .state
                    .as_ref()
                    .unwrap()
                    .known_hosts()
                    .to_vec()
            }),
            std::slice::from_ref(&existing)
        );
        assert_eq!(target.load_known_hosts().unwrap().hosts, [existing]);

        view.update(cx, |app, _cx| {
            app.confirm = Some(ConfirmAction::ImportKnownHostsReplace(backup.clone()));
        });
        cx.update(|window, app_cx| {
            view.update(app_cx, |app, cx| app.perform_confirm(window, cx));
        });
        assert_eq!(
            cx.read(|app| {
                view.read(app)
                    .state
                    .as_ref()
                    .unwrap()
                    .known_hosts()
                    .to_vec()
            }),
            std::slice::from_ref(&replacement)
        );
        assert_eq!(target.load_known_hosts().unwrap().hosts, [replacement]);
    }

    #[test]
    fn editor_focus_identity_survives_payload_updates() {
        let editor_id = EditorId(Uuid::new_v4());
        let initial = DialogIdentityKind::Editor {
            editor_id,
            auth_method: AuthMethod::Password,
            remember: false,
            save_state: EditorSaveState::Idle,
            is_editing: false,
        };
        let updated = DialogIdentityKind::Editor {
            editor_id,
            auth_method: AuthMethod::PrivateKey,
            remember: true,
            save_state: EditorSaveState::SavingThis,
            is_editing: false,
        };

        assert_eq!(initial.focus_identity(), updated.focus_identity());
    }

    #[test]
    fn request_focus_identity_requires_the_exact_request() {
        let tab_id = TabId::new();
        let first = DialogIdentityKind::HostKey {
            tab_id,
            prompt_id: Uuid::new_v4(),
        };
        let second = DialogIdentityKind::HostKey {
            tab_id,
            prompt_id: Uuid::new_v4(),
        };

        assert_ne!(first.focus_identity(), second.focus_identity());
    }
}
