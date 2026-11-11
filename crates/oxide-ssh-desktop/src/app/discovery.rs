//! SSH/WSL discovery preview and local connection management.

use std::{collections::HashSet, io, path::PathBuf};

#[cfg(target_os = "windows")]
use oxide_ssh_core::discovery::discover_wsl_distributions;
use oxide_ssh_core::discovery::{
    DiscoveredSshHost, SshConfigDiscovery, SshConfigSkipReason, default_ssh_config_path,
    discover_ssh_config,
};

use super::*;

/// Identifies one candidate across both sources; names are compared
/// case-insensitively within a source but SSH and WSL entries stay separate.
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub(super) enum DiscoveryKey {
    Ssh(String),
    #[cfg(target_os = "windows")]
    Wsl(String),
}

impl DiscoveryKey {
    fn ssh(alias: &str) -> Self {
        Self::Ssh(alias.to_lowercase())
    }
    #[cfg(target_os = "windows")]
    fn wsl(distribution: &str) -> Self {
        Self::Wsl(distribution.to_lowercase())
    }
}

#[derive(Clone)]
pub(super) struct DiscoveryDialog {
    pub(super) generation: u64,
    pub(super) scanning: bool,
    pub(super) include_ssh: bool,
    pub(super) ssh_path: Option<PathBuf>,
    pub(super) ssh: Option<SshConfigDiscovery>,
    pub(super) ssh_error: Option<String>,
    #[cfg(target_os = "windows")]
    pub(super) include_wsl: bool,
    #[cfg(target_os = "windows")]
    pub(super) wsl: Option<Vec<String>>,
    #[cfg(target_os = "windows")]
    pub(super) wsl_error: Option<String>,
    pub(super) selected: HashSet<DiscoveryKey>,
}

impl DiscoveryDialog {
    fn new(generation: u64) -> Self {
        Self {
            generation,
            scanning: true,
            include_ssh: true,
            ssh_path: default_ssh_config_path(),
            ssh: None,
            ssh_error: None,
            #[cfg(target_os = "windows")]
            include_wsl: true,
            #[cfg(target_os = "windows")]
            wsl: None,
            #[cfg(target_os = "windows")]
            wsl_error: None,
            selected: HashSet::new(),
        }
    }

    pub(super) fn import_count(&self) -> usize {
        #[cfg(target_os = "windows")]
        let include_wsl = self.include_wsl;
        self.selected
            .iter()
            .filter(|key| match key {
                DiscoveryKey::Ssh(_) => self.include_ssh,
                #[cfg(target_os = "windows")]
                DiscoveryKey::Wsl(_) => include_wsl,
            })
            .count()
    }
}

#[derive(Clone)]
pub(super) struct LocalRenameDialog {
    pub(super) id: LocalConnectionId,
    pub(super) distribution: String,
    pub(super) name: Entity<InputState>,
}

struct DiscoveryScanResult {
    include_ssh: bool,
    ssh_path: Option<PathBuf>,
    ssh: Option<io::Result<SshConfigDiscovery>>,
    #[cfg(target_os = "windows")]
    wsl: Option<io::Result<Vec<String>>>,
}

fn default_selection(
    state: Option<&AppState>,
    include_ssh: bool,
    ssh: Option<&SshConfigDiscovery>,
    wsl: Option<&[String]>,
) -> HashSet<DiscoveryKey> {
    #[cfg(not(target_os = "windows"))]
    let _ = wsl;
    let existing_aliases: HashSet<String> = state
        .map(|state| {
            state
                .profiles()
                .iter()
                .map(|profile| profile.name.to_lowercase())
                .collect()
        })
        .unwrap_or_default();
    #[cfg(target_os = "windows")]
    let existing_distributions: HashSet<String> = state
        .map(|state| {
            state
                .local_connections()
                .iter()
                .map(|connection| connection.distribution.to_lowercase())
                .collect()
        })
        .unwrap_or_default();
    let mut selected = HashSet::new();
    if include_ssh && let Some(ssh) = ssh {
        for host in &ssh.hosts {
            if !existing_aliases.contains(&host.alias.to_lowercase()) {
                selected.insert(DiscoveryKey::ssh(&host.alias));
            }
        }
    }
    #[cfg(target_os = "windows")]
    if let Some(wsl) = wsl {
        for distribution in wsl {
            if !existing_distributions.contains(&distribution.to_lowercase()) {
                selected.insert(DiscoveryKey::wsl(distribution));
            }
        }
    }
    selected
}

fn discovered_profile(host: &DiscoveredSshHost) -> ConnectionProfile {
    ConnectionProfile {
        id: ProfileId::new(),
        name: host.alias.clone(),
        endpoint: Endpoint {
            host: host.hostname.clone(),
            port: host.port,
        },
        username: discovered_username(host),
        auth: match &host.identity_file {
            Some(path) => AuthConfig::PrivateKey {
                path: path.clone(),
                passphrase_ref: None,
            },
            None => AuthConfig::Agent,
        },
        tag_ids: Vec::new(),
        startup_command: None,
        ssh_keepalive_interval_seconds: None,
        ssh_keepalive_max_count: None,
        terminal_font_size: None,
        ansi_palette: None,
        environment_variables: Vec::new(),
    }
}

