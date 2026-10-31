//! Add/edit connection form: field state, validation, and save requests.

use std::path::PathBuf;

use oxide_ssh_core::model::{
    AuthConfig, ConnectionProfile, Endpoint, EnvironmentVariable, ProfileId,
    ProfileValidationError, TagId,
};
use secrecy::SecretString;

use crate::credentials::SaveProfileRequest;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AuthMethod {
    Password,
    PrivateKey,
    Agent,
}

pub struct ConnectionForm {
    pub name: String,
    pub host: String,
    pub port: String,
    pub username: String,
    pub auth_method: AuthMethod,
    pub private_key_path: PathBuf,
    pub secret: String,
    pub remember: bool,
    pub tag_ids: Vec<TagId>,
    pub startup_command: String,
    pub ssh_keepalive_interval_seconds: String,
    pub ssh_keepalive_max_count: String,
    pub terminal_font_size: String,
    pub ansi_palette: String,
    pub environment_variables_json: String,
    previous: Option<ConnectionProfile>,
    profile_id: ProfileId,
}

impl ConnectionForm {
    pub fn new() -> Self {
        Self {
            name: String::new(),
            host: String::new(),
            port: "22".into(),
            username: String::new(),
            auth_method: AuthMethod::Password,
            private_key_path: PathBuf::new(),
            secret: String::new(),
            remember: false,
            tag_ids: Vec::new(),
            startup_command: String::new(),
            terminal_font_size: String::new(),
            ansi_palette: String::new(),
            ssh_keepalive_interval_seconds: String::new(),
            ssh_keepalive_max_count: String::new(),
            environment_variables_json: "[]".into(),
            previous: None,
            profile_id: ProfileId::new(),
        }
    }

    pub fn edit(profile: ConnectionProfile) -> Self {
        let (auth_method, private_key_path, remember) = match &profile.auth {
            AuthConfig::Password { credential_ref } => (
                AuthMethod::Password,
                PathBuf::new(),
                credential_ref.is_some(),
            ),
            AuthConfig::PrivateKey {
                path,
                passphrase_ref,
            } => (
                AuthMethod::PrivateKey,
                path.clone(),
                passphrase_ref.is_some(),
            ),
            AuthConfig::Agent => (AuthMethod::Agent, PathBuf::new(), false),
        };
        Self {
            name: profile.name.clone(),
            host: profile.endpoint.host.clone(),
            port: profile.endpoint.port.to_string(),
            username: profile.username.clone(),
            auth_method,
            private_key_path,
            secret: String::new(),
            remember,
            tag_ids: profile.tag_ids.clone(),
            startup_command: profile.startup_command.clone().unwrap_or_default(),
            ssh_keepalive_interval_seconds: profile
                .ssh_keepalive_interval_seconds
                .map(|value| value.to_string())
                .unwrap_or_default(),
            ssh_keepalive_max_count: profile
                .ssh_keepalive_max_count
                .map(|value| value.to_string())
                .unwrap_or_default(),
            terminal_font_size: profile
                .terminal_font_size
                .map(|value| value.to_string())
                .unwrap_or_default(),
            ansi_palette: profile
                .ansi_palette
                .map(|colors| {
                    colors
                        .iter()
                        .map(|color| format!("{color:06x}"))
                        .collect::<Vec<_>>()
                        .join(",")
                })
                .unwrap_or_default(),
            environment_variables_json: serde_json::to_string_pretty(
                &profile.environment_variables,
            )
            .expect("serializing environment variables cannot fail"),
            profile_id: profile.id,
            previous: Some(profile),
        }
    }

    pub fn is_editing(&self) -> bool {
        self.previous.is_some()
    }

