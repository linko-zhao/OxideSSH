use super::*;

pub(in crate::app) fn tab_state_icon(state: &TabState) -> IconName {
    match state {
        TabState::Connecting => IconName::LoaderCircle,
        TabState::AwaitingHostKey => IconName::TriangleAlert,
        TabState::AwaitingSecret => IconName::CircleUser,
        TabState::Connected => IconName::CircleCheck,
        TabState::Disconnected { .. } => IconName::CircleX,
    }
}

impl AppView {
    /// Pill-shaped tab base: the selected pill lifts one step above the
    /// strip; hovering an unselected pill shows a fainter step of the same
    /// color, so both states read consistently in light and dark mode.
    fn tab_pill(
        id: impl Into<ElementId>,
        selected: bool,
        cx: &App,
    ) -> gpui_component::button::Button {
        let theme = cx.theme();
        let base = TextRole::Body.apply(
            custom_button(id, ButtonVariant::Tertiary)
                .items_center()
                .gap(px(8.))
                .px(px(12.))
                // py(3) keeps the pill compact; the title bar adds the
                // client-wide CARD_GAP margin around it.
                .py(px(3.))
                .border_1()
                .border_color(theme.border)
                .rounded(theme.radius)
                .cursor_pointer(),
        );
        if selected {
            TextColor::Primary.apply(base.selected(true).bg(theme.secondary), cx)
        } else {
            let hover_bg = theme.secondary.opacity(0.55);
            TextColor::Secondary.apply(
                base.with_variant(gpui_component::button::ButtonVariant::Custom(
                    gpui_component::button::ButtonCustomVariant::new(cx)
                        .foreground(theme.muted_foreground)
                        .hover(hover_bg)
                        .active(hover_bg),
                )),
                cx,
            )
        }
    }

