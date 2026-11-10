//! Theme resolution, the OxideSSH widget palette, and the terminal palette.

use std::{
    fs,
    path::{Component, PathBuf},
};

use gpui::{App, Hsla, Window, hsla, px};
use gpui_component::{Theme, ThemeColor, ThemeMode as ComponentThemeMode};
use oxide_ssh_core::model::ThemeMode;
use oxide_ssh_terminal::{RgbColor, TerminalColors};
use serde::Deserialize;

pub const BUILT_IN_THEME: &str = "built-in";

#[derive(Clone, Debug, Default, PartialEq)]
pub struct ThemeOverrides {
    pub background: Option<Hsla>,
    pub raised: Option<Hsla>,
    pub hover: Option<Hsla>,
    pub border: Option<Hsla>,
    pub foreground: Option<Hsla>,
    pub muted_foreground: Option<Hsla>,
    pub primary: Option<Hsla>,
    pub primary_hover: Option<Hsla>,
    pub primary_active: Option<Hsla>,
    pub primary_foreground: Option<Hsla>,
    pub strip: Option<Hsla>,
    pub danger: Option<Hsla>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct RawThemeOverrides {
    background: Option<String>,
    raised: Option<String>,
    hover: Option<String>,
    border: Option<String>,
    foreground: Option<String>,
    muted_foreground: Option<String>,
    primary: Option<String>,
    primary_hover: Option<String>,
    primary_active: Option<String>,
    primary_foreground: Option<String>,
    strip: Option<String>,
    danger: Option<String>,
}

fn parse_color(value: Option<String>, field: &str) -> Result<Option<Hsla>, String> {
    let Some(value) = value else {
        return Ok(None);
    };
    let error = || format!("{field}: expected #RRGGBB or #RRGGBBAA");
    let digits = value.strip_prefix('#').ok_or_else(error)?;
    if !matches!(digits.len(), 6 | 8) || !digits.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        return Err(error());
    }
    let rgba = u32::from_str_radix(digits, 16).map_err(|_| error())?;
    let rgba = if digits.len() == 6 {
        (rgba << 8) | 0xff
    } else {
        rgba
    };
    Ok(Some(gpui::rgba(rgba).into()))
}

impl ThemeOverrides {
    pub fn parse(json: &str) -> Result<Self, String> {
        let raw: RawThemeOverrides =
            serde_json::from_str(json).map_err(|error| format!("invalid theme JSON: {error}"))?;
        Ok(Self {
            background: parse_color(raw.background, "background")?,
            raised: parse_color(raw.raised, "raised")?,
            hover: parse_color(raw.hover, "hover")?,
            border: parse_color(raw.border, "border")?,
            foreground: parse_color(raw.foreground, "foreground")?,
            muted_foreground: parse_color(raw.muted_foreground, "mutedForeground")?,
            primary: parse_color(raw.primary, "primary")?,
            primary_hover: parse_color(raw.primary_hover, "primaryHover")?,
            primary_active: parse_color(raw.primary_active, "primaryActive")?,
            primary_foreground: parse_color(raw.primary_foreground, "primaryForeground")?,
            strip: parse_color(raw.strip, "strip")?,
            danger: parse_color(raw.danger, "danger")?,
        })
    }
}

fn themes_directory() -> Result<PathBuf, String> {
    let executable = std::env::current_exe().map_err(|error| error.to_string())?;
    let parent = executable
        .parent()
        .ok_or_else(|| "executable has no parent directory".to_owned())?;
    Ok(parent.join("themes"))
}

pub fn load_theme(name: &str) -> Result<ThemeOverrides, String> {
    if name == BUILT_IN_THEME {
        return Ok(ThemeOverrides::default());
    }
    if name.is_empty()
        || PathBuf::from(name).components().count() != 1
        || !matches!(
            PathBuf::from(name).components().next(),
            Some(Component::Normal(_))
        )
    {
        return Err(format!("invalid theme name: {name}"));
    }
    let path = themes_directory()?.join(format!("{name}.json"));
    let json = fs::read_to_string(&path).map_err(|error| format!("{}: {error}", path.display()))?;
    ThemeOverrides::parse(&json).map_err(|error| format!("{}: {error}", path.display()))
}

