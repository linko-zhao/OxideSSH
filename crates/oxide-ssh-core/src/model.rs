use std::path::PathBuf;

use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Clone, Copy, Debug, Deserialize, Eq, Hash, PartialEq, Serialize)]
#[serde(transparent)]
pub struct ProfileId(pub Uuid);

impl ProfileId {
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }
}

impl Default for ProfileId {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, Hash, PartialEq, Serialize)]
#[serde(transparent)]
pub struct SessionId(pub Uuid);

impl SessionId {
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }
}

impl Default for SessionId {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, Hash, PartialEq, Serialize)]
#[serde(transparent)]
pub struct TagId(pub Uuid);

impl TagId {
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }
}

impl Default for TagId {
    fn default() -> Self {
        Self::new()
    }
}

/// Fixed palette of tag colors; rendered via theme variables in the UI.
#[derive(Clone, Copy, Debug, Deserialize, Eq, Hash, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum TagColor {
    Red,
    Orange,
    Yellow,
    Green,
    Blue,
    Purple,
    Pink,
    Gray,
}

impl TagColor {
    pub const ALL: [TagColor; 8] = [
        TagColor::Red,
        TagColor::Orange,
        TagColor::Yellow,
        TagColor::Green,
        TagColor::Blue,
        TagColor::Purple,
        TagColor::Pink,
        TagColor::Gray,
    ];
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Tag {
    pub id: TagId,
    pub name: String,
    pub color: Option<TagColor>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, thiserror::Error)]
pub enum TagValidationError {
    #[error("tag name is invalid")]
    Name,
}

impl Tag {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            id: TagId::new(),
            name: name.into(),
            color: None,
        }
    }

    pub fn validate(&self) -> Result<(), TagValidationError> {
        let name_len = self.name.trim().chars().count();
        if !(1..=32).contains(&name_len) {
            return Err(TagValidationError::Name);
        }
        Ok(())
    }
}

#[derive(Clone, Debug, Deserialize, Eq, Hash, PartialEq, Serialize)]
#[serde(transparent)]
pub struct CredentialRef(pub String);

impl CredentialRef {
    pub fn as_str(&self) -> &str {
        &self.0
    }

    pub fn password(profile_id: ProfileId) -> Self {
        Self(format!("profile/{}/password", profile_id.0))
    }

    pub fn private_key_passphrase(profile_id: ProfileId) -> Self {
        Self(format!("profile/{}/private-key-passphrase", profile_id.0))
    }
}

