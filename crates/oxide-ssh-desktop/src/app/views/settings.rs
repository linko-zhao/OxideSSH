use std::path::Path;

use super::*;
use gpui_component::select::Select;

fn settings_card(rows: Vec<AnyElement>, cx: &Context<AppView>) -> AnyElement {
    let mut card = div()
        .w_full()
        .flex()
        .flex_col()
        .rounded(cx.theme().radius_lg)
        .border_1()
        .border_color(cx.theme().border)
        .bg(cx.theme().background);
    for (index, row) in rows.into_iter().enumerate() {
        if index > 0 {
            card = card.child(div().h(px(1.)).mx(px(20.)).bg(cx.theme().border));
        }
        card = card.child(row);
    }
    card.into_any_element()
}

fn settings_page(title: &'static str, rows: Vec<AnyElement>, cx: &Context<AppView>) -> AnyElement {
    div()
        .w_full()
        .max_w(px(880.))
        .mx_auto()
        .flex()
        .flex_col()
        .gap(px(24.))
        .child(
            div()
                .text_size(px(30.))
                .font_weight(gpui::FontWeight::BOLD)
                .child(title),
        )
        .child(settings_card(rows, cx))
        .into_any_element()
}

impl AppView {
    fn setting_row(
        &self,
        title: MessageId,
        description: MessageId,
        control: AnyElement,
        cx: &Context<Self>,
    ) -> AnyElement {
        h_flex()
            .w_full()
            .items_center()
            .gap(px(24.))
            .p(px(20.))
            .child(
                div()
                    .flex_1()
                    .min_w(px(0.))
                    .flex()
                    .flex_col()
                    .gap(px(4.))
                    .child(
                        div()
                            .text_sm()
                            .font_weight(gpui::FontWeight::SEMIBOLD)
                            .child(self.text(title)),
                    )
                    .child(
                        div()
                            .text_sm()
                            .text_color(cx.theme().muted_foreground)
                            .child(self.text(description)),
                    ),
            )
            .child(div().flex_shrink_0().child(control))
            .into_any_element()
    }

