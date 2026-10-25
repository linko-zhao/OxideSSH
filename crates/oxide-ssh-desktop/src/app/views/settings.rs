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
            SettingsCategory::General => settings_page(
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
                ],
                cx,
            ),
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
                                            .child(algorithm),
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
}