pub fn available_themes() -> Vec<String> {
    let Ok(directory) = themes_directory() else {
        return Vec::new();
    };
    let Ok(entries) = fs::read_dir(directory) else {
        return Vec::new();
    };
    let mut themes: Vec<_> = entries
        .filter_map(Result::ok)
        .filter(|entry| entry.path().is_file())
        .filter(|entry| {
            entry
                .path()
                .extension()
                .is_some_and(|extension| extension == "json")
        })
        .filter_map(|entry| {
            let name = entry.path().file_stem()?.to_str()?.to_owned();
            if name == BUILT_IN_THEME {
                return None;
            }
            ThemeOverrides::parse(&fs::read_to_string(entry.path()).ok()?).ok()?;
            Some(name)
        })
        .collect();
    themes.sort_by_key(|name| name.to_lowercase());
    themes
}

/// Card layout scale: gap around floating cards, and inner padding of the
/// terminal card. CARD_PADDING >= radius_lg so glyphs never reach the
/// rounded corners.
pub const CARD_GAP: f32 = 8.;
pub const CARD_PADDING: f32 = 12.;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ResolvedTheme {
    Light,
    Dark,
}

impl ResolvedTheme {
    pub fn resolve(setting: ThemeMode, system_is_dark: bool) -> Self {
        match setting {
            ThemeMode::Light => Self::Light,
            ThemeMode::Dark => Self::Dark,
            ThemeMode::System if system_is_dark => Self::Dark,
            ThemeMode::System => Self::Light,
        }
    }

    pub fn terminal_colors(self, ansi_palette: Option<[u32; 16]>) -> TerminalColors {
        let (foreground, background, cursor, default_palette) = match self {
            Self::Dark => (
                0xe8e6dc,
                0x1f1e1d,
                0xd97757,
                [
                    0x1b1d23, 0xe06c75, 0x98c379, 0xe5c07b, 0x61afef, 0xc678dd, 0x56b6c2, 0xabb2bf,
                    0x5c6370, 0xff7b86, 0xb3e98c, 0xffd68a, 0x76c7ff, 0xd99bff, 0x70e1ed, 0xf5f7fa,
                ],
            ),
            Self::Light => (
                0x1f1e1d,
                0xfaf9f5,
                0xd97757,
                [
                    0x202124, 0xb4232c, 0x2f7d32, 0x8a5a00, 0x005fb8, 0x7a3e9d, 0x007a7a, 0xdadce0,
                    0x5f6368, 0xd93025, 0x188038, 0xb06000, 0x1967d2, 0x9334e6, 0x008b8b, 0xffffff,
                ],
            ),
        };
        let ansi = ansi_palette.unwrap_or(default_palette);
        TerminalColors {
            foreground: rgb_color(foreground),
            background: rgb_color(background),
            cursor: rgb_color(cursor),
            ansi: ansi.map(|color| rgb_color(color & 0x00ff_ffff)),
        }
    }
}

/// Switch gpui-component to the resolved mode, then lay the OxideSSH palette
/// over its default tokens. Warm Claude-style neutrals (cream light, umber
/// dark) with the Claude terracotta-orange accent #D97757.
pub fn apply_theme(
    theme: ResolvedTheme,
    name: &str,
    window: &mut Window,
    cx: &mut App,
) -> Result<(), String> {
    let overrides = load_theme(name);
    Theme::change(
        match theme {
            ResolvedTheme::Light => ComponentThemeMode::Light,
            ResolvedTheme::Dark => ComponentThemeMode::Dark,
        },
        Some(window),
        cx,
    );
    let global = Theme::global_mut(cx);
    // Radius scale: 8px for controls, 12px for cards and dialogs.
    global.radius = px(8.);
    global.radius_lg = px(12.);
    match theme {
        ResolvedTheme::Dark => apply_dark_palette(&mut global.colors),
        ResolvedTheme::Light => apply_light_palette(&mut global.colors),
    }
    match overrides {
        Ok(overrides) => {
            apply_overrides(&mut global.colors, theme, &overrides);
            Ok(())
        }
        Err(error) => Err(error),
    }
}

