use super::dialog::ConfirmAction;
use super::messages::{disconnect_reason_message_id, tab_state_message_id};
use super::terminal::{RgbColorExt, TERMINAL_FONT, TerminalElement};
use super::*;

mod settings;
mod sidebar;
mod tab_strip;
mod workspace;

impl AppView {
    pub(in crate::app) fn render_recovery(&self, cx: &mut Context<Self>) -> gpui::AnyElement {
        div()
            .size_full()
            .flex()
            .items_center()
            .justify_center()
            .child(
                div()
                    .w(px(560.))
                    .flex()
                    .flex_col()
                    .gap(px(16.))
                    .child(
                        Alert::error("recovery-alert", self.text(MessageId::StorageCorrupt))
                            .title("OxideSSH"),
                    )
                    .child(
                        div()
                            .text_sm()
                            .text_color(cx.theme().muted_foreground)
                            .child(self.recovery_error.clone().unwrap_or_default()),
                    )
                    .child(
                        div()
                            .text_sm()
                            .text_color(cx.theme().muted_foreground)
                            .child(self.root.to_string_lossy().into_owned()),
                    )
                    .child(
                        div()
                            .flex()
                            .gap(px(10.))
                            .child(
                                Button::new("recovery-open")
                                    .label(self.text(MessageId::OpenConfigDirectory))
                                    .on_click(
                                        cx.listener(|this, _, _, cx| {
                                            this.open_config_directory(cx)
                                        }),
                                    ),
                            )
                            .child(
                                Button::new("recovery-retry")
                                    .label(self.text(MessageId::Retry))
                                    .primary()
                                    .on_click(cx.listener(|this, _, window, cx| {
                                        this.retry_storage(window, cx)
                                    })),
                            ),
                    ),
            )
            .into_any_element()
    }

    pub(in crate::app) fn render_sessions(&mut self, cx: &mut Context<Self>) -> gpui::AnyElement {
        self.render_active_tab(cx)
    }

    fn render_active_tab(&mut self, cx: &mut Context<Self>) -> gpui::AnyElement {
        let Some(tab_id) = self.tabs.active() else {
            return div().into_any_element();
        };
        let state = self.tabs.tab(tab_id).map(|tab| *tab.state());
        let profile_font_size = self
            .tabs
            .tab(tab_id)
            .and_then(|tab| tab.profile())
            .and_then(|profile| profile.terminal_font_size);
        let font_size = self
            .state
            .as_ref()
            .map(|state| state.settings().terminal_font_size as f32)
            .unwrap_or(13.);
        let font_size = profile_font_size.map(f32::from).unwrap_or(font_size);
        match state {
            Some(TabState::Connecting)
            | Some(TabState::Connected)
            | Some(TabState::Disconnected { .. }) => {
                let disconnected = matches!(state, Some(TabState::Disconnected { .. }));
                div()
                    .id("terminal")
                    .size_full()
                    .overflow_hidden()
                    .relative()
                    .rounded(cx.theme().radius_lg)
                    .border_1()
                    .border_color(cx.theme().border)
                    .p(px(CARD_PADDING))
                    .when(!disconnected, |element| element.key_context("Terminal"))
                    .bg(rgb(self.terminal_colors().background.to_hex()))
                    .text_color(rgb(self.terminal_colors().foreground.to_hex()))
                    .font_family(
                        self.state
                            .as_ref()
                            .map(|state| state.settings().terminal_font_family.clone())
                            .unwrap_or_else(|| TERMINAL_FONT.to_owned()),
                    )
                    .text_size(px(font_size))
                    .line_height(px(self
                        .state
                        .as_ref()
                        .map(|state| state.settings().terminal_line_height as f32)
                        .unwrap_or(17.)))
                    .track_focus(&self.terminal_focus)
                    .when(!disconnected, |element| {
                        element.on_key_down(cx.listener(Self::handle_key_down))
                    })
                    .on_mouse_down(MouseButton::Left, cx.listener(Self::terminal_mouse_down))
                    .on_mouse_down(MouseButton::Right, cx.listener(Self::terminal_mouse_down))
                    .on_mouse_down(MouseButton::Middle, cx.listener(Self::terminal_mouse_down))
                    .on_mouse_move(cx.listener(Self::terminal_mouse_move))
                    .on_mouse_up(MouseButton::Left, cx.listener(Self::terminal_mouse_up))
                    .on_mouse_up(MouseButton::Right, cx.listener(Self::terminal_mouse_up))
                    .on_mouse_up(MouseButton::Middle, cx.listener(Self::terminal_mouse_up))
                    .on_mouse_up_out(MouseButton::Left, cx.listener(Self::terminal_mouse_up))
                    .on_mouse_up_out(MouseButton::Right, cx.listener(Self::terminal_mouse_up))
                    .on_mouse_up_out(MouseButton::Middle, cx.listener(Self::terminal_mouse_up))
                    .on_scroll_wheel(cx.listener(Self::terminal_scroll))
                    .child(TerminalElement {
                        view: cx.entity(),
                        tab_id,
                    })
                    .when_some(
                        (!matches!(
                            state,
                            Some(TabState::Connected) | Some(TabState::Disconnected { .. })
                        ))
                        .then_some(state)
                        .flatten(),
                        |element, state| {
                            element.child(
                                div()
                                    .absolute()
                                    .inset_0()
                                    .flex()
                                    .items_center()
                                    .justify_center()
                                    .text_color(cx.theme().muted_foreground)
                                    .child(self.text(tab_state_message_id(&state))),
                            )
                        },
                    )
                    .when_some(disconnected.then_some(state), |element, state| {
                        let Some(TabState::Disconnected { reason }) = state else {
                            return element;
                        };
                        element.child(
                            div()
                                .absolute()
                                .left_0()
                                .right_0()
                                .top(px(12.))
                                .flex()
                                .justify_center()
                                .child(
                                    div()
                                        .flex()
                                        .items_center()
                                        .gap(px(10.))
                                        .px(px(14.))
                                        .py(px(8.))
                                        .rounded(px(6.))
                                        .border_1()
                                        .border_color(cx.theme().border)
                                        .bg(cx.theme().background)
                                        .text_color(cx.theme().foreground)
                                        .text_size(px(13.))
                                        .child(self.text(disconnect_reason_message_id(reason)))
                                        .child(
                                            Button::new("retry-session")
                                                .label(self.text(MessageId::Retry))
                                                .primary()
                                                .on_click(cx.listener(
                                                    move |this, _, window, cx| {
                                                        this.reconnect(tab_id, window, cx)
                                                    },
                                                )),
                                        ),
                                ),
                        )
                    })
                    .into_any_element()
            }
            Some(state) => div()
                .size_full()
                .flex()
                .items_center()
                .justify_center()
                .text_color(cx.theme().muted_foreground)
                .child(self.text(tab_state_message_id(&state)))
                .into_any_element(),
            None => div().into_any_element(),
        }
    }
}
