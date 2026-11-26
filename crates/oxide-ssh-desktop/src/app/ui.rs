use crate::state::theme::SemanticColors;
use gpui::{
    App, Div, ElementId, FontWeight, Hsla, InteractiveElement as _, IntoElement,
    ParentElement as _, ReadGlobal as _, RenderOnce, SharedString, Styled, Window, div,
    prelude::FluentBuilder as _, px, relative, rems,
};
use gpui_component::{
    ActiveTheme as _, Icon, Sizable as _,
    button::{Button, ButtonVariants as _},
    checkbox::Checkbox,
};

#[derive(IntoElement)]
pub(super) struct CheckboxFocusRing {
    id: ElementId,
    checkbox: Checkbox,
}

pub(super) fn checkbox_focus_ring(
    id: impl Into<ElementId>,
    checkbox: Checkbox,
) -> CheckboxFocusRing {
    CheckboxFocusRing {
        id: id.into(),
        checkbox,
    }
}

impl RenderOnce for CheckboxFocusRing {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        // gpui-component paints its own ring at 20% opacity; share its keyed
        // focus handle and add an opaque outline that meets WCAG 1.4.11.
        let focus_handle = window
            .use_keyed_state(self.id, cx, |_, cx| cx.focus_handle())
            .read(cx)
            .clone();
        let focused = focus_handle.is_focused(window);
        div()
            .relative()
            .track_focus(&focus_handle)
            .child(self.checkbox)
            .when(focused, |this| {
                this.child(
                    div()
                        .absolute()
                        .inset(px(-3.))
                        .rounded(px(4.))
                        .border_2()
                        .border_color(cx.theme().ring),
                )
            })
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum TextRole {
    PageTitle,
    DialogTitle,
    PanelTitle,
    SectionTitle,
    Label,
    Body,
    Button,
    Helper,
    Caption,
}

impl TextRole {
    pub(super) fn apply<T: Styled>(self, element: T) -> T {
        let (size, line_height, weight) = match self {
            Self::PageTitle => (1.875, 1.2, FontWeight::BOLD),
            Self::DialogTitle => (1.25, 1.3, FontWeight::BOLD),
            Self::PanelTitle => (1.125, 1.333, FontWeight::BOLD),
            Self::SectionTitle => (1., 1.375, FontWeight::SEMIBOLD),
            Self::Label => (0.875, 1.429, FontWeight::SEMIBOLD),
            Self::Body => (0.875, 1.571, FontWeight::NORMAL),
            Self::Button => (0.875, 1.429, FontWeight::MEDIUM),
            Self::Helper => (0.75, 1.5, FontWeight::NORMAL),
            Self::Caption => (0.75, 1.333, FontWeight::NORMAL),
        };
        element
            .text_size(rems(size))
            .line_height(relative(line_height))
            .font_weight(weight)
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum TextColor {
    Primary,
    Secondary,
    Weak,
    // GPUI Input paints placeholders directly; this keeps their role distinct from helper text.
    #[allow(dead_code)]
    Placeholder,
    Danger,
    Warning,
    Accent,
    OnAccent,
    OnSelection,
}

impl TextColor {
    pub(super) fn resolve(self, cx: &App) -> Hsla {
        let theme = cx.theme();
        let semantic = SemanticColors::global(cx);
        match self {
            Self::Primary => theme.foreground,
            Self::Secondary | Self::Weak | Self::Placeholder => theme.muted_foreground,
            Self::Danger => semantic.danger,
            Self::Warning => semantic.warning,
            Self::Accent => semantic.primary,
            Self::OnAccent => theme.primary_foreground,
            Self::OnSelection => theme.accent_foreground,
        }
    }

    pub(super) fn apply<T: Styled>(self, element: T, cx: &App) -> T {
        element.text_color(self.resolve(cx))
    }
}

pub(super) fn text(
    role: TextRole,
    color: TextColor,
    content: impl Into<SharedString>,
    cx: &App,
) -> Div {
    color.apply(role.apply(div()).child(content.into()), cx)
}

pub(super) fn helper_text(content: impl Into<SharedString>, cx: &App) -> Div {
    text(TextRole::Helper, TextColor::Secondary, content, cx)
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum ButtonVariant {
    Primary,
    Secondary,
    Tertiary,
    Danger,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum ButtonSize {
    Medium,
    Compact,
    Icon,
    SmallIcon,
}

pub(super) trait ButtonDesign: Sized {
    fn variant(self, variant: ButtonVariant) -> Self;
    fn button_size(self, size: ButtonSize) -> Self;
}

impl ButtonDesign for Button {
    fn variant(self, variant: ButtonVariant) -> Self {
        match variant {
            ButtonVariant::Primary => self.primary(),
            ButtonVariant::Secondary => self,
            ButtonVariant::Tertiary => self.ghost(),
            ButtonVariant::Danger => self.danger(),
        }
    }

    fn button_size(self, size: ButtonSize) -> Self {
        match size {
            ButtonSize::Medium => self,
            // Compact text and icon buttons are 24px high; text has 6px horizontal padding.
            ButtonSize::Compact | ButtonSize::Icon => self.small().compact(),
            ButtonSize::SmallIcon => self.compact().xsmall(),
        }
    }
}

pub(super) fn button(
    id: impl Into<ElementId>,
    label: impl Into<SharedString>,
    variant: ButtonVariant,
) -> Button {
    Button::new(id)
        .child(TextRole::Button.apply(div()).child(label.into()))
        .variant(variant)
        .button_size(ButtonSize::Medium)
}

pub(super) fn custom_button(id: impl Into<ElementId>, variant: ButtonVariant) -> Button {
    Button::new(id)
        .variant(variant)
        .button_size(ButtonSize::Medium)
}

pub(super) fn icon_button(
    id: impl Into<ElementId>,
    icon: impl Into<Icon>,
    variant: ButtonVariant,
    size: ButtonSize,
) -> Button {
    Button::new(id)
        .icon(icon)
        .variant(variant)
        .button_size(size)
}

#[cfg(test)]
mod tests {
    use super::*;
    use gpui::{AppContext as _, Context, Modifiers, Render, point, px};
    use gpui_component::{Disableable as _, IconName, Root};

    struct ButtonTestView;

    impl Render for ButtonTestView {
        fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
            div()
                .flex()
                .flex_col()
                .items_start()
                .child(
                    custom_button("compact", ButtonVariant::Secondary)
                        .button_size(ButtonSize::Compact)
                        .child(
                            TextRole::Button
                                .apply(div())
                                .child("Compact")
                                .debug_selector(|| "compact-label".into()),
                        )
                        .debug_selector(|| "compact-button".into()),
                )
                .child(
                    icon_button(
                        "small-icon",
                        IconName::Close,
                        ButtonVariant::Tertiary,
                        ButtonSize::SmallIcon,
                    )
                    .debug_selector(|| "small-icon-button".into()),
                )
                .child(
                    icon_button(
                        "compact-icon",
                        IconName::Plus,
                        ButtonVariant::Tertiary,
                        ButtonSize::Icon,
                    )
                    .debug_selector(|| "compact-icon-button".into()),
                )
        }
    }

    #[gpui::test]
    async fn compact_buttons_have_dense_external_targets(cx: &mut gpui::TestAppContext) {
        cx.update(gpui_component::init);
        let (_view, cx) = cx.add_window_view(|_, _| ButtonTestView);
        cx.run_until_parked();

        let compact = cx.debug_bounds("compact-button").unwrap();
        let label = cx.debug_bounds("compact-label").unwrap();
        assert_eq!(compact.size.height, px(24.));
        // Six pixels of padding plus the existing one-pixel border on each side.
        assert_eq!(compact.size.width - label.size.width, px(14.));

        for selector in ["small-icon-button", "compact-icon-button"] {
            let icon = cx.debug_bounds(selector).unwrap();
            assert!(icon.size.width >= px(24.));
            assert!(icon.size.height >= px(24.));
            assert!(icon.contains(&icon.center()));
            assert!(!icon.contains(&point(icon.right() + px(1.), icon.center().y,)));
        }
    }

    struct ActivationTestView {
        clicks: usize,
        focus_handle: gpui::FocusHandle,
    }

    impl gpui::Focusable for ActivationTestView {
        fn focus_handle(&self, _: &App) -> gpui::FocusHandle {
            self.focus_handle.clone()
        }
    }

    impl Render for ActivationTestView {
        fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
            let action = |id: &'static str| {
                button(id, id, ButtonVariant::Primary).on_click(cx.listener(|this, _, _, cx| {
                    this.clicks += 1;
                    cx.notify();
                }))
            };
            div()
                .child(action("enabled").debug_selector(|| "enabled-button".into()))
                .child(
                    action("disabled")
                        .disabled(true)
                        .debug_selector(|| "disabled-button".into()),
                )
                .child(
                    action("loading")
                        .loading(true)
                        .debug_selector(|| "loading-button".into()),
                )
        }
    }

    #[gpui::test]
    async fn disabled_and_loading_buttons_never_activate(cx: &mut gpui::TestAppContext) {
        cx.update(gpui_component::init);
        let mut test_view = None;
        let (_root, cx) = cx.add_window_view(|window, cx| {
            let view = cx.new(|cx| ActivationTestView {
                clicks: 0,
                focus_handle: cx.focus_handle(),
            });
            test_view = Some(view.clone());
            Root::new(view, window, cx)
        });
        let view = test_view.unwrap();
        cx.run_until_parked();

        let enabled = cx.debug_bounds("enabled-button").unwrap();
        cx.simulate_click(enabled.center(), Modifiers::default());
        assert_eq!(cx.read(|app| view.read(app).clicks), 1);

        for selector in ["disabled-button", "loading-button"] {
            let bounds = cx.debug_bounds(selector).unwrap();
            cx.simulate_click(bounds.center(), Modifiers::default());
            assert_eq!(cx.read(|app| view.read(app).clicks), 1);
        }

        cx.update(|window, app| {
            let focus_handle = view.read(app).focus_handle.clone();
            focus_handle.focus(window);
            window.focus_next();
        });
        cx.simulate_keystrokes("enter");
        assert_eq!(cx.read(|app| view.read(app).clicks), 1);

        cx.update(|window, _| window.focus_next());
        cx.simulate_keystrokes("enter");
        assert_eq!(cx.read(|app| view.read(app).clicks), 1);

        cx.update(|window, _| window.focus_next());
        cx.simulate_keystrokes("enter");
        assert_eq!(cx.read(|app| view.read(app).clicks), 1);
    }
}