fn apply_overrides(colors: &mut ThemeColor, theme: ResolvedTheme, overrides: &ThemeOverrides) {
    if let Some(color) = overrides.background {
        colors.background = color;
        colors.accordion = color;
        colors.list = color;
    }
    if let Some(color) = overrides.raised {
        colors.secondary = color;
        colors.popover = color;
        colors.group_box = color;
        colors.tab_active = color;
        colors.list_even = color;
        colors.list_head = color;
    }
    if let Some(color) = overrides.hover {
        colors.secondary_hover = color;
        colors.secondary_active = color;
        colors.accent = color;
        colors.accordion_hover = color;
        colors.list_hover = color;
    }
    if let Some(color) = overrides.border {
        colors.border = color;
        colors.input = color;
        colors.window_border = color;
        colors.title_bar_border = color;
    }
    if let Some(color) = overrides.foreground {
        colors.foreground = color;
        colors.caret = color;
        colors.secondary_foreground = color;
        colors.accent_foreground = color;
        colors.popover_foreground = color;
        colors.group_box_foreground = color;
        colors.description_list_label_foreground = color;
        colors.sidebar_accent_foreground = color;
        colors.tab_active_foreground = color;
    }
    if let Some(color) = overrides.muted_foreground {
        colors.muted_foreground = color;
        colors.tab_foreground = color;
    }
    if let Some(color) = overrides.primary {
        colors.primary = color;
        colors.link = color;
        colors.progress_bar = color;
        colors.drag_border = color;
        colors.list_active_border = color;
        colors.slider_thumb = color;
        colors.sidebar_primary = color;
        colors.ring = Hsla {
            a: match theme {
                ResolvedTheme::Light => 0.45,
                ResolvedTheme::Dark => 0.55,
            },
            ..color
        };
        colors.selection = Hsla {
            a: match theme {
                ResolvedTheme::Light => 0.28,
                ResolvedTheme::Dark => 0.35,
            },
            ..color
        };
        colors.list_active = Hsla { a: 0.16, ..color };
        colors.drop_target = Hsla {
            a: match theme {
                ResolvedTheme::Light => 0.12,
                ResolvedTheme::Dark => 0.15,
            },
            ..color
        };
    }
    if let Some(color) = overrides.primary_hover {
        colors.primary_hover = color;
        colors.link_hover = color;
    }
    if let Some(color) = overrides.primary_active {
        colors.primary_active = color;
        colors.link_active = color;
    }
    if let Some(color) = overrides.primary_foreground {
        colors.primary_foreground = color;
        colors.sidebar_primary_foreground = color;
    }
    if let Some(color) = overrides.strip {
        colors.sidebar = color;
        colors.tab_bar = color;
        colors.tab_bar_segmented = color;
        colors.title_bar = color;
    }
    if let Some(color) = overrides.danger {
        colors.danger = color;
    }
}

fn hsl(h: f32, s: f32, l: f32) -> Hsla {
    hsla(h / 360., s / 100., l / 100., 1.)
}