    pub(in crate::app) fn render_settings(&mut self, cx: &mut Context<Self>) -> gpui::AnyElement {
        let category = self.settings_category;

        let mut side_nav = div().flex().flex_col().gap(px(4.));
        let categories = [
            (
                SettingsCategory::General,
                "settings-cat-general",
                IconName::Settings2,
                self.text(MessageId::General),
            ),
            (
                SettingsCategory::Appearance,
                "settings-cat-appearance",
                IconName::Palette,
                self.text(MessageId::Appearance),
            ),
            (
                SettingsCategory::Security,
                "settings-cat-security",
                IconName::CircleCheck,
                self.text(MessageId::Security),
            ),
        ];
        for (item_category, id, icon, label) in categories {
            side_nav = side_nav.child(
                self.nav_item(id, icon, label, category == item_category, cx)
                    .cursor_pointer()
                    .on_click(cx.listener(move |this, _, _, cx| {
                        this.settings_category = item_category;
                        cx.notify();
                    })),
            );
        }

        let content = match category {
            SettingsCategory::General => {
                let confirm = self
                    .state
                    .as_ref()
                    .is_none_or(|state| state.settings().confirm_before_close_with_sessions);
                let view = cx.entity();
                settings_page(
                    self.text(MessageId::General),
                    vec![
                        self.setting_row(
                            MessageId::Language,
                            MessageId::LanguageDescription,
                            Select::new(&self.locale_select)
                                .w(px(180.))
                                .with_size(gpui_component::Size::Small)
                                .into_any_element(),
                            cx,
                        ),
                        self.setting_row(
                            MessageId::SshConnectTimeout,
                            MessageId::SshConnectTimeoutDescription,
                            Select::new(&self.ssh_connect_timeout_select)
                                .w(px(180.))
                                .with_size(gpui_component::Size::Small)
                                .into_any_element(),
                            cx,
                        ),
                        self.setting_row(
                            MessageId::SshKeepaliveInterval,
                            MessageId::SshKeepaliveInterval,
                            Input::new(&self.ssh_keepalive_interval)
                                .w(px(180.))
                                .into_any_element(),
                            cx,
                        ),
                        self.setting_row(
                            MessageId::SshKeepaliveMaxCount,
                            MessageId::SshKeepaliveMaxCount,
                            Input::new(&self.ssh_keepalive_max_count)
                                .w(px(180.))
                                .into_any_element(),
                            cx,
                        ),
                        self.setting_row(
                            MessageId::TerminalBell,
                            MessageId::BellDescription,
                            Select::new(&self.terminal_bell_select)
                                .w(px(180.))
                                .with_size(gpui_component::Size::Small)
                                .into_any_element(),
                            cx,
                        ),
                        self.setting_row(
                            MessageId::NewTabButton,
                            MessageId::NewTabButtonDescription,
                            Select::new(&self.new_tab_action_select)
                                .w(px(180.))
                                .with_size(gpui_component::Size::Small)
                                .into_any_element(),
                            cx,
                        ),
                        self.setting_row(
                            MessageId::SshCompression,
                            MessageId::SshCompressionDescription,
                            Checkbox::new("ssh-compression")
                                .checked(
                                    self.state
                                        .as_ref()
                                        .is_some_and(|state| state.settings().ssh_compression),
                                )
                                .on_click(cx.listener(|this, checked, _, cx| {
                                    this.set_ssh_compression(*checked, cx);
                                }))
                                .into_any_element(),
                            cx,
                        ),
                        self.setting_row(
                            MessageId::ConfirmBeforeCloseWithSessions,
                            MessageId::ConfirmBeforeCloseWithSessionsDescription,
                            Checkbox::new("confirm-before-close-with-sessions")
                                .checked(confirm)
                                .on_click(move |checked, _, cx| {
                                    view.update(cx, |this, cx| {
                                        this.set_confirm_before_close_with_sessions(*checked, cx)
                                    });
                                })
                                .into_any_element(),
                            cx,
                        ),
                    ],
                    cx,
                )
            }
            SettingsCategory::Appearance => settings_page(
                self.text(MessageId::Appearance),
                vec![
                    self.setting_row(
                        MessageId::Theme,
                        MessageId::ThemeDescription,
                        Select::new(&self.theme_select)
                            .w(px(180.))
                            .with_size(gpui_component::Size::Small)
                            .into_any_element(),
                        cx,
                    ),
                    self.setting_row(
                        MessageId::AnsiPalette,
                        MessageId::AnsiPaletteDescription,
                        div()
                            .flex()
                            .flex_col()
                            .gap(px(8.))
                            .child(Input::new(&self.ansi_palette_input))
                            .child(
                                h_flex()
                                    .gap(px(8.))
                                    .child(
                                        Button::new("save-ansi-palette")
                                            .label(self.text(MessageId::Save))
                                            .on_click(cx.listener(|this, _, window, cx| {
                                                this.save_ansi_palette(window, cx);
                                            })),
                                    )
                                    .child(
                                        Button::new("reset-ansi-palette")
                                            .label(self.text(MessageId::Reset))
                                            .on_click(cx.listener(|this, _, window, cx| {
                                                this.reset_ansi_palette(window, cx);
                                            })),
                                    ),
                            )
                            .into_any_element(),
                        cx,
                    ),
                ],
                cx,
            ),
            SettingsCategory::Security => {
                let known_hosts = self
                    .state
                    .as_ref()
                    .map(|state| state.known_hosts().to_vec())
                    .unwrap_or_default();
                let mut rows = Vec::with_capacity(known_hosts.len().max(1));
                if known_hosts.is_empty() {
                    rows.push(
                        div()
                            .p(px(20.))
                            .text_sm()
                            .text_color(cx.theme().muted_foreground)
                            .child(self.text(MessageId::NoTrustedHosts))
                            .into_any_element(),
                    );
                }
                for host in known_hosts {
                    let endpoint = host.endpoint();
                    let title = format!("{}:{}", host.host, host.port);
                    let algorithm = host.algorithm.clone();
                    let view = cx.entity();
                    rows.push(
                        h_flex()
                            .w_full()
                            .items_center()
                            .gap(px(24.))
                            .p(px(20.))
                            .child(
                                div()
                                    .flex_1()
                                    .min_w(px(0.))
                                    .flex()
                                    .flex_col()
                                    .gap(px(4.))
                                    .child(
                                        div()
                                            .text_sm()
                                            .font_weight(gpui::FontWeight::SEMIBOLD)
                                            .child(title),
                                    )
                                    .child(
                                        div()
                                            .text_sm()
                                            .text_color(cx.theme().muted_foreground)
                                            .child(format!(
                                                "{}\n{}",
                                                algorithm, host.fingerprint_sha256
                                            )),
                                    ),
                            )
                            .child(
                                Button::new(SharedString::from(format!(
                                    "delete-host-{}-{}",
                                    endpoint.host, endpoint.port
                                )))
                                .icon(gpui_component::Icon::empty().path("icons/trash-2.svg"))
                                .danger()
                                .compact()
                                .on_click(move |_, _, cx| {
                                    view.update(cx, |this, cx| {
                                        this.confirm =
                                            Some(ConfirmAction::DeleteHost(endpoint.clone()));
                                        cx.notify();
                                    });
                                }),
                            )
                            .into_any_element(),
                    );
                }
                let view = cx.entity();
                rows.push(
                    h_flex()
                        .w_full()
                        .items_center()
                        .justify_between()
                        .gap(px(16.))
                        .p(px(20.))
                        .child(
                            div()
                                .text_sm()
                                .child(self.text(MessageId::ClearAllCredentials)),
                        )
                        .child(
                            Button::new("clear-all-credentials")
                                .label(self.text(MessageId::ClearAllCredentials))
                                .danger()
                                .on_click(move |_, _, cx| {
                                    view.update(cx, |this, cx| {
                                        this.confirm = Some(ConfirmAction::ClearAllCredentials);
                                        cx.notify();
                                    });
                                }),
                        )
                        .into_any_element(),
                );
                if !self.credential_clear_failures.is_empty() {
                    rows.push(
                        div()
                            .p(px(20.))
                            .text_sm()
                            .child(self.text(MessageId::ClearCredentialsFailures))
                            .child(self.credential_clear_failures.join("\n"))
                            .into_any_element(),
                    );
                }
                rows.push(
                    h_flex()
                        .gap(px(8.))
                        .p(px(16.))
                        .child(
                            Button::new("export-known-hosts")
                                .label(self.text(MessageId::ExportKnownHosts))
                                .on_click(cx.listener(|_this, _, _, cx| {
                                    let receiver = cx.prompt_for_new_path(
                                        Path::new("."),
                                        Some("known_hosts.json"),
                                    );
                                    cx.spawn(async move |weak, cx| {
                                        if let Ok(Ok(Some(path))) = receiver.await {
                                            weak.update(cx, |this, cx| {
                                                let result = this.state.as_ref().map(|state| {
                                                    state.store().export_known_hosts(&path)
                                                });
                                                this.status_message = Some(
                                                    if matches!(result, Some(Ok(()))) {
                                                        MessageId::KnownHostsExported
                                                    } else {
                                                        MessageId::KnownHostsTransferFailed
                                                    },
                                                );
                                                cx.notify();
                                            }).ok();
                                        }
                                    }).detach();
                                })),
                        )
                        .child(
                            Button::new("import-known-hosts-keep")
                                .label(self.text(MessageId::ImportKeepExisting))
                                .on_click(cx.listener(|this, _, _, cx| {
                                    this.choose_known_hosts_import(
                                        oxide_ssh_core::storage::KnownHostsConflictPolicy::KeepExisting,
                                        cx,
                                    );
                                })),
                        )
                        .child(
                            Button::new("import-known-hosts-replace")
                                .label(self.text(MessageId::ImportReplaceExisting))
                                .on_click(cx.listener(|this, _, _, cx| {
                                    this.choose_known_hosts_import(
                                        oxide_ssh_core::storage::KnownHostsConflictPolicy::ReplaceExisting,
                                        cx,
                                    );
                                })),
                        )
                        .into_any_element(),
                );
                settings_page(self.text(MessageId::Security), rows, cx)
            }
        };

        div()
            .size_full()
            .flex()
            .flex_col()
            .child(
                h_flex()
                    .flex_1()
                    .min_h(px(0.))
                    .gap(px(CARD_GAP))
                    .child(
                        div()
                            .w(px(200.))
                            .h_full()
                            .flex()
                            .flex_col()
                            .gap(px(10.))
                            .p(px(12.))
                            .rounded(cx.theme().radius_lg)
                            .border_1()
                            .border_color(cx.theme().border)
                            .bg(cx.theme().secondary)
                            .child(
                                h_flex()
                                    .items_center()
                                    .gap(px(4.))
                                    .child(
                                        Button::new("back-to-workspace")
                                            .icon(IconName::ArrowLeft)
                                            .ghost()
                                            .compact()
                                            .on_click(cx.listener(|this, _, _, cx| {
                                                this.main_view = MainView::Workspace;
                                                cx.notify();
                                            })),
                                    )
                                    .child(
                                        div()
                                            .px(px(10.))
                                            .py(px(8.))
                                            .text_size(px(18.))
                                            .font_weight(gpui::FontWeight::BOLD)
                                            .child(self.text(MessageId::Settings)),
                                    ),
                            )
                            .child(side_nav),
                    )
                    .child(
                        div()
                            .id("settings-scroll")
                            .flex_1()
                            .h_full()
                            .overflow_y_scroll()
                            .p(px(32.))
                            .child(content),
                    ),
            )
            .into_any_element()
    }
    fn choose_known_hosts_import(
        &mut self,
        policy: oxide_ssh_core::storage::KnownHostsConflictPolicy,
        cx: &mut Context<Self>,
    ) {
        let receiver = cx.prompt_for_paths(gpui::PathPromptOptions {
            files: true,
            directories: false,
            multiple: false,
            prompt: Some(self.text(MessageId::ImportKnownHosts).into()),
        });
        cx.spawn(async move |weak, cx| {
            if let Ok(Ok(Some(paths))) = receiver.await
                && let Some(path) = paths.into_iter().next()
            {
                weak.update(cx, |this, cx| {
                    let result = this.state.as_mut().map(|state| {
                        let store = state.store().clone();
                        store
                            .import_known_hosts(&path, policy)
                            .and_then(|()| state.reload_known_hosts())
                    });
                    this.status_message = Some(if matches!(result, Some(Ok(()))) {
                        MessageId::KnownHostsImported
                    } else {
                        MessageId::KnownHostsTransferFailed
                    });
                    cx.notify();
                })
                .ok();
            }
        })
        .detach();
    }
}