#[derive(Clone, Debug, Deserialize, Eq, Hash, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Endpoint {
    pub host: String,
    pub port: u16,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct EnvironmentVariable {
    pub name: String,
    pub value: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ConnectionProfile {
    pub id: ProfileId,
    pub name: String,
    pub endpoint: Endpoint,
    pub username: String,
    pub auth: AuthConfig,
    #[serde(default)]
    pub tag_ids: Vec<TagId>,
    #[serde(default)]
    pub startup_command: Option<String>,
    #[serde(default)]
    pub ssh_keepalive_interval_seconds: Option<u64>,
    #[serde(default)]
    pub ssh_keepalive_max_count: Option<usize>,
    #[serde(default)]
    pub environment_variables: Vec<EnvironmentVariable>,
}

#[cfg(test)]
mod startup_command_tests {
    use super::*;

    #[test]
    fn old_profiles_load_without_a_startup_command() {
        let profile = ConnectionProfile {
            id: ProfileId::new(),
            name: "old".into(),
            endpoint: Endpoint {
                host: "example.com".into(),
                port: 22,
            },
            username: "user".into(),
            auth: AuthConfig::Agent,
            tag_ids: Vec::new(),
            startup_command: None,
            ssh_keepalive_interval_seconds: None,
            ssh_keepalive_max_count: None,
            environment_variables: Vec::new(),
        };
        let mut value = serde_json::to_value(profile).unwrap();
        value.as_object_mut().unwrap().remove("startupCommand");
        let profile: ConnectionProfile = serde_json::from_value(value).unwrap();
        assert_eq!(profile.startup_command, None);
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, thiserror::Error)]
pub enum ProfileValidationError {
    #[error("profile name is invalid")]
    Name,
    #[error("host is invalid")]
    Host,
    #[error("username is invalid")]
    Username,
    #[error("port is invalid")]
    Port,
    #[error("private key path is empty")]
    PrivateKeyPath,
    #[error("SSH keepalive values must be positive")]
    Keepalive,
    #[error("environment variable name is invalid")]
    EnvironmentVariable,
}

impl ConnectionProfile {
    pub fn validate(&self) -> Result<(), ProfileValidationError> {
        let name_len = self.name.trim().chars().count();
        if !(1..=64).contains(&name_len) {
            return Err(ProfileValidationError::Name);
        }

        let host = self.endpoint.host.as_bytes();
        if host.is_empty()
            || host.len() > 253
            || !host.is_ascii()
            || host
                .iter()
                .any(|byte| byte.is_ascii_whitespace() || matches!(byte, 0 | b'/' | b'\\'))
        {
            return Err(ProfileValidationError::Host);
        }

        let username = self.username.as_bytes();
        if username.is_empty()
            || username.len() > 255
            || username
                .iter()
                .any(|byte| matches!(byte, 0 | b'\r' | b'\n'))
        {
            return Err(ProfileValidationError::Username);
        }

        if self.endpoint.port == 0 {
            return Err(ProfileValidationError::Port);
        }

        if let AuthConfig::PrivateKey { path, .. } = &self.auth
            && path.as_os_str().is_empty()
        {
            return Err(ProfileValidationError::PrivateKeyPath);
        }
        if self.ssh_keepalive_interval_seconds == Some(0) || self.ssh_keepalive_max_count == Some(0)
        {
            return Err(ProfileValidationError::Keepalive);
        }

        if self.environment_variables.iter().any(|variable| {
            let mut bytes = variable.name.bytes();
            !matches!(bytes.next(), Some(b'A'..=b'Z' | b'a'..=b'z' | b'_'))
                || !bytes.all(|byte| byte.is_ascii_alphanumeric() || byte == b'_')
        }) {
            return Err(ProfileValidationError::EnvironmentVariable);
        }

        Ok(())
    }

    pub fn inherit_keepalive_defaults(&mut self, settings: &AppSettings) {
        self.ssh_keepalive_interval_seconds
            .get_or_insert(settings.ssh_keepalive_interval_seconds);
        self.ssh_keepalive_max_count
            .get_or_insert(settings.ssh_keepalive_max_count);
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(
    tag = "method",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
pub enum AuthConfig {
    Password {
        credential_ref: Option<CredentialRef>,
    },
    PrivateKey {
        path: PathBuf,
        passphrase_ref: Option<CredentialRef>,
    },
    Agent,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TerminalSize {
    pub columns: u32,
    pub rows: u32,
    pub pixel_width: u32,
    pub pixel_height: u32,
}

impl TerminalSize {
    pub fn is_valid(self) -> bool {
        self.columns > 0 && self.rows > 0
    }
}

#[derive(Clone, Copy, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
pub enum LocaleSetting {
    #[default]
    #[serde(rename = "system")]
    System,
    #[serde(rename = "en-US")]
    EnUs,
    #[serde(rename = "zh-CN")]
    ZhCn,
}

#[derive(Clone, Copy, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum ThemeSetting {
    #[default]
    System,
    Light,
    Dark,
}

#[derive(Clone, Copy, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum Osc52Clipboard {
    #[default]
    Off,
    Ask,
    On,
}

#[derive(Clone, Copy, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum NewTabAction {
    #[default]
    AddConnection,
    LocalTerminal,
}

fn default_ssh_connect_timeout_seconds() -> u64 {
    15
}
#[derive(Clone, Copy, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WindowSize {
    pub width: u32,
    pub height: u32,
}

#[derive(Clone, Copy, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum TerminalBell {
    #[default]
    Visual,
    Sound,
    Off,
}
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AppSettings {
    #[serde(default)]
    pub locale: LocaleSetting,
    #[serde(default)]
    pub theme: ThemeSetting,
    #[serde(default = "default_ssh_connect_timeout_seconds")]
    pub ssh_connect_timeout_seconds: u64,
    #[serde(default)]
    pub local_working_directory: String,
    #[serde(default)]
    pub local_shell_path: String,
    #[serde(default)]
    pub local_shell_args: Vec<String>,
    #[serde(default)]
    pub new_tab_action: NewTabAction,
    #[serde(default = "default_terminal_font_family")]
    pub terminal_font_family: String,
    #[serde(default = "default_terminal_font_size")]
    pub terminal_font_size: u16,
    #[serde(default = "default_terminal_line_height")]
    pub terminal_line_height: u16,
    #[serde(default)]
    pub remember_window_size: bool,
    #[serde(default)]
    pub window_size: Option<WindowSize>,
    #[serde(default)]
    pub ssh_compression: bool,
    #[serde(default = "default_confirm_before_close_with_sessions")]
    pub confirm_before_close_with_sessions: bool,
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "deserialize_ansi_palette"
    )]
    pub ansi_palette: Option<[u32; 16]>,
    #[serde(default = "default_keepalive_interval")]
    pub ssh_keepalive_interval_seconds: u64,
    #[serde(default = "default_keepalive_max")]
    pub ssh_keepalive_max_count: usize,
    #[serde(default)]
    pub terminal_bell: TerminalBell,
    #[serde(default = "default_paste_confirm_threshold")]
    pub paste_confirm_threshold: usize,
    #[serde(default)]
    pub osc52_clipboard: Osc52Clipboard,
    #[serde(default)]
    pub copy_on_select: bool,
    #[serde(default = "default_ssh_term")]
    pub ssh_term: String,
    #[serde(default = "default_scrollback_lines")]
    pub terminal_scrollback_lines: usize,
}