fn apply_dark_palette(colors: &mut ThemeColor) {
    let background = hsl(60., 2.7, 14.5);
    let raised = hsl(60., 2.1, 18.4);
    let hover = hsl(40., 2.7, 22.2);
    let border = hsl(45., 3., 25.9);
    let foreground = hsl(50., 20.7, 88.6);
    let muted_foreground = hsl(51.4, 3., 54.7);
    let primary = hsl(14.8, 63.1, 59.6);
    let primary_hover = hsl(15., 66., 65.);
    let primary_active = hsl(15.1, 55.6, 52.4);
    let primary_foreground = hsl(30., 3.3, 11.8);
    let strip = hsl(60., 2.7, 12.);

    colors.background = background;
    colors.foreground = foreground;
    colors.muted = hsl(60., 2.1, 17.);
    colors.muted_foreground = muted_foreground;
    colors.border = border;
    colors.input = border;
    colors.caret = foreground;
    colors.window_border = border;

    colors.primary = primary;
    colors.primary_hover = primary_hover;
    colors.primary_active = primary_active;
    colors.primary_foreground = primary_foreground;
    colors.ring = hsla(15. / 360., 0.63, 0.60, 0.55);
    colors.selection = hsla(15. / 360., 0.55, 0.60, 0.35);
    colors.link = primary;
    colors.link_hover = primary_hover;
    colors.link_active = primary_active;
    colors.progress_bar = primary;
    colors.drag_border = primary;
    colors.drop_target = hsla(15. / 360., 0.63, 0.60, 0.15);

    colors.secondary = raised;
    colors.secondary_hover = hover;
    colors.secondary_active = hover;
    colors.secondary_foreground = foreground;
    colors.accent = hover;
    colors.accent_foreground = foreground;

    colors.popover = hsl(60., 2.1, 17.5);
    colors.popover_foreground = foreground;
    colors.group_box = raised;
    colors.group_box_foreground = foreground;
    colors.accordion = background;
    colors.accordion_hover = hover;
    colors.description_list_label = raised;
    colors.description_list_label_foreground = foreground;
    colors.skeleton = hsl(60., 2.1, 17.);

    colors.sidebar = strip;
    colors.sidebar_foreground = hsl(50., 9., 73.7);
    colors.sidebar_border = hsl(45., 3., 21.);
    colors.sidebar_accent = hsl(40., 2.7, 20.5);
    colors.sidebar_accent_foreground = foreground;
    colors.sidebar_primary = primary;
    colors.sidebar_primary_foreground = primary_foreground;

    colors.tab_bar = strip;
    colors.tab_bar_segmented = strip;
    colors.tab = Hsla::transparent_black();
    colors.tab_active = raised;
    colors.tab_active_foreground = foreground;
    colors.tab_foreground = muted_foreground;
    colors.title_bar = strip;
    colors.title_bar_border = border;

    colors.list = background;
    colors.list_hover = hover;
    colors.list_active = hsla(15. / 360., 0.63, 0.60, 0.16);
    colors.list_active_border = primary;
    colors.list_even = raised;
    colors.list_head = raised;

    colors.scrollbar = Hsla::transparent_black();
    colors.scrollbar_thumb = hsl(45., 2.5, 34.);
    colors.scrollbar_thumb_hover = hsl(45., 2.5, 40.);
    colors.slider_bar = hsl(60., 2.1, 17.);
    colors.slider_thumb = primary;
    colors.switch = hsl(45., 3., 28.);
    colors.switch_thumb = hsl(50., 20.7, 88.6);

    colors.danger = hsl(3., 48., 50.);
    colors.danger_hover = hsl(3., 52., 56.);
    colors.danger_active = hsl(3., 45., 44.);
    colors.danger_foreground = hsl(0., 30., 96.);
}

