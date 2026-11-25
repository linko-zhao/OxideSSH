use gpui::{
    App, Div, ElementId, FontWeight, Hsla, ParentElement as _, SharedString, Styled, div, relative,
    rems, rgba,
};
use gpui_component::{
    ActiveTheme as _, Icon, Sizable as _,
    button::{Button, ButtonVariants as _},
    theme::Theme,
};

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
    pub(super) fn resolve(self, theme: &Theme) -> Hsla {
        match self {
            Self::Primary => theme.foreground,
            Self::Secondary | Self::Weak | Self::Placeholder => theme.muted_foreground,
            Self::Danger => theme.danger,
            Self::Warning => rgba(if theme.is_dark() {
                0xfacc15ff
            } else {
                0x854d0eff
            })
            .into(),
            Self::Accent => theme.primary,
            Self::OnAccent => theme.primary_foreground,
            Self::OnSelection => theme.accent_foreground,
        }
    }

    pub(super) fn apply<T: Styled>(self, element: T, cx: &App) -> T {
        element.text_color(self.resolve(cx.theme()))
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
            ButtonSize::Compact | ButtonSize::Icon => self.compact(),
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