pub fn default_ssh_term() -> String {
    "xterm-256color".into()
}

pub fn valid_ssh_term(term: &str) -> bool {
    !term.is_empty()
        && term.len() <= 64
        && term
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || b"._+-".contains(&byte))
}

pub fn effective_ssh_term(term: &str) -> &str {
    if valid_ssh_term(term) {
        term
    } else {
        "xterm-256color"
    }
}
#[cfg(test)]
mod term_tests {
    use super::*;

    #[test]
    fn missing_term_defaults_and_invalid_values_are_rejected() {
        let settings: AppSettings =
            serde_json::from_str(r#"{"locale":"zh-CN","theme":"dark"}"#).unwrap();
        assert_eq!(settings.ssh_term, "xterm-256color");
        assert_eq!(AppSettings::default().ssh_term, "xterm-256color");
        assert!(valid_ssh_term("tmux-256color"));
        assert!(!valid_ssh_term(""));
        assert!(!valid_ssh_term("bad term"));
        assert_eq!(effective_ssh_term(""), "xterm-256color");
        assert_eq!(effective_ssh_term("bad term"), "xterm-256color");
    }
}

pub const fn default_paste_confirm_threshold() -> usize {
    4 * 1024
}

fn default_keepalive_interval() -> u64 {
    30
}

fn default_keepalive_max() -> usize {
    3
}

fn deserialize_ansi_palette<'de, D>(deserializer: D) -> Result<Option<[u32; 16]>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let values = serde_json::Value::deserialize(deserializer)?;
    let Some(values) = values.as_array() else {
        return Ok(None);
    };
    if values.len() != 16 {
        return Ok(None);
    }
    let mut palette = [0; 16];
    for (index, value) in values.iter().enumerate() {
        let Some(color) = value.as_u64().filter(|color| *color <= 0x00ff_ffff) else {
            return Ok(None);
        };
        palette[index] = color as u32;
    }
    Ok(Some(palette))
}
impl Default for AppSettings {
    fn default() -> Self {
        Self {
            locale: LocaleSetting::default(),
            theme: ThemeSetting::default(),
            ssh_connect_timeout_seconds: default_ssh_connect_timeout_seconds(),
            local_shell_path: String::new(),
            local_shell_args: Vec::new(),
            new_tab_action: NewTabAction::default(),
            ssh_compression: false,
            ssh_keepalive_interval_seconds: default_keepalive_interval(),
            ssh_keepalive_max_count: default_keepalive_max(),
            terminal_font_family: default_terminal_font_family(),
            terminal_font_size: default_terminal_font_size(),
            terminal_line_height: default_terminal_line_height(),
            remember_window_size: false,
            window_size: None,
            confirm_before_close_with_sessions: default_confirm_before_close_with_sessions(),
            ansi_palette: None,
            terminal_bell: TerminalBell::default(),
            paste_confirm_threshold: default_paste_confirm_threshold(),
            osc52_clipboard: Osc52Clipboard::default(),
            local_working_directory: String::new(),
            copy_on_select: false,
            ssh_term: default_ssh_term(),
            terminal_scrollback_lines: DEFAULT_SCROLLBACK_LINES,
        }
    }
}