    pub fn save_request(&self) -> Result<SaveProfileRequest, FormError> {
        let port = self
            .port
            .trim()
            .parse::<u16>()
            .ok()
            .filter(|port| *port != 0)
            .ok_or(FormError::Port)?;
        let auth =
            match self.auth_method {
                AuthMethod::Password => AuthConfig::Password {
                    credential_ref: self.previous.as_ref().and_then(|profile| {
                        match &profile.auth {
                            AuthConfig::Password { credential_ref } => credential_ref.clone(),
                            _ => None,
                        }
                    }),
                },
                AuthMethod::PrivateKey => AuthConfig::PrivateKey {
                    path: self.private_key_path.clone(),
                    passphrase_ref: self.previous.as_ref().and_then(|profile| {
                        match &profile.auth {
                            AuthConfig::PrivateKey { passphrase_ref, .. } => passphrase_ref.clone(),
                            _ => None,
                        }
                    }),
                },
                AuthMethod::Agent => AuthConfig::Agent,
            };
        let keepalive_interval = parse_override(&self.ssh_keepalive_interval_seconds)?;
        let keepalive_max_count = parse_override(&self.ssh_keepalive_max_count)?;
        let environment_variables: Vec<EnvironmentVariable> =
            serde_json::from_str(&self.environment_variables_json)
                .map_err(|_| FormError::EnvironmentVariable)?;
        let terminal_font_size = parse_override(&self.terminal_font_size)?
            .map(u16::try_from)
            .transpose()
            .map_err(|_| FormError::Keepalive)?;
        let ansi_palette = parse_palette(&self.ansi_palette)?;
        let profile = ConnectionProfile {
            id: self.profile_id,
            name: self.name.trim().into(),
            endpoint: Endpoint {
                host: self.host.trim().into(),
                port,
            },
            username: self.username.trim().into(),
            auth,
            tag_ids: self.tag_ids.clone(),
            startup_command: (!self.startup_command.trim().is_empty())
                .then(|| self.startup_command.trim().to_owned()),
            ssh_keepalive_interval_seconds: keepalive_interval,
            ssh_keepalive_max_count: keepalive_max_count
                .map(usize::try_from)
                .transpose()
                .map_err(|_| FormError::Keepalive)?,
            environment_variables,
            terminal_font_size,
            ansi_palette,
        };
        profile.validate().map_err(FormError::from)?;

        Ok(SaveProfileRequest {
            profile,
            previous: self.previous.clone(),
            secret: (!self.secret.is_empty()).then(|| SecretString::from(self.secret.clone())),
            remember: self.remember && self.auth_method != AuthMethod::Agent,
        })
    }
}

fn parse_override(value: &str) -> Result<Option<u64>, FormError> {
    if value.trim().is_empty() {
        return Ok(None);
    }
    value
        .trim()
        .parse::<u64>()
        .ok()
        .filter(|value| *value > 0)
        .map(Some)
        .ok_or(FormError::Keepalive)
}

fn parse_palette(value: &str) -> Result<Option<[u32; 16]>, FormError> {
    if value.trim().is_empty() {
        return Ok(None);
    }
    let values = value.split(',').map(str::trim).collect::<Vec<_>>();
    if values.len() != 16 {
        return Err(FormError::Keepalive);
    }
    let mut palette = [0; 16];
    for (slot, color) in palette.iter_mut().zip(values) {
        if color.len() != 6 {
            return Err(FormError::Keepalive);
        }
        *slot = u32::from_str_radix(color, 16).map_err(|_| FormError::Keepalive)?;
    }
    Ok(Some(palette))
}