fn apply_light_palette(colors: &mut ThemeColor) {
    let background = hsl(48., 25., 92.2);
    let raised = hsl(48., 33.3, 97.1);
    let hover = hsl(51.4, 14.3, 90.4);
    let border = hsl(52.5, 9.8, 83.9);
    let foreground = hsl(30., 3.3, 11.8);
    let muted_foreground = hsl(50., 2.9, 40.8);
    let primary = hsl(14.8, 63.1, 59.6);
    let primary_hover = hsl(15.1, 55.6, 52.4);
    let primary_active = hsl(13.8, 49.8, 46.1);
    let primary_foreground = hsl(48., 33.3, 97.1);
    let strip = hsl(48., 18., 89.);

    colors.background = background;
    colors.foreground = foreground;
    colors.muted = hsl(51.4, 14.3, 90.4);
    colors.muted_foreground = muted_foreground;
    colors.border = border;
    colors.input = border;
    colors.caret = foreground;
    colors.window_border = border;

    colors.primary = primary;
    colors.primary_hover = primary_hover;
    colors.primary_active = primary_active;
    colors.primary_foreground = primary_foreground;
    colors.ring = hsla(15. / 360., 0.63, 0.60, 0.45);
    colors.selection = hsla(15. / 360., 0.63, 0.60, 0.28);
    colors.link = primary;
    colors.link_hover = primary_hover;
    colors.link_active = primary_active;
    colors.progress_bar = primary;
    colors.drag_border = primary;
    colors.drop_target = hsla(15. / 360., 0.63, 0.60, 0.12);

    colors.secondary = raised;
    colors.secondary_hover = hover;
    colors.secondary_active = hover;
    colors.secondary_foreground = foreground;
    colors.accent = hover;
    colors.accent_foreground = foreground;

    colors.popover = hsl(48., 33.3, 97.1);
    colors.popover_foreground = foreground;
    colors.group_box = raised;
    colors.group_box_foreground = foreground;
    colors.accordion = background;
    colors.accordion_hover = hover;
    colors.description_list_label = raised;
    colors.description_list_label_foreground = foreground;
    colors.skeleton = hsl(51.4, 14.3, 90.4);

    colors.sidebar = strip;
    colors.sidebar_foreground = hsl(60., 2.5, 23.3);
    colors.sidebar_border = hsl(52.5, 9.8, 81.);
    colors.sidebar_accent = hsl(51.4, 14.3, 87.);
    colors.sidebar_accent_foreground = foreground;
    colors.sidebar_primary = primary;
    colors.sidebar_primary_foreground = primary_foreground;

    colors.tab_bar = strip;
    colors.tab_bar_segmented = strip;
    colors.tab = Hsla::transparent_black();
    colors.tab_active = raised;
    colors.tab_active_foreground = foreground;
    colors.tab_foreground = muted_foreground;
    colors.title_bar = strip;
    colors.title_bar_border = border;

    colors.list = background;
    colors.list_hover = hover;
    colors.list_active = hsla(15. / 360., 0.63, 0.60, 0.12);
    colors.list_active_border = primary;
    colors.list_even = raised;
    colors.list_head = raised;

    colors.scrollbar = Hsla::transparent_black();
    colors.scrollbar_thumb = hsl(50., 5., 70.);
    colors.scrollbar_thumb_hover = hsl(50., 5., 62.);
    colors.slider_bar = hsl(51.4, 14.3, 88.);
    colors.slider_thumb = primary;
    colors.switch = hsl(52.5, 9.8, 80.);
    colors.switch_thumb = hsl(48., 33.3, 97.1);

    colors.danger = hsl(3., 52., 52.);
    colors.danger_hover = hsl(3., 56., 57.);
    colors.danger_active = hsl(3., 48., 46.);
    colors.danger_foreground = hsl(0., 40., 97.);
}

const fn rgb_color(hex: u32) -> RgbColor {
    RgbColor::new(
        ((hex >> 16) & 0xff) as u8,
        ((hex >> 8) & 0xff) as u8,
        (hex & 0xff) as u8,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bundled_codex_themes_are_valid() {
        for json in [
            include_str!("../../../../themes/codex-light.json"),
            include_str!("../../../../themes/codex-dark.json"),
        ] {
            let theme = ThemeOverrides::parse(json).unwrap();
            assert!(theme.background.is_some());
            assert!(theme.foreground.is_some());
            assert!(theme.primary.is_some());
        }
    }

    #[test]
    fn theme_file_supports_partial_colors_and_rejects_invalid_values() {
        let theme = ThemeOverrides::parse(
            r##"{"background":"#112233","primary":"#44556677","mutedForeground":"#AABBCC"}"##,
        )
        .unwrap();
        assert_eq!(theme.background, Some(gpui::rgb(0x112233).into()));
        assert_eq!(theme.primary, Some(gpui::rgba(0x44556677).into()));
        assert_eq!(theme.muted_foreground, Some(gpui::rgb(0xAABBCC).into()));
        assert_eq!(theme.raised, None);
        assert!(ThemeOverrides::parse(r##"{"background":"red"}"##).is_err());
    }

    #[test]
    fn explicit_ansi_palette_overrides_each_theme_and_default_remains_theme_specific() {
        let custom = [0x123456; 16];
        assert_eq!(
            ResolvedTheme::Dark.terminal_colors(Some(custom)).ansi[0],
            RgbColor::new(0x12, 0x34, 0x56)
        );
        assert_eq!(
            ResolvedTheme::Light.terminal_colors(Some(custom)).ansi[0],
            RgbColor::new(0x12, 0x34, 0x56)
        );
        assert_eq!(
            ResolvedTheme::Light.terminal_colors(None).ansi[0],
            RgbColor::new(0x20, 0x21, 0x24)
        );
        assert_eq!(
            ResolvedTheme::Dark.terminal_colors(None).ansi[0],
            RgbColor::new(0x1b, 0x1d, 0x23)
        );
    }
}