fn default_terminal_font_family() -> String {
    #[cfg(target_os = "macos")]
    {
        "Menlo".into()
    }
    #[cfg(target_os = "windows")]
    {
        "Cascadia Mono".into()
    }
    #[cfg(all(not(target_os = "macos"), not(target_os = "windows")))]
    {
        "monospace".into()
    }
}

fn default_terminal_font_size() -> u16 {
    13
}
fn default_terminal_line_height() -> u16 {
    17
}

fn default_confirm_before_close_with_sessions() -> bool {
    true
}

pub const DEFAULT_SCROLLBACK_LINES: usize = 10_000;

fn default_scrollback_lines() -> usize {
    DEFAULT_SCROLLBACK_LINES
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn app_settings_default_close_confirmation_for_legacy_config() {
        let settings: AppSettings = serde_json::from_str(
            r#"{"locale":"system","theme":"system","newTabAction":"addConnection"}"#,
        )
        .unwrap();
        assert!(settings.confirm_before_close_with_sessions);
        assert!(AppSettings::default().confirm_before_close_with_sessions);
    }

    fn valid_profile() -> ConnectionProfile {
        ConnectionProfile {
            id: ProfileId::new(),
            name: "Local fixture".into(),
            endpoint: Endpoint {
                host: "example.com".into(),
                port: 22,
            },
            username: "oxide".into(),
            auth: AuthConfig::Agent,
            tag_ids: Vec::new(),
            startup_command: None,
            ssh_keepalive_interval_seconds: None,
            ssh_keepalive_max_count: None,
            environment_variables: Vec::new(),
        }
    }

    #[test]
    fn terminal_settings_default_and_deserialize_from_legacy_settings() {
        let defaults = AppSettings::default();
        assert_eq!(defaults.terminal_font_size, 13);
        assert_eq!(defaults.terminal_line_height, 17);
        assert!(!defaults.terminal_font_family.is_empty());
        let settings: AppSettings = serde_json::from_str("{}").unwrap();
        assert_eq!(settings.terminal_font_size, 13);
        assert_eq!(settings.terminal_line_height, 17);
        assert!(!settings.terminal_font_family.is_empty());
        let settings: AppSettings =
            serde_json::from_str(r#"{"locale":"system","theme":"system"}"#).unwrap();
        assert_eq!(settings.terminal_font_size, 13);
        assert_eq!(settings.terminal_line_height, 17);
    }

    #[test]
    fn profiles_without_environment_variables_load_and_invalid_names_are_rejected() {
        let profile: ConnectionProfile = serde_json::from_str(
            r#"{"id":"00000000-0000-0000-0000-000000000000","name":"host","endpoint":{"host":"example.com","port":22},"username":"oxide","auth":{"method":"agent"}}"#,
        )
        .unwrap();
        assert!(profile.environment_variables.is_empty());

        let mut profile = valid_profile();
        profile.environment_variables.push(EnvironmentVariable {
            name: "BUILD-NAME".into(),
            value: "release".into(),
        });
        assert_eq!(
            profile.validate(),
            Err(ProfileValidationError::EnvironmentVariable)
        );
    }

    #[test]
    fn valid_environment_variable_names_accept_underscores_and_values_verbatim() {
        let mut profile = valid_profile();
        profile.environment_variables.push(EnvironmentVariable {
            name: "_BUILD_2".into(),
            value: "  value=with spaces\nand lines  ".into(),
        });
        assert_eq!(profile.validate(), Ok(()));
    }

    #[test]
    fn valid_profile_accepts_dns_ipv4_and_unbracketed_ipv6() {
        for host in ["example.com", "127.0.0.1", "2001:db8::1"] {
            let mut profile = valid_profile();
            profile.endpoint.host = host.into();
            assert_eq!(profile.validate(), Ok(()));
        }
    }

    #[test]
    fn name_uses_trimmed_unicode_scalar_length() {
        let mut profile = valid_profile();
        profile.name = format!("  {}  ", "界".repeat(64));
        assert_eq!(profile.validate(), Ok(()));

        profile.name = "界".repeat(65);
        assert_eq!(profile.validate(), Err(ProfileValidationError::Name));
        profile.name = " \t ".into();
        assert_eq!(profile.validate(), Err(ProfileValidationError::Name));
    }

    #[test]
    fn host_rejects_non_ascii_length_and_forbidden_bytes() {
        for host in [
            "",
            "host name",
            "host/name",
            "host\\name",
            "host\0name",
            "münchen.example",
        ] {
            let mut profile = valid_profile();
            profile.endpoint.host = host.into();
            assert_eq!(profile.validate(), Err(ProfileValidationError::Host));
        }

        let mut profile = valid_profile();
        profile.endpoint.host = "a".repeat(254);
        assert_eq!(profile.validate(), Err(ProfileValidationError::Host));
    }

    #[test]
    fn scrollback_setting_defaults_for_old_settings() {
        let settings: AppSettings = serde_json::from_str(
            r#"{"locale":"zh-CN","theme":"dark","newTabAction":"localTerminal"}"#,
        )
        .unwrap();
        assert_eq!(settings.terminal_scrollback_lines, DEFAULT_SCROLLBACK_LINES);
    }

    #[test]
    fn scrollback_lines_round_trip() {
        let settings = AppSettings {
            terminal_scrollback_lines: 250,
            ..AppSettings::default()
        };
        let encoded = serde_json::to_string(&settings).unwrap();
        let decoded: AppSettings = serde_json::from_str(&encoded).unwrap();
        assert_eq!(decoded.terminal_scrollback_lines, 250);
    }

    #[test]
    fn settings_without_new_fields_keep_legacy_defaults() {
        // config.json written before newTabAction existed must still load,
        // otherwise the app falls into the read-only Recovery UI.
        let settings: AppSettings =
            serde_json::from_str(r#"{"locale":"zh-CN","theme":"dark"}"#).unwrap();
        assert_eq!(settings.new_tab_action, NewTabAction::AddConnection);
        assert!(!settings.ssh_compression);
        assert_eq!(settings.ssh_connect_timeout_seconds, 15);
        assert!(!settings.copy_on_select);
        assert_eq!(settings.theme, ThemeSetting::Dark);
        assert!(settings.local_shell_path.is_empty());
        assert!(settings.local_shell_args.is_empty());
    }

    #[test]
    fn malformed_ansi_palette_falls_back_to_theme_defaults() {
        let settings: AppSettings =
            serde_json::from_str(r#"{"ansiPalette":[4294967295]}"#).unwrap();
        assert_eq!(settings.ansi_palette, None);
    }

    #[test]
    fn missing_ansi_palette_preserves_legacy_settings() {
        let settings: AppSettings = serde_json::from_str(
            r#"{"locale":"en-US","theme":"dark","newTabAction":"addConnection"}"#,
        )
        .unwrap();
        assert_eq!(settings.ansi_palette, None);
    }

    #[test]
    fn settings_without_window_size_preferences_keep_defaults() {
        let settings: AppSettings =
            serde_json::from_str(r#"{"locale":"zh-CN","theme":"dark"}"#).unwrap();
        assert!(!settings.remember_window_size);
        assert_eq!(settings.window_size, None);
    }

    #[test]
    fn window_size_settings_round_trip() {
        let settings = AppSettings {
            remember_window_size: true,
            window_size: Some(WindowSize {
                width: 1280,
                height: 800,
            }),
            ..AppSettings::default()
        };
        let encoded = serde_json::to_string(&settings).unwrap();
        assert_eq!(
            serde_json::from_str::<AppSettings>(&encoded).unwrap(),
            settings
        );
    }

    #[test]
    fn keepalive_defaults_preserve_existing_behavior_and_profile_overrides_are_optional() {
        let settings: AppSettings =
            serde_json::from_str(r#"{"locale":"zh-CN","theme":"dark"}"#).unwrap();
        assert_eq!(settings.ssh_keepalive_interval_seconds, 30);
        assert_eq!(settings.ssh_keepalive_max_count, 3);

        let mut profile = valid_profile();
        profile.ssh_keepalive_interval_seconds = Some(12);
        profile.ssh_keepalive_max_count = Some(5);
        let mut old_profile = serde_json::to_value(valid_profile()).unwrap();
        old_profile
            .as_object_mut()
            .unwrap()
            .remove("sshKeepaliveIntervalSeconds");
        old_profile
            .as_object_mut()
            .unwrap()
            .remove("sshKeepaliveMaxCount");
        let old_profile: ConnectionProfile = serde_json::from_value(old_profile).unwrap();
        assert_eq!(old_profile.ssh_keepalive_interval_seconds, None);
        assert_eq!(old_profile.ssh_keepalive_max_count, None);
        let serialized = serde_json::to_value(profile).unwrap();
        let round_trip: ConnectionProfile = serde_json::from_value(serialized).unwrap();

        let defaults = AppSettings {
            ssh_keepalive_interval_seconds: 45,
            ssh_keepalive_max_count: 4,
            ..AppSettings::default()
        };
        let mut profile = valid_profile();
        profile.inherit_keepalive_defaults(&defaults);
        assert_eq!(profile.ssh_keepalive_interval_seconds, Some(45));
        assert_eq!(profile.ssh_keepalive_max_count, Some(4));

        profile.ssh_keepalive_interval_seconds = Some(12);
        profile.ssh_keepalive_max_count = Some(5);
        profile.inherit_keepalive_defaults(&defaults);
        assert_eq!(profile.ssh_keepalive_interval_seconds, Some(12));
        assert_eq!(profile.ssh_keepalive_max_count, Some(5));
        assert_eq!(round_trip.ssh_keepalive_interval_seconds, Some(12));
        assert_eq!(round_trip.ssh_keepalive_max_count, Some(5));
    }

    #[test]
    fn terminal_bell_defaults_to_visual_and_round_trips_all_modes() {
        let old: AppSettings =
            serde_json::from_str(r#"{"locale":"zh-CN","theme":"dark"}"#).unwrap();
        assert_eq!(old.terminal_bell, TerminalBell::Visual);
        for (json, mode) in [
            (r#""visual""#, TerminalBell::Visual),
            (r#""sound""#, TerminalBell::Sound),
            (r#""off""#, TerminalBell::Off),
        ] {
            assert_eq!(serde_json::from_str::<TerminalBell>(json).unwrap(), mode);
            assert_eq!(serde_json::to_string(&mode).unwrap(), json);
        }
    }

    #[test]
    fn old_settings_default_paste_confirmation_threshold() {
        let settings: AppSettings =
            serde_json::from_str(r#"{"locale":"zh-CN","theme":"dark"}"#).unwrap();
        assert_eq!(settings.paste_confirm_threshold, 4 * 1024);
    }

    #[test]
    fn osc52_clipboard_defaults_off_for_legacy_settings() {
        let settings: AppSettings =
            serde_json::from_str(r#"{"locale":"zh-CN","theme":"dark"}"#).unwrap();
        assert_eq!(settings.osc52_clipboard, Osc52Clipboard::Off);
    }

    #[test]
    fn copy_on_select_round_trips_in_settings() {
        let settings = AppSettings {
            copy_on_select: true,
            ..AppSettings::default()
        };
        let encoded = serde_json::to_string(&settings).unwrap();
        let decoded: AppSettings = serde_json::from_str(&encoded).unwrap();
        assert!(decoded.copy_on_select);
    }

    #[test]
    fn username_port_and_private_key_path_are_validated() {
        let mut profile = valid_profile();
        profile.username = "a".repeat(255);
        assert_eq!(profile.validate(), Ok(()));

        profile.username = "a".repeat(256);
        assert_eq!(profile.validate(), Err(ProfileValidationError::Username));
        profile.username = "bad\rname".into();
        assert_eq!(profile.validate(), Err(ProfileValidationError::Username));

        profile = valid_profile();
        profile.endpoint.port = 0;
        assert_eq!(profile.validate(), Err(ProfileValidationError::Port));

        profile = valid_profile();
        profile.auth = AuthConfig::PrivateKey {
            path: PathBuf::new(),
            passphrase_ref: None,
        };
        assert_eq!(
            profile.validate(),
            Err(ProfileValidationError::PrivateKeyPath)
        );
    }

    #[test]
    fn old_settings_default_local_working_directory() {
        let settings: AppSettings = serde_json::from_str(
            r#"{"locale":"system","theme":"system","newTabAction":"addConnection"}"#,
        )
        .unwrap();
        assert!(settings.local_working_directory.is_empty());
    }
}
