use super::*;

impl AppView {
    fn render_host_card(
        &mut self,
        profile: ConnectionProfile,
        scroll_index: usize,
        cx: &mut Context<Self>,
    ) -> gpui::AnyElement {
        let (tile_color, initial_color) = avatar_colors(&profile.id.0);
        let initial: String = profile
            .name
            .trim()
            .chars()
            .next()
            .map(|ch| ch.to_uppercase().collect())
            .unwrap_or_else(|| "?".into());
        let endpoint = format!(
            "{}@{}:{}",
            profile.username, profile.endpoint.host, profile.endpoint.port
        );
        let status = self.profile_status(profile.id);
        let edit_id = SharedString::from(format!("edit-{}", profile.id.0));
        let delete_id = SharedString::from(format!("delete-{}", profile.id.0));
        let profile_for_connect = profile.clone();
        let profile_for_edit = profile.clone();
        let profile_for_delete = profile.clone();
        let view = cx.entity();
        let theme = cx.theme();
        let border = theme.input;
        let card_bg = theme.secondary;
        // Hover lifts the card one step above its resting background.
        let hover_bg = theme.accent;
        let radius_lg = theme.radius_lg;
        let card_scroll = self.workspace_scroll.clone();
        let edit_scroll = self.workspace_scroll.clone();
        let delete_scroll = self.workspace_scroll.clone();
        custom_button(
            SharedString::from(format!("host-card-{}", profile.id.0)),
            ButtonVariant::Tertiary,
        )
        .with_variant(gpui_component::button::ButtonVariant::Custom(
            gpui_component::button::ButtonCustomVariant::new(cx)
                .color(card_bg)
                .hover(hover_bg)
                .active(hover_bg)
                .border(border),
        ))
        .w(px(300.))
        .h_auto()
        .p(px(14.))
        .rounded(radius_lg)
        .border_1()
        .border_color(border)
        .bg(card_bg)
        .cursor_pointer()
        .on_focus(move |_, _| card_scroll.scroll_to_item(scroll_index))
        .on_click(cx.listener(move |this, _, window, cx| {
            this.connect_profile(profile_for_connect.clone(), window, cx)
        }))
        .child(
            div()
                .w_full()
                .flex()
                .flex_col()
                .gap(px(10.))
                .child(
                    div()
                        .flex()
                        .items_center()
                        .gap(px(12.))
                        .child(
                            div()
                                .size(px(36.))
                                .flex_none()
                                .rounded_full()
                                .bg(tile_color)
                                .flex()
                                .items_center()
                                .justify_center()
                                .text_color(initial_color)
                                .font_weight(gpui::FontWeight::BOLD)
                                .child(initial),
                        )
                        .child(
                            div()
                                .flex_1()
                                .min_w_0()
                                .overflow_hidden()
                                .flex()
                                .flex_col()
                                .gap(px(2.))
                                .child(
                                    ui_text(
                                        TextRole::Label,
                                        TextColor::Primary,
                                        profile.name.clone(),
                                        cx,
                                    )
                                    .text_ellipsis(),
                                )
                                .child(
                                    ui_text(TextRole::Caption, TextColor::Secondary, endpoint, cx)
                                        .text_ellipsis(),
                                ),
                        )
                        .child(
                            h_flex()
                                .flex_none()
                                .gap(px(2.))
                                .child(
                                    icon_button(
                                        edit_id,
                                        IconName::Settings2,
                                        ButtonVariant::Tertiary,
                                        ButtonSize::Icon,
                                    )
                                    .on_focus(move |_, _| edit_scroll.scroll_to_item(scroll_index))
                                    .tooltip(format!(
                                        "{}\n{}\n{}@{}:{}",
                                        self.text(MessageId::Edit),
                                        profile.name,
                                        profile.username,
                                        profile.endpoint.host,
                                        profile.endpoint.port
                                    ))
                                    .on_click(
                                        move |_, window, cx| {
                                            view.update(cx, |this, cx| {
                                                cx.stop_propagation();
                                                this.show_edit_connection(
                                                    profile_for_edit.clone(),
                                                    window,
                                                    cx,
                                                );
                                            });
                                        },
                                    ),
                                )
                                .child({
                                    let view = cx.entity();
                                    icon_button(
                                        delete_id,
                                        IconName::Delete,
                                        ButtonVariant::Tertiary,
                                        ButtonSize::Icon,
                                    )
                                    .tooltip(self.text(MessageId::Delete))
                                    .on_focus(move |_, _| {
                                        delete_scroll.scroll_to_item(scroll_index)
                                    })
                                    .on_click(
                                        move |_, _, cx| {
                                            view.update(cx, |this, cx| {
                                                cx.stop_propagation();
                                                this.confirm = Some(ConfirmAction::DeleteProfile(
                                                    profile_for_delete.id,
                                                ));
                                                cx.notify();
                                            });
                                        },
                                    )
                                }),
                        ),
                )
                .when_some(status, |card, (color, status)| {
                    card.child(ui_text(TextRole::Caption, color, status, cx))
                }),
        )
        .into_any_element()
    }