    /// Permanent top tab strip: the always-present Workspace pill comes
    /// first; each open connection follows as its own closable pill.
    #[cfg_attr(not(target_os = "windows"), allow(unused_variables))]
    pub(in crate::app) fn render_tab_strip(
        &mut self,
        window: &Window,
        cx: &mut Context<Self>,
    ) -> gpui::AnyElement {
        let active = self.tabs.active();
        let tab_data: Vec<_> = self
            .tabs
            .tabs()
            .iter()
            .map(|tab| {
                let title = match tab.kind() {
                    TabKind::Ssh(profile) => profile.name.clone(),
                    TabKind::Local => self.text(MessageId::LocalTerminal).to_string(),
                };
                (tab.id(), title, *tab.state())
            })
            .collect();
        let theme = cx.theme();
        let semantic = SemanticColors::global(cx);
        let state_color = |state: &TabState| match state {
            TabState::Connected => semantic.primary,
            TabState::Disconnected { .. } => semantic.danger,
            TabState::AwaitingHostKey | TabState::AwaitingSecret | TabState::Connecting => {
                semantic.warning
            }
        };
        let workspace_selected = match self.main_view {
            MainView::Workspace => true,
            // Sessions with no open connections renders the workspace.
            MainView::Sessions => tab_data.is_empty(),
            MainView::Settings => false,
        };
        let mut strip = h_flex()
            .flex_shrink()
            .min_w(px(0.))
            .items_center()
            .gap(px(4.))
            .child(
                Self::tab_pill("tab-workspace", workspace_selected, cx)
                    .child(Icon::new(IconName::LayoutDashboard).size(px(14.)))
                    .child(self.text(MessageId::Workspace))
                    .on_click(cx.listener(|this, _, _, cx| {
                        this.main_view = MainView::Workspace;
                        cx.notify();
                    })),
            );
        // Fixed separator between the permanent workspace pill and the
        // session area (connection pills + new-tab button).
        strip = strip.child(
            TextRole::Body
                .apply(div().text_color(theme.muted_foreground.opacity(0.6)))
                .child("|"),
        );
        // Only the session pills overflow; Workspace, the new-tab button and
        // the native caption controls stay permanently visible.
        let mut sessions = div()
            .id("session-tabs-scroll")
            .flex_shrink()
            .min_w(px(0.))
            .flex()
            .items_center()
            .gap(px(4.))
            .pr(px(4.))
            .track_scroll(&self.session_tabs_scroll)
            .overflow_x_scroll();
        for (index, (id, name, state)) in tab_data.iter().enumerate() {
            let tab_id = *id;
            let session_scroll = self.session_tabs_scroll.clone();
            let close_scroll = self.session_tabs_scroll.clone();
            let selected = self.main_view == MainView::Sessions && active == Some(tab_id);
            let title_truncated = {
                let text_style = window.text_style();
                let text = SharedString::from(name.clone());
                let run = TextRun {
                    len: text.len(),
                    font: text_style.font(),
                    color: text_style.color,
                    background_color: None,
                    underline: None,
                    strikethrough: None,
                };
                window
                    .text_system()
                    .shape_line(text, cx.theme().font_size * 0.875, &[run], None)
                    .width
                    > px(180.)
            };
            sessions = sessions.child(
                Self::tab_pill(SharedString::from(format!("tab-{tab_id:?}")), selected, cx)
                    .debug_selector(|| "session-tab".into())
                    .child(
                        Icon::new(tab_state_icon(state))
                            .size(px(12.))
                            .flex_none()
                            .text_color(state_color(state)),
                    )
                    .child(
                        div()
                            .truncate()
                            .max_w(px(180.))
                            .child(SharedString::from(name.clone())),
                    )
                    .child(
                        ui_text(
                            TextRole::Caption,
                            match state {
                                TabState::Connected => TextColor::Accent,
                                TabState::Disconnected { .. } => TextColor::Danger,
                                TabState::AwaitingHostKey
                                | TabState::AwaitingSecret
                                | TabState::Connecting => TextColor::Warning,
                            },
                            self.text(tab_state_message_id(state)),
                            cx,
                        )
                        .whitespace_nowrap(),
                    )
                    .when(title_truncated, |tab| {
                        let title_for_click = name.clone();
                        let details_scroll = self.session_tabs_scroll.clone();
                        tab.child(
                            icon_button(
                                SharedString::from(format!("details-tab-{tab_id:?}")),
                                IconName::Info,
                                ButtonVariant::Tertiary,
                                ButtonSize::SmallIcon,
                            )
                            .flex_none()
                            .debug_selector(|| "tab-details-button".into())
                            .tooltip(format!("{}\n{}", self.text(MessageId::SessionTitle), name))
                            .on_focus(move |_, _| details_scroll.scroll_to_item(index))
                            .on_click(cx.listener(
                                move |this, _, _, cx| {
                                    cx.stop_propagation();
                                    this.session_title_details = Some(title_for_click.clone());
                                    cx.notify();
                                },
                            )),
                        )
                    })
                    .child(
                        // Keep a 24px target without inflating the pill;
                        // the default medium icon button is 32px.
                        icon_button(
                            SharedString::from(format!("close-{tab_id:?}")),
                            IconName::Close,
                            ButtonVariant::Tertiary,
                            ButtonSize::SmallIcon,
                        )
                        .flex_none()
                        .tooltip(self.text(MessageId::CloseTab))
                        .debug_selector(|| "tab-close-button".into())
                        .on_focus(move |_, _| close_scroll.scroll_to_item(index))
                        .on_click(cx.listener(move |this, _, _, cx| {
                            cx.stop_propagation();
                            this.close_tab(tab_id, cx);
                        })),
                    )
                    .on_focus(move |_, _| session_scroll.scroll_to_item(index))
                    .on_click(cx.listener(move |this, _, _, cx| {
                        this.tabs.set_active(tab_id);
                        this.main_view = MainView::Sessions;
                        cx.notify();
                    })),
            );
        }
        let strip = strip.child(sessions).child(
            icon_button(
                "new-tab",
                IconName::Plus,
                ButtonVariant::Tertiary,
                ButtonSize::SmallIcon,
            )
            .tooltip(self.text(MessageId::NewTabButton))
            .debug_selector(|| "new-tab-button".into())
            .on_click(cx.listener(|this, _, window, cx| {
                let open_local = this.state.as_ref().is_some_and(|state| {
                    state.settings().new_tab_action == NewTabAction::LocalTerminal
                });
                if open_local {
                    this.connect_local(window, cx);
                } else {
                    this.show_add_connection(window, cx);
                }
            })),
        );
        // Linux keeps the native title bar, so the strip stays a standalone row.
        #[cfg(target_os = "linux")]
        {
            div()
                .flex_none()
                .px(px(CARD_GAP))
                .py(px(CARD_GAP))
                .border_b_1()
                .border_color(cx.theme().border)
                .child(strip)
                .into_any_element()
        }
        // macOS: the strip merges into the custom-drawn title bar; TitleBar
        // renders the native window controls itself. The padding both matches
        // the CARD_GAP margins around the content below and gives the traffic
        // lights enough height to avoid clipping.
        #[cfg(target_os = "macos")]
        {
            // gpui-component's drag bar refuses to shrink (flex_shrink_0) and
            // takes its min-content from its children, so the strip needs a
            // definite width or overflowing session pills push the new-tab
            // button offscreen. 80px is TitleBar's private traffic-light inset.
            let width = (window.viewport_size().width - px(80.)).max(px(0.));
            TitleBar::new()
                .w_full()
                .child(div().w(width).py(px(CARD_GAP)).child(strip))
                .into_any_element()
        }
        // Windows: route caption interactions through gpui's native
        // WM_NCHITTEST path (`window_control_area`): the spacer reports
        // HTCAPTION (native move, drag-to-restore-when-maximized,
        // double-click toggle, snap layouts) and the caption buttons report
        // HTMINBUTTON/HTMAXBUTTON/HTCLOSE, with gpui's nc_button_pressed
        // handling performing minimize / maximize-toggle / close. Close
        // funnels through WM_CLOSE → `on_window_should_close`, keeping the
        // quit confirmation. The event-driven alternatives are dead ends in
        // gpui 0.2.2: `start_window_move()` is a no-op outside Wayland/X11
        // and `zoom_window()` only ever maximizes (SW_MAXIMIZE).
        //
        // Every control hitbox MUST `occlude()`: the NC mouse down is also
        // forwarded into the app, and the root view's focus-transfer
        // listener would otherwise call `prevent_default()`, making gpui
        // treat the click as app-handled and skip the native caption
        // behavior entirely.
        #[cfg(target_os = "windows")]
        {
            let theme = cx.theme();
            let (fg, hover_bg, danger, danger_fg) = (
                theme.foreground,
                theme.secondary_hover,
                theme.danger,
                theme.danger_foreground,
            );
            let caption =
                move |id: &'static str, icon: IconName, area: WindowControlArea, is_close: bool| {
                    div()
                        .id(id)
                        .window_control_area(area)
                        .occlude()
                        .debug_selector(|| id.into())
                        .flex_none()
                        .flex()
                        .w(px(34.))
                        .h_full()
                        .items_center()
                        .justify_center()
                        .text_color(fg)
                        .hover(move |s| {
                            if is_close {
                                s.bg(danger).text_color(danger_fg)
                            } else {
                                s.bg(hover_bg)
                            }
                        })
                        .child(Icon::new(icon).size(px(14.)))
                };
            let max_icon = if window.is_maximized() {
                IconName::WindowRestore
            } else {
                IconName::WindowMaximize
            };
            h_flex()
                .w_full()
                // Height derives from the pill plus the client-wide CARD_GAP
                // margin, so the tabs' top gap matches the content padding.
                .py(px(CARD_GAP))
                .flex_none()
                .items_center()
                .pl(px(CARD_GAP))
                .border_b_1()
                .border_color(theme.border)
                .child(strip)
                .child(
                    div()
                        .id("titlebar-drag")
                        .window_control_area(WindowControlArea::Drag)
                        .occlude()
                        .flex_1()
                        .h_full(),
                )
                .child(caption(
                    "win-min",
                    IconName::WindowMinimize,
                    WindowControlArea::Min,
                    false,
                ))
                .child(caption("win-max", max_icon, WindowControlArea::Max, false))
                .child(caption(
                    "win-close",
                    IconName::WindowClose,
                    WindowControlArea::Close,
                    true,
                ))
                .into_any_element()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use gpui_component::IconNamed as _;
    use oxide_ssh_terminal::TerminalColors;

    fn profile(name: &str) -> ConnectionProfile {
        ConnectionProfile {
            id: ProfileId::new(),
            name: name.into(),
            endpoint: Endpoint {
                host: "production.internal.example".into(),
                port: 22,
            },
            username: "deploy".into(),
            auth: AuthConfig::Agent,
            tag_ids: Vec::new(),
            startup_command: None,
            ssh_keepalive_interval_seconds: None,
            ssh_keepalive_max_count: None,
            terminal_font_size: None,
            ansi_palette: None,
            environment_variables: Vec::new(),
        }
    }

    #[test]
    fn every_tab_state_has_a_distinct_icon_and_localized_text() {
        let states = [
            (
                TabState::Connecting,
                "icons/loader-circle.svg",
                MessageId::Connecting,
            ),
            (
                TabState::AwaitingHostKey,
                "icons/triangle-alert.svg",
                MessageId::AwaitingHostKey,
            ),
            (
                TabState::AwaitingSecret,
                "icons/circle-user.svg",
                MessageId::AwaitingSecret,
            ),
            (
                TabState::Connected,
                "icons/circle-check.svg",
                MessageId::Connected,
            ),
            (
                TabState::Disconnected {
                    reason: DisconnectReason::Closed,
                },
                "icons/circle-x.svg",
                MessageId::Disconnected,
            ),
        ];

        for (state, icon_path, expected_message) in states {
            assert_eq!(tab_state_icon(&state).path(), icon_path);
            let message = tab_state_message_id(&state);
            assert_eq!(message, expected_message);
            for locale in [ResolvedLocale::EnUs, ResolvedLocale::ZhCn] {
                assert!(!Catalog::text(locale, message).is_empty());
            }
        }
    }

    #[gpui::test]
    async fn selecting_a_session_tab_preserves_its_connection_state(cx: &mut gpui::TestAppContext) {
        cx.update(gpui_component::init);
        let directory = tempfile::tempdir().unwrap();
        let root = directory.path().to_path_buf();
        let mut app_view = None;
        let (_root, cx) = cx.add_window_view(|window, cx| {
            let view = cx.new(|cx| AppView::new(root.clone(), AppState::load(root), window, cx));
            app_view = Some(view.clone());
            Root::new(view, window, cx)
        });
        let view = app_view.unwrap();
        let tab_id = view.update(cx, |app, cx| {
            let tab_id = app.tabs.open_local(
                TerminalSize {
                    columns: 80,
                    rows: 24,
                    pixel_width: 640,
                    pixel_height: 384,
                },
                TerminalColors::default(),
            );
            app.main_view = MainView::Workspace;
            cx.notify();
            tab_id
        });
        cx.run_until_parked();

        let tab = cx.debug_bounds("session-tab").unwrap();
        cx.simulate_click(tab.center(), gpui::Modifiers::default());

        assert!(cx.read(|app| {
            let app = view.read(app);
            app.main_view == MainView::Sessions
                && app.tabs.active() == Some(tab_id)
                && app.tabs.tab(tab_id).unwrap().state() == &TabState::Connecting
        }));
    }

    #[gpui::test]
    async fn short_session_title_adds_no_recovery_chrome(cx: &mut gpui::TestAppContext) {
        cx.update(gpui_component::init);
        let directory = tempfile::tempdir().unwrap();
        let root = directory.path().to_path_buf();
        let profile = profile("Prod");
        let store = oxide_ssh_core::storage::AppStore::open(root.clone()).unwrap();
        let mut config = store.load_config().unwrap();
        config.profiles = vec![profile.clone()];
        store.save_config(&config).unwrap();
        let mut app_view = None;
        let (_root, cx) = cx.add_window_view(|window, cx| {
            let view = cx.new(|cx| AppView::new(root.clone(), AppState::load(root), window, cx));
            app_view = Some(view.clone());
            Root::new(view, window, cx)
        });
        let view = app_view.unwrap();
        view.update(cx, |app, cx| {
            app.tabs.open(
                profile,
                TerminalSize {
                    columns: 80,
                    rows: 24,
                    pixel_width: 640,
                    pixel_height: 384,
                },
                TerminalColors::default(),
                false,
            );
            cx.notify();
        });
        cx.run_until_parked();

        assert!(cx.debug_bounds("tab-details-button").is_none());
    }

    #[gpui::test]
    async fn tab_icon_targets_are_24px_and_non_overlapping(cx: &mut gpui::TestAppContext) {
        cx.update(gpui_component::init);
        let directory = tempfile::tempdir().unwrap();
        let root = directory.path().to_path_buf();
        let mut app_view = None;
        let (_root, cx) = cx.add_window_view(|window, cx| {
            let view = cx.new(|cx| AppView::new(root.clone(), AppState::load(root), window, cx));
            app_view = Some(view.clone());
            Root::new(view, window, cx)
        });
        let view = app_view.unwrap();
        cx.run_until_parked();
        let new_tab = cx.debug_bounds("new-tab-button").unwrap();
        assert!(new_tab.size.width >= px(24.));
        assert!(new_tab.size.height >= px(24.));

        view.update(cx, |app, cx| {
            app.state
                .as_mut()
                .unwrap()
                .set_confirm_before_close_with_sessions(false)
                .unwrap();
            app.tabs.open_local(
                TerminalSize {
                    columns: 80,
                    rows: 24,
                    pixel_width: 640,
                    pixel_height: 384,
                },
                TerminalColors::default(),
            );
            cx.notify();
        });
        cx.run_until_parked();

        let close = cx.debug_bounds("tab-close-button").unwrap();
        assert!(close.size.width >= px(24.));
        assert!(close.size.height >= px(24.));
        let new_tab = cx.debug_bounds("new-tab-button").unwrap();
        assert!(!close.intersects(&new_tab));

        view.update(cx, |app, cx| {
            app.main_view = MainView::Workspace;
            cx.notify();
        });
        cx.run_until_parked();
        cx.simulate_click(
            point(close.origin.x + px(1.), close.center().y),
            gpui::Modifiers::default(),
        );
        assert!(cx.read(|app| {
            let tabs = &view.read(app).tabs;
            tabs.tabs().is_empty() || tabs.tabs()[0].closing()
        }));
        assert_eq!(cx.read(|app| view.read(app).main_view), MainView::Workspace);
    }

    #[gpui::test]
    async fn session_tabs_overflow_without_hiding_permanent_controls(
        cx: &mut gpui::TestAppContext,
    ) {
        cx.update(gpui_component::init);
        let directory = tempfile::tempdir().unwrap();
        let root = directory.path().to_path_buf();
        let mut app_view = None;
        let (_window, cx) = cx.add_window_view(|window, cx| {
            let view = cx.new(|cx| AppView::new(root.clone(), AppState::load(root), window, cx));
            app_view = Some(view.clone());
            Root::new(view, window, cx)
        });
        let view = app_view.unwrap();
        view.update(cx, |app, cx| {
            for _ in 0..12 {
                app.tabs.open_local(
                    TerminalSize {
                        columns: 80,
                        rows: 24,
                        pixel_width: 640,
                        pixel_height: 384,
                    },
                    TerminalColors::default(),
                );
            }
            cx.notify();
        });
        for window_size in [size(px(1080.), px(720.)), size(px(800.), px(520.))] {
            cx.simulate_window_resize(cx.windows()[0], window_size);
            cx.run_until_parked();

            let new_tab = cx.debug_bounds("new-tab-button").unwrap();
            assert!(
                new_tab.right() <= window_size.width,
                "new tab clipped: {new_tab:?}"
            );
            let close = cx.debug_bounds("tab-close-button").unwrap();
            assert!(
                !close.intersects(&new_tab),
                "tab close overlaps new tab: {close:?} vs {new_tab:?}"
            );
            #[cfg(target_os = "windows")]
            for selector in ["win-min", "win-max", "win-close"] {
                let bounds = cx.debug_bounds(selector).unwrap();
                assert!(
                    bounds.right() <= window_size.width,
                    "{selector} clipped: {bounds:?}"
                );
            }
        }

        cx.update(|window, app| {
            view.read(app).focus_handle.clone().focus(window);
            for _ in 0..24 {
                window.focus_next();
            }
        });
        cx.run_until_parked();
        let assert_visible = |cx: &mut gpui::VisualTestContext, index| {
            let (item, offset, viewport) = cx.read(|app| {
                let scroll = &view.read(app).session_tabs_scroll;
                (
                    scroll.bounds_for_item(index).unwrap(),
                    scroll.offset(),
                    scroll.bounds(),
                )
            });
            assert!(item.left() + offset.x >= viewport.left());
            assert!(item.right() + offset.x <= viewport.right());
        };
        assert_visible(cx, 11);
        cx.update(|window, _| {
            window.focus_next();
            window.focus_prev();
        });
        cx.run_until_parked();
        assert_visible(cx, 11);
        cx.update(|window, _| {
            window.focus_prev();
            window.focus_prev();
            window.focus_prev();
        });
        cx.run_until_parked();
        assert_visible(cx, 10);
    }
}