pub(super) fn discovered_username(host: &DiscoveredSshHost) -> String {
    host.username
        .clone()
        .filter(|username| !username.trim().is_empty())
        .or_else(|| std::env::var("USERNAME").ok())
        .or_else(|| std::env::var("USER").ok())
        .unwrap_or_default()
}

pub(super) fn skip_reason_text(locale: ResolvedLocale, reason: &SshConfigSkipReason) -> String {
    let text = |id| Catalog::text(locale, id);
    match reason {
        SshConfigSkipReason::WildcardHost => text(MessageId::DiscoverySkipWildcard).to_owned(),
        SshConfigSkipReason::Match => text(MessageId::DiscoverySkipMatch).to_owned(),
        SshConfigSkipReason::DuplicateAlias => text(MessageId::DiscoverySkipDuplicate).to_owned(),
        SshConfigSkipReason::MultipleIdentityFiles => {
            text(MessageId::DiscoverySkipMultipleIdentityFiles).to_owned()
        }
        SshConfigSkipReason::UnsupportedDirective { directive, value } => format!(
            "{}: {directive} {value}",
            text(MessageId::DiscoverySkipUnsupportedDirective)
        ),
        SshConfigSkipReason::InvalidValue { directive, value } => format!(
            "{}: {directive} {value}",
            text(MessageId::DiscoverySkipInvalidValue)
        ),
        SshConfigSkipReason::IncludeFailed(path) => {
            format!("{}: {path}", text(MessageId::DiscoverySkipIncludeFailed))
        }
    }
}

impl AppView {
    pub(super) fn open_discovery(&mut self, cx: &mut Context<Self>) {
        self.discovery_generation = self.discovery_generation.wrapping_add(1);
        self.discovery = Some(DiscoveryDialog::new(self.discovery_generation));
        self.start_discovery_scan(cx);
    }

    pub(super) fn close_discovery(&mut self, cx: &mut Context<Self>) {
        self.discovery = None;
        cx.notify();
    }

    pub(super) fn start_discovery_scan(&mut self, cx: &mut Context<Self>) {
        let Some(dialog) = &mut self.discovery else {
            return;
        };
        dialog.scanning = true;
        dialog.ssh_error = None;
        #[cfg(target_os = "windows")]
        {
            dialog.wsl_error = None;
        }
        let generation = dialog.generation;
        let include_ssh = dialog.include_ssh;
        #[cfg(target_os = "windows")]
        let include_wsl = dialog.include_wsl;
        cx.notify();
        cx.spawn(async move |weak, cx| {
            let result = cx
                .background_executor()
                .spawn(async move {
                    let ssh_path = default_ssh_config_path();
                    let ssh = if include_ssh {
                        ssh_path.as_ref().map(|path| discover_ssh_config(path))
                    } else {
                        None
                    };
                    #[cfg(target_os = "windows")]
                    let wsl = include_wsl.then(discover_wsl_distributions);
                    DiscoveryScanResult {
                        include_ssh,
                        ssh_path,
                        ssh,
                        #[cfg(target_os = "windows")]
                        wsl,
                    }
                })
                .await;
            weak.update(cx, |this, cx| {
                this.apply_discovery_scan(generation, result, cx)
            })
            .ok();
        })
        .detach();
    }

    fn apply_discovery_scan(
        &mut self,
        generation: u64,
        result: DiscoveryScanResult,
        cx: &mut Context<Self>,
    ) {
        let rescan = {
            let Some(dialog) = &mut self.discovery else {
                return;
            };
            if dialog.generation != generation {
                return;
            }
            dialog.scanning = false;
            dialog.ssh_path = result.ssh_path;
            if result.include_ssh && dialog.include_ssh {
                match result.ssh {
                    Some(Ok(discovery)) => {
                        dialog.ssh = Some(discovery);
                        dialog.ssh_error = None;
                    }
                    Some(Err(error)) => {
                        dialog.ssh = None;
                        dialog.ssh_error = Some(error.to_string());
                    }
                    None => {
                        dialog.ssh = None;
                        dialog.ssh_error = Some(
                            Catalog::text(self.locale, MessageId::DiscoverySshConfigUnavailable)
                                .to_owned(),
                        );
                    }
                }
            } else {
                dialog.ssh = None;
                dialog.ssh_error = None;
            }
            #[cfg(target_os = "windows")]
            match result.wsl {
                Some(Ok(distributions)) => {
                    dialog.wsl = Some(distributions);
                    dialog.wsl_error = None;
                }
                Some(Err(error)) => {
                    dialog.wsl = None;
                    dialog.wsl_error = Some(error.to_string());
                }
                None => {
                    dialog.wsl = None;
                    dialog.wsl_error = None;
                }
            }
            #[cfg(target_os = "windows")]
            let wsl = dialog.wsl.as_deref();
            #[cfg(not(target_os = "windows"))]
            let wsl: Option<&[String]> = None;
            let selected = default_selection(
                self.state.as_ref(),
                dialog.include_ssh,
                dialog.ssh.as_ref(),
                wsl,
            );
            dialog.selected = selected;
            dialog.include_ssh && !result.include_ssh
        };
        if rescan {
            self.start_discovery_scan(cx);
        }
        cx.notify();
    }