    fn render_local_card(
        &mut self,
        connection: LocalConnection,
        scroll_index: usize,
        cx: &mut Context<Self>,
    ) -> gpui::AnyElement {
        let (tile_color, initial_color) = avatar_colors(&connection.id.0);
        let initial: String = connection
            .name
            .trim()
            .chars()
            .next()
            .map(|ch| ch.to_uppercase().collect())
            .unwrap_or_else(|| "?".into());
        let name = connection.name.clone();
        let distribution = connection.distribution.clone();
        let rename_id = SharedString::from(format!("rename-local-{}", connection.id.0));
        let delete_id = SharedString::from(format!("delete-local-{}", connection.id.0));
        let connection_for_connect = connection.clone();
        let connection_for_rename = connection.clone();
        let connection_for_delete = connection.clone();
        let view = cx.entity();
        let theme = cx.theme();
        let border = theme.input;
        let card_bg = theme.secondary;
        let hover_bg = theme.accent;
        let radius_lg = theme.radius_lg;
        let card_scroll = self.workspace_scroll.clone();
        let rename_scroll = self.workspace_scroll.clone();
        let delete_scroll = self.workspace_scroll.clone();
        custom_button(
            SharedString::from(format!("local-card-{}", connection.id.0)),
            ButtonVariant::Tertiary,
        )
        .with_variant(gpui_component::button::ButtonVariant::Custom(
            gpui_component::button::ButtonCustomVariant::new(cx)
                .color(card_bg)
                .hover(hover_bg)
                .active(hover_bg)
                .border(border),
        ))
        .w(px(300.))
        .h_auto()
        .p(px(14.))
        .rounded(radius_lg)
        .border_1()
        .border_color(border)
        .bg(card_bg)
        .cursor_pointer()
        .on_focus(move |_, _| card_scroll.scroll_to_item(scroll_index))
        .on_click(cx.listener(move |this, _, window, cx| {
            this.connect_local_connection(connection_for_connect.clone(), window, cx)
        }))
        .child(
            div()
                .flex()
                .items_center()
                .gap(px(12.))
                .child(
                    div()
                        .size(px(36.))
                        .flex_none()
                        .rounded_full()
                        .bg(tile_color)
                        .flex()
                        .items_center()
                        .justify_center()
                        .text_color(initial_color)
                        .font_weight(gpui::FontWeight::BOLD)
                        .child(initial),
                )
                .child(
                    div()
                        .flex_1()
                        .min_w_0()
                        .overflow_hidden()
                        .flex()
                        .flex_col()
                        .gap(px(2.))
                        .child(
                            ui_text(TextRole::Label, TextColor::Primary, name, cx).text_ellipsis(),
                        )
                        .child(
                            ui_text(TextRole::Caption, TextColor::Secondary, distribution, cx)
                                .text_ellipsis(),
                        ),
                )
                .child(
                    h_flex()
                        .flex_none()
                        .gap(px(2.))
                        .child(
                            icon_button(
                                rename_id,
                                IconName::Settings2,
                                ButtonVariant::Tertiary,
                                ButtonSize::Icon,
                            )
                            .on_focus(move |_, _| rename_scroll.scroll_to_item(scroll_index))
                            .tooltip(format!(
                                "{}\n{}\n{}",
                                self.text(MessageId::Rename),
                                connection.name,
                                connection.distribution
                            ))
                            .on_click(move |_, window, cx| {
                                view.update(cx, |this, cx| {
                                    cx.stop_propagation();
                                    this.start_local_rename(
                                        connection_for_rename.clone(),
                                        window,
                                        cx,
                                    );
                                });
                            }),
                        )
                        .child({
                            let view = cx.entity();
                            icon_button(
                                delete_id,
                                IconName::Delete,
                                ButtonVariant::Tertiary,
                                ButtonSize::Icon,
                            )
                            .tooltip(self.text(MessageId::Delete))
                            .on_focus(move |_, _| delete_scroll.scroll_to_item(scroll_index))
                            .on_click(move |_, _, cx| {
                                view.update(cx, |this, cx| {
                                    cx.stop_propagation();
                                    this.confirm = Some(ConfirmAction::DeleteLocalConnection(
                                        connection_for_delete.id,
                                    ));
                                    cx.notify();
                                });
                            })
                        }),
                ),
        )
        .into_any_element()
    }