impl Default for ConnectionForm {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum FormError {
    Name,
    Host,
    Username,
    Port,
    PrivateKeyPath,
    Keepalive,
    EnvironmentVariable,
}

impl From<ProfileValidationError> for FormError {
    fn from(error: ProfileValidationError) -> Self {
        match error {
            ProfileValidationError::Name => Self::Name,
            ProfileValidationError::Host => Self::Host,
            ProfileValidationError::Username => Self::Username,
            ProfileValidationError::Port => Self::Port,
            ProfileValidationError::PrivateKeyPath => Self::PrivateKeyPath,
            ProfileValidationError::Keepalive => Self::Keepalive,
            ProfileValidationError::EnvironmentVariable => Self::EnvironmentVariable,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn connection_form_is_shared_for_add_and_edit() {
        let mut add = ConnectionForm::new();
        add.name = " Fixture ".into();
        add.host = "EXAMPLE.COM".into();
        add.username = "oxide".into();
        add.auth_method = AuthMethod::Password;
        add.secret = "secret".into();
        add.remember = true;
        let request = add.save_request().unwrap();
        assert_eq!(request.profile.endpoint.port, 22);
        assert_eq!(request.profile.name, "Fixture");
        assert!(request.profile.startup_command.is_none());
        assert!(request.previous.is_none());
        assert!(request.remember);

        let id = request.profile.id;
        let edit = ConnectionForm::edit(request.profile);
        let edited = edit.save_request().unwrap();
        assert_eq!(edited.profile.id, id);
        assert!(edited.previous.is_some());
    }
    #[test]
    fn startup_command_is_saved_and_restored_when_editing_a_profile() {
        let mut form = ConnectionForm::new();
        form.name = "Fixture".into();
        form.host = "example.com".into();
        form.username = "oxide".into();
        form.startup_command = "tmux attach".into();
        let profile = form.save_request().unwrap().profile;
        assert_eq!(profile.startup_command.as_deref(), Some("tmux attach"));
        assert_eq!(
            ConnectionForm::edit(profile)
                .save_request()
                .unwrap()
                .profile
                .startup_command
                .as_deref(),
            Some("tmux attach")
        );
    }

    #[test]
    fn keepalive_form_overrides_round_trip_and_blank_inherits() {
        let mut form = ConnectionForm::new();
        form.name = "fixture".into();
        form.host = "example.com".into();
        form.username = "oxide".into();
        assert_eq!(
            form.save_request()
                .unwrap()
                .profile
                .ssh_keepalive_interval_seconds,
            None
        );
        form.ssh_keepalive_interval_seconds = "12".into();
        form.ssh_keepalive_max_count = "5".into();
        let saved = form.save_request().unwrap().profile;
        assert_eq!(saved.ssh_keepalive_interval_seconds, Some(12));
        assert_eq!(saved.ssh_keepalive_max_count, Some(5));
        let mut edit = ConnectionForm::edit(saved);
        assert_eq!(edit.ssh_keepalive_interval_seconds, "12");
        edit.ssh_keepalive_interval_seconds.clear();
        assert_eq!(
            edit.save_request()
                .unwrap()
                .profile
                .ssh_keepalive_interval_seconds,
            None
        );
        edit.ssh_keepalive_max_count = "0".into();
        assert_eq!(edit.save_request().unwrap_err(), FormError::Keepalive);
    }

    #[test]
    fn appearance_overrides_round_trip_and_clear_to_global_inheritance() {
        let mut form = ConnectionForm::new();
        form.name = "fixture".into();
        form.host = "example.com".into();
        form.username = "oxide".into();
        assert_eq!(
            form.save_request().unwrap().profile.terminal_font_size,
            None
        );
        assert_eq!(form.save_request().unwrap().profile.ansi_palette, None);

        form.terminal_font_size = "18".into();
        form.ansi_palette = (0..16)
            .map(|value| format!("{value:06x}"))
            .collect::<Vec<_>>()
            .join(",");
        let saved = form.save_request().unwrap().profile;
        assert_eq!(saved.terminal_font_size, Some(18));
        assert_eq!(saved.ansi_palette.unwrap()[15], 15);

        let mut edit = ConnectionForm::edit(saved);
        assert_eq!(edit.terminal_font_size, "18");
        edit.terminal_font_size.clear();
        edit.ansi_palette.clear();
        let cleared = edit.save_request().unwrap().profile;
        assert_eq!(cleared.terminal_font_size, None);
        assert_eq!(cleared.ansi_palette, None);
    }

    #[test]
    fn environment_variables_round_trip_without_trimming_and_invalid_names_do_not_save() {
        let mut form = ConnectionForm::new();
        form.name = "fixture".into();
        form.host = "host".into();
        form.username = "oxide".into();
        form.environment_variables_json =
            r#"[{"name":"BUILD_ID","value":"  release=blue\nwith newline  "}]"#.into();
        let profile = form.save_request().unwrap().profile;
        assert_eq!(profile.environment_variables[0].name, "BUILD_ID");
        assert_eq!(
            profile.environment_variables[0].value,
            "  release=blue\nwith newline  "
        );

        form.environment_variables_json = r#"[{"name":"BUILD-ID","value":"release"}]"#.into();
        assert_eq!(
            form.save_request().unwrap_err(),
            FormError::EnvironmentVariable
        );
    }

    #[test]
    fn connection_form_reports_boundary_errors() {
        let mut form = ConnectionForm::new();
        form.name = "fixture".into();
        form.host = "host".into();
        form.username = "oxide".into();
        form.port = "0".into();
        assert_eq!(form.save_request().unwrap_err(), FormError::Port);

        form.port = "22".into();
        form.auth_method = AuthMethod::PrivateKey;
        assert_eq!(form.save_request().unwrap_err(), FormError::PrivateKeyPath);
    }
}