    pub(super) fn set_discovery_selection(
        &mut self,
        key: DiscoveryKey,
        selected: bool,
        cx: &mut Context<Self>,
    ) {
        if let Some(dialog) = &mut self.discovery {
            if selected {
                dialog.selected.insert(key);
            } else {
                dialog.selected.remove(&key);
            }
        }
        cx.notify();
    }

    pub(super) fn set_discovery_ssh_enabled(&mut self, enabled: bool, cx: &mut Context<Self>) {
        let rescan = if let Some(dialog) = &mut self.discovery {
            dialog.include_ssh = enabled;
            if !enabled {
                dialog.ssh = None;
                dialog.ssh_error = None;
            }
            enabled && dialog.ssh.is_none() && !dialog.scanning
        } else {
            false
        };
        if rescan {
            self.start_discovery_scan(cx);
        }
        cx.notify();
    }

    #[cfg(target_os = "windows")]
    pub(super) fn set_discovery_wsl_enabled(&mut self, enabled: bool, cx: &mut Context<Self>) {
        let rescan = if let Some(dialog) = &mut self.discovery {
            dialog.include_wsl = enabled;
            enabled && dialog.wsl.is_none() && !dialog.scanning
        } else {
            false
        };
        if rescan {
            self.start_discovery_scan(cx);
        }
        cx.notify();
    }

    pub(super) fn commit_discovery_import(&mut self, cx: &mut Context<Self>) {
        let Some(dialog) = self.discovery.as_ref() else {
            return;
        };
        let mut profiles = Vec::new();
        if dialog.include_ssh
            && let Some(ssh) = &dialog.ssh
        {
            for host in &ssh.hosts {
                if dialog.selected.contains(&DiscoveryKey::ssh(&host.alias)) {
                    profiles.push(discovered_profile(host));
                }
            }
        }
        #[cfg(target_os = "windows")]
        let mut local_connections = Vec::new();
        #[cfg(not(target_os = "windows"))]
        let local_connections = Vec::new();
        #[cfg(target_os = "windows")]
        if dialog.include_wsl
            && let Some(wsl) = &dialog.wsl
        {
            for distribution in wsl {
                if dialog.selected.contains(&DiscoveryKey::wsl(distribution)) {
                    local_connections.push(LocalConnection::new(
                        distribution.clone(),
                        distribution.clone(),
                    ));
                }
            }
        }
        if profiles.is_empty() && local_connections.is_empty() {
            return;
        }
        match self
            .state
            .as_mut()
            .map(|state| state.import_discovered(profiles, local_connections))
        {
            Some(Ok(())) => {
                self.discovery = None;
                self.status_message = Some(MessageId::DiscoveryImportSuccess);
            }
            Some(Err(_)) => self.status_message = Some(MessageId::DiscoveryImportFailed),
            None => self.status_message = Some(MessageId::StorageCorrupt),
        }
        cx.notify();
    }

    pub(super) fn start_local_rename(
        &mut self,
        connection: LocalConnection,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let name = new_input(
            window,
            cx,
            MessageId::Name,
            &connection.name,
            false,
            self.locale,
        );
        self.local_rename = Some(LocalRenameDialog {
            id: connection.id,
            distribution: connection.distribution,
            name,
        });
        cx.notify();
    }

    pub(super) fn commit_local_rename(&mut self, _: &mut Window, cx: &mut Context<Self>) {
        let Some(dialog) = &self.local_rename else {
            return;
        };
        let id = dialog.id;
        let name = dialog.name.read(cx).value().trim().to_string();
        let name_len = name.chars().count();
        if !(1..=64).contains(&name_len) {
            self.status_message = Some(MessageId::InvalidLocalConnectionName);
            cx.notify();
            return;
        }
        match self
            .state
            .as_mut()
            .map(|state| state.rename_local_connection(id, name))
        {
            Some(Ok(())) => self.local_rename = None,
            Some(Err(_)) => self.status_message = Some(MessageId::StorageCorrupt),
            None => self.status_message = Some(MessageId::StorageCorrupt),
        }
        cx.notify();
    }

    pub(super) fn cancel_local_rename(&mut self, cx: &mut Context<Self>) {
        self.local_rename = None;
        cx.notify();
    }
}