    fn reset_workspace_filters(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if let Some(state) = &mut self.state {
            state.set_search_query("");
            state.clear_tag_filter();
        }
        self.search.update(cx, |search, cx| {
            search.set_value("", window, cx);
            search.focus(window, cx);
        });
        cx.notify();
    }

    pub(in crate::app) fn render_workspace(
        &mut self,
        window: &Window,
        cx: &mut Context<Self>,
    ) -> gpui::AnyElement {
        let query = self.search.read(cx).value().to_string();
        let Some(state) = &mut self.state else {
            return div().into_any_element();
        };
        state.set_search_query(query);
        let profiles: Vec<_> = state.filtered_profiles().into_iter().cloned().collect();
        let local_connections: Vec<_> = state
            .filtered_local_connections()
            .into_iter()
            .cloned()
            .collect();
        let total_profiles = state.profiles().len();
        let filter = state.tag_filter().clone();
        let untagged_count = state
            .profiles()
            .iter()
            .filter(|profile| profile.tag_ids.is_empty())
            .count();
        let tag_rows: Vec<(TagId, String, Option<TagColor>, usize)> = state
            .tags()
            .into_iter()
            .map(|tag| {
                let count = state
                    .profiles()
                    .iter()
                    .filter(|profile| profile.tag_ids.contains(&tag.id))
                    .count();
                (tag.id, tag.name.clone(), tag.color, count)
            })
            .collect();
        let has_results = !profiles.is_empty() || !local_connections.is_empty();
        let empty_state = state.workspace_empty_state_for_results(has_results);
        let section_width = (window.viewport_size().width
            - px(200.)
            - px(CARD_GAP * 3.)
            - px(40.)
            - px(18.)
            - px(200.))
        .max(px(0.));
        let mut groups = div()
            .flex_none()
            .flex()
            .flex_wrap()
            .items_start()
            .content_start()
            .gap_x(px(12.))
            .gap_y(px(18.));
        let mut scroll_index = 0;
        if !profiles.is_empty() {
            groups = groups.child(
                ui_text(
                    TextRole::SectionTitle,
                    TextColor::Primary,
                    self.text(MessageId::SshHosts),
                    cx,
                )
                .w(section_width),
            );
            scroll_index += 1;
            for profile in profiles {
                groups = groups.child(self.render_host_card(profile, scroll_index, cx));
                scroll_index += 1;
            }
        }
        if !local_connections.is_empty() {
            groups = groups.child(
                ui_text(
                    TextRole::SectionTitle,
                    TextColor::Primary,
                    self.text(MessageId::LocalConnections),
                    cx,
                )
                .w(section_width),
            );
            scroll_index += 1;
            for connection in local_connections {
                groups = groups.child(self.render_local_card(connection, scroll_index, cx));
                scroll_index += 1;
            }
        }
        let content = if let Some(empty_state) = empty_state {
            let (message, action) = match empty_state {
                WorkspaceEmptyState::NoConnections => (MessageId::NoConnections, None),
                WorkspaceEmptyState::Search => {
                    (MessageId::NoSearchResults, Some(MessageId::ClearSearch))
                }
                WorkspaceEmptyState::TagFilter => (
                    MessageId::NoTagFilterResults,
                    Some(MessageId::ClearTagFilters),
                ),
                WorkspaceEmptyState::SearchAndTagFilter => (
                    MessageId::NoSearchAndTagResults,
                    Some(MessageId::ClearSearchAndFilters),
                ),
            };
            let empty = div()
                .id("workspace-empty-state")
                .size_full()
                .flex()
                .flex_col()
                .items_center()
                .justify_center()
                .gap(px(12.))
                .child(ui_text(
                    TextRole::Body,
                    TextColor::Secondary,
                    self.text(message),
                    cx,
                ));
            if let Some(action) = action {
                empty
                    .child(
                        button(
                            "clear-empty-results",
                            self.text(action),
                            ButtonVariant::Secondary,
                        )
                        .debug_selector(|| "clear-empty-results".into())
                        .on_click(cx.listener(|this, _, window, cx| {
                            this.reset_workspace_filters(window, cx);
                        })),
                    )
                    .into_any_element()
            } else {
                empty.into_any_element()
            }
        } else {
            groups
                .id("workspace-hosts")
                .size_full()
                .track_scroll(&self.workspace_scroll)
                .overflow_y_scroll()
                .into_any_element()
        };
        // Tag sidebar: "All hosts" clears the filter (highlighted when no
        // filter is active), "Untagged" and registry tags are mutually
        // exclusive; registry tags toggle into an OR union among themselves.
        let sidebar_border = cx.theme().border;
        let tags_label = self.text(MessageId::Tags);
        let manage_label = self.text(MessageId::ManageTags);
        let all_hosts_label = self.text(MessageId::AllHosts).to_owned();
        let untagged_label = self.text(MessageId::Untagged).to_owned();
        let mut list = div().flex().flex_col().gap(px(2.));
        let all_row = self.tag_sidebar_row(
            SharedString::from("filter-all-hosts"),
            !filter.is_active(),
            None,
            all_hosts_label,
            total_profiles,
            cx,
        );
        list = list.child(all_row.on_click(cx.listener(|this, _, _, cx| {
            if let Some(state) = &mut this.state {
                state.clear_tag_filter();
            }
            cx.notify();
        })));
        let untagged_row = self.tag_sidebar_row(
            SharedString::from("filter-untagged"),
            filter.untagged,
            None,
            untagged_label,
            untagged_count,
            cx,
        );
        list = list.child(untagged_row.on_click(cx.listener(|this, _, _, cx| {
            if let Some(state) = &mut this.state {
                state.toggle_untagged_filter();
            }
            cx.notify();
        })));
        for (id, name, color, count) in tag_rows {
            let row = self.tag_sidebar_row(
                SharedString::from(format!("filter-tag-{}", id.0)),
                filter.tags.contains(&id),
                color.map(tag_color_rgb),
                name,
                count,
                cx,
            );
            list = list.child(row.on_click(cx.listener(move |this, _, _, cx| {
                if let Some(state) = &mut this.state {
                    state.toggle_tag_filter(id);
                }
                cx.notify();
            })));
        }
        let sidebar = div()
            .id("tag-sidebar")
            .w(px(200.))
            .h_full()
            .flex_none()
            .flex()
            .flex_col()
            .gap(px(6.))
            .pr(px(14.))
            .border_r_1()
            .border_color(sidebar_border)
            .child(
                h_flex()
                    .px(px(10.))
                    .py(px(4.))
                    .items_center()
                    .justify_between()
                    .child(ui_text(
                        TextRole::SectionTitle,
                        TextColor::Primary,
                        tags_label,
                        cx,
                    ))
                    .child(
                        icon_button(
                            "manage-tags",
                            IconName::Settings2,
                            ButtonVariant::Tertiary,
                            ButtonSize::SmallIcon,
                        )
                        .tooltip(manage_label)
                        .on_click(
                            cx.listener(|this, _, window, cx| this.open_tag_manager(window, cx)),
                        ),
                    ),
            )
            .child(list);
        div()
            .size_full()
            .flex()
            .gap(px(18.))
            .p(px(20.))
            .child(sidebar)
            .child(
                div()
                    .flex_1()
                    .min_w(px(0.))
                    .min_h(px(0.))
                    .flex()
                    .flex_col()
                    .gap(px(18.))
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .justify_between()
                            .gap(px(12.))
                            .child(
                                h_flex().flex_none().child(
                                    button(
                                        "add-connection",
                                        self.text(MessageId::Add),
                                        ButtonVariant::Primary,
                                    )
                                    .icon(IconName::Plus)
                                    .debug_selector(|| "add-connection".into())
                                    .on_click(
                                        cx.listener(|this, _, _, cx| this.open_discovery(cx)),
                                    ),
                                ),
                            )
                            .child(
                                div()
                                    .id("workspace-search")
                                    .flex_1()
                                    .min_w(px(0.))
                                    .max_w(px(280.))
                                    .debug_selector(|| "workspace-search".into())
                                    .child(
                                        Input::new(&self.search)
                                            .prefix(Icon::new(IconName::Search)),
                                    ),
                            ),
                    )
                    .child(div().flex_1().min_w(px(0.)).min_h(px(0.)).child(content)),
            )
            .into_any_element()
    }

    /// One sidebar filter row: optional color dot, label, and host count.
    /// Selection styling mirrors the app navigation items.
    fn tag_sidebar_row(
        &self,
        row_id: SharedString,
        selected: bool,
        dot: Option<u32>,
        label: String,
        count: usize,
        cx: &App,
    ) -> gpui_component::button::Button {
        let theme = cx.theme();
        let accent = theme.accent;
        let accent_foreground = theme.accent_foreground;
        let muted_foreground = theme.muted_foreground;
        let hover_bg = theme.secondary;
        let count_color = if selected {
            accent_foreground
        } else {
            muted_foreground
        };
        let row = TextRole::Body
            .apply(
                custom_button(row_id, ButtonVariant::Tertiary)
                    .w_full()
                    .items_center()
                    .gap(px(8.))
                    .px(px(10.))
                    .py(px(6.))
                    .rounded(px(8.)),
            )
            .cursor_pointer()
            .child(match dot {
                Some(color) => div()
                    .size(px(8.))
                    .flex_none()
                    .rounded_full()
                    .bg(rgb(color))
                    .into_any_element(),
                None => div().size(px(8.)).flex_none().into_any_element(),
            })
            .child(div().flex_1().truncate().child(label))
            .when(selected, |row| {
                row.child(Icon::new(IconName::Check).size(px(14.)).flex_none())
                    .child(self.text(MessageId::Selected))
            })
            .child(
                TextRole::Caption
                    .apply(div())
                    .text_color(count_color)
                    .child(format!("{count}")),
            );
        if selected {
            row.selected(true).bg(accent).text_color(accent_foreground)
        } else {
            row.with_variant(gpui_component::button::ButtonVariant::Custom(
                gpui_component::button::ButtonCustomVariant::new(cx)
                    .hover(hover_bg)
                    .active(hover_bg),
            ))
        }
    }

    fn profile_status(&self, profile_id: ProfileId) -> Option<(TextColor, &'static str)> {
        let tab = self.tabs.tabs().iter().rev().find(|tab| {
            tab.profile()
                .is_some_and(|profile| profile.id == profile_id)
        })?;
        let color = match tab.state() {
            TabState::Connected => TextColor::Accent,
            TabState::Disconnected { .. } => TextColor::Danger,
            TabState::AwaitingHostKey | TabState::AwaitingSecret | TabState::Connecting => {
                TextColor::Warning
            }
        };
        Some((color, self.text(tab_state_message_id(tab.state()))))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::state::TagFilter;

    #[gpui::test]
    async fn empty_result_recovery_restores_results_and_focus(cx: &mut gpui::TestAppContext) {
        use tempfile::tempdir;

        cx.update(gpui_component::init);
        let directory = tempdir().unwrap();
        let store =
            oxide_ssh_core::storage::AppStore::open(directory.path().to_path_buf()).unwrap();
        let tag = Tag::new("Production");
        let empty_tag = Tag::new("Unused");
        let profile = ConnectionProfile {
            id: oxide_ssh_core::model::ProfileId::new(),
            name: "Web".into(),
            endpoint: Endpoint {
                host: "web.example".into(),
                port: 22,
            },
            username: "oxide".into(),
            auth: AuthConfig::Agent,
            tag_ids: vec![tag.id],
            startup_command: None,
            ssh_keepalive_interval_seconds: None,
            ssh_keepalive_max_count: None,
            terminal_font_size: None,
            ansi_palette: None,
            environment_variables: Vec::new(),
        };
        let mut config = store.load_config().unwrap();
        config.tags = vec![tag.clone(), empty_tag.clone()];
        config.profiles = vec![profile];
        store.save_config(&config).unwrap();

        let root = directory.path().to_path_buf();
        let mut app_view = None;
        let (_root, cx) = cx.add_window_view(|window, cx| {
            let view = cx.new(|cx| AppView::new(root.clone(), AppState::load(root), window, cx));
            app_view = Some(view.clone());
            Root::new(view, window, cx)
        });
        let view = app_view.unwrap();

        let activate_clear = |cx: &mut gpui::VisualTestContext, expected: WorkspaceEmptyState| {
            cx.run_until_parked();
            assert_eq!(
                cx.read(|app| {
                    view.read(app)
                        .state
                        .as_ref()
                        .unwrap()
                        .workspace_empty_state()
                }),
                Some(expected)
            );
            let clear = cx.debug_bounds("clear-empty-results").unwrap();
            cx.simulate_click(clear.center(), gpui::Modifiers::default());
            cx.run_until_parked();

            assert_eq!(
                cx.read(|app| {
                    let state = view.read(app).state.as_ref().unwrap();
                    (state.filtered_profiles().len(), state.tag_filter().clone())
                }),
                (1, TagFilter::default())
            );
            cx.update(|window, app| {
                let search = view.read(app).search.clone();
                assert!(search.read(app).value().is_empty());
                assert!(search.read(app).focus_handle(app).is_focused(window));
            });
            assert_eq!(store.load_config().unwrap(), config);
        };

        cx.update(|window, app| {
            let search = view.read(app).search.clone();
            search.update(app, |search, cx| search.set_value("missing", window, cx));
            view.update(app, |_, cx| cx.notify());
        });
        activate_clear(cx, WorkspaceEmptyState::Search);

        view.update(cx, |app, cx| {
            app.state.as_mut().unwrap().toggle_tag_filter(empty_tag.id);
            cx.notify();
        });
        activate_clear(cx, WorkspaceEmptyState::TagFilter);

        view.update(cx, |app, cx| {
            app.state.as_mut().unwrap().toggle_tag_filter(empty_tag.id);
            cx.notify();
        });
        cx.update(|window, app| {
            let search = view.read(app).search.clone();
            search.update(app, |search, cx| search.set_value("missing", window, cx));
            view.update(app, |_, cx| cx.notify());
        });
        activate_clear(cx, WorkspaceEmptyState::SearchAndTagFilter);
    }

    #[gpui::test]
    async fn complete_connection_and_tag_values_are_recoverable_from_independent_actions(
        cx: &mut gpui::TestAppContext,
    ) {
        cx.update(gpui_component::init);
        let directory = tempfile::tempdir().unwrap();
        let root = directory.path().to_path_buf();
        let store = oxide_ssh_core::storage::AppStore::open(root.clone()).unwrap();
        store
            .update_settings(|settings| settings.locale = LocaleSetting::EnUs)
            .unwrap();
        let profile_name = "production-shared-prefix-东亚-development";
        let username = "deploy-user";
        let host = "production-shared-prefix.internal.example";
        let local_name = "Ubuntu shared prefix 开发环境";
        let distribution = "Ubuntu-24.04-shared-prefix-development";
        let tag_name = "Production shared prefix 生产环境";
        let profile = ConnectionProfile {
            id: ProfileId::new(),
            name: profile_name.into(),
            endpoint: Endpoint {
                host: host.into(),
                port: 2222,
            },
            username: username.into(),
            auth: AuthConfig::Agent,
            tag_ids: Vec::new(),
            startup_command: None,
            ssh_keepalive_interval_seconds: None,
            ssh_keepalive_max_count: None,
            terminal_font_size: None,
            ansi_palette: None,
            environment_variables: Vec::new(),
        };
        let mut config = store.load_config().unwrap();
        config.profiles = vec![profile];
        config.local_connections = vec![LocalConnection::new(local_name, distribution)];
        config.tags = vec![Tag::new(tag_name)];
        store.save_config(&config).unwrap();

        let mut app_view = None;
        let (_root, cx) = cx.add_window_view(|window, cx| {
            let view = cx.new(|cx| AppView::new(root.clone(), AppState::load(root), window, cx));
            app_view = Some(view.clone());
            Root::new(view, window, cx)
        });
        let view = app_view.unwrap();
        cx.simulate_window_resize(cx.windows()[0], size(px(800.), px(520.)));
        cx.run_until_parked();

        cx.update(|window, app| {
            view.update(app, |app, cx| app.open_tag_manager(window, cx));
            assert_eq!(
                view.read(app)
                    .tag_manager
                    .as_ref()
                    .map(|manager| manager.entries[0].name.read(app).value().to_string())
                    .as_deref(),
                Some(tag_name)
            );
            view.update(app, |app, _| app.tag_manager = None);
        });

        cx.update(|window, app| {
            view.update(app, |app, cx| {
                app.show_edit_connection(config.profiles[0].clone(), window, cx)
            });
            assert_eq!(
                {
                    let app_view = view.read(app);
                    let editor = app_view.editor.as_ref().unwrap();
                    (
                        editor.name.read(app).value().to_string(),
                        editor.username.read(app).value().to_string(),
                        editor.host.read(app).value().to_string(),
                        editor.port.read(app).value().to_string(),
                    )
                },
                (
                    profile_name.into(),
                    username.into(),
                    host.into(),
                    "2222".into()
                )
            );
            view.update(app, |app, _| app.editor = None);
        });

        cx.update(|window, app| {
            view.update(app, |app, cx| {
                app.start_local_rename(config.local_connections[0].clone(), window, cx)
            });
            assert_eq!(
                {
                    let app_view = view.read(app);
                    let dialog = app_view.local_rename.as_ref().unwrap();
                    (
                        dialog.name.read(app).value().to_string(),
                        dialog.distribution.clone(),
                    )
                },
                (local_name.into(), distribution.into())
            );
            assert!({
                let app = view.read(app);
                app.tabs.tabs().is_empty() && app.main_view == MainView::Workspace
            });
            view.update(app, |app, _| app.local_rename = None);
        });
    }

    #[gpui::test]
    async fn overflowing_workspace_actions_scroll_into_view(cx: &mut gpui::TestAppContext) {
        cx.update(gpui_component::init);
        let directory = tempfile::tempdir().unwrap();
        let root = directory.path().to_path_buf();
        let store = oxide_ssh_core::storage::AppStore::open(root.clone()).unwrap();
        let mut config = store.load_config().unwrap();
        config.local_connections = (0..24)
            .map(|index| LocalConnection::new(format!("Local {index}"), format!("Ubuntu-{index}")))
            .collect();
        store.save_config(&config).unwrap();
        let mut app_view = None;
        let (_root, cx) = cx.add_window_view(|window, cx| {
            let view = cx.new(|cx| AppView::new(root.clone(), AppState::load(root), window, cx));
            app_view = Some(view.clone());
            Root::new(view, window, cx)
        });
        let view = app_view.unwrap();
        cx.simulate_window_resize(cx.windows()[0], size(px(800.), px(520.)));
        cx.run_until_parked();
        assert!(cx.read(|app| view.read(app).workspace_scroll.max_offset().height > px(0.)));

        cx.update(|window, app| view.read(app).focus_handle.clone().focus(window));
        for _ in 0..81 {
            cx.update(|window, _| {
                window.focus_next();
                window.refresh();
            });
            cx.run_until_parked();
        }
        let assert_last_visible = |cx: &mut gpui::VisualTestContext| {
            let (item, offset, viewport) = cx.read(|app| {
                let scroll = &view.read(app).workspace_scroll;
                (
                    scroll.bounds_for_item(24).unwrap(),
                    scroll.offset(),
                    scroll.bounds(),
                )
            });
            assert!(item.size.height < px(100.), "stretched card: {item:?}");
            assert!(item.top() + offset.y >= viewport.top());
            assert!(item.bottom() + offset.y <= viewport.bottom());
        };
        assert_last_visible(cx);

        for _ in 0..2 {
            cx.update(|window, _| {
                window.focus_prev();
                window.refresh();
            });
            cx.run_until_parked();
        }
        assert_last_visible(cx);
    }

    #[gpui::test]
    async fn workspace_header_fits_minimum_window(cx: &mut gpui::TestAppContext) {
        cx.update(gpui_component::init);
        let directory = tempfile::tempdir().unwrap();
        let root = directory.path().to_path_buf();
        // Pin English: its empty-state copy is the widest, so the geometry is
        // the same on every host locale.
        oxide_ssh_core::storage::AppStore::open(root.clone())
            .unwrap()
            .update_settings(|settings| settings.locale = LocaleSetting::EnUs)
            .unwrap();
        let (_view, cx) = cx.add_window_view(|window, cx| {
            AppView::new(root.clone(), AppState::load(root), window, cx)
        });
        cx.simulate_window_resize(cx.windows()[0], size(px(800.), px(520.)));
        cx.run_until_parked();
        let add = cx.debug_bounds("add-connection").unwrap();
        let search = cx.debug_bounds("workspace-search").unwrap();
        assert!(add.right() <= px(800.), "Add clipped: {add:?}");
        assert!(search.right() <= px(800.), "search clipped: {search:?}");
        assert!(
            search.origin.x >= add.right(),
            "overlap: {add:?} {search:?}"
        );
        assert!(search.size.width > px(80.), "search collapsed: {search:?}");
    }
}
