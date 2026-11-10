//! Loaded application state: config store, profiles, known hosts, search.

use std::{collections::HashSet, path::PathBuf, sync::Arc};

use oxide_ssh_core::{
    model::{
        AppSettings, ConnectionProfile, CursorStyleSetting, Endpoint, LocaleSetting, NewTabAction,
        Osc52Clipboard, Tag, TagColor, TagId, TerminalBell, ThemeMode,
    },
    storage::{AppConfig, AppStore, KnownHostEntry, KnownHosts, StorageError},
};

use crate::i18n::ResolvedLocale;
use crate::state::theme::ResolvedTheme;

pub mod connection_form;
pub mod theme;

pub enum AppLoadOutcome {
    Ready(Box<AppState>),
    Recovery(RecoveryState),
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RecoveryState {
    pub root: PathBuf,
}

pub struct AppState {
    store: Arc<AppStore>,
    config: AppConfig,
    known_hosts: KnownHosts,
    search_query: String,
    tag_filter: TagFilter,
}

/// Sidebar tag selection. Empty selection with `untagged == false` shows all
/// profiles; otherwise a profile matches if any of its tags is selected, or
/// it has no tags while `untagged` is selected. `untagged` is mutually
/// exclusive with tag selection: the toggle methods clear the other side.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct TagFilter {
    pub tags: HashSet<TagId>,
    pub untagged: bool,
}

impl TagFilter {
    pub fn is_active(&self) -> bool {
        self.untagged || !self.tags.is_empty()
    }

    fn matches(&self, profile: &ConnectionProfile) -> bool {
        if !self.is_active() {
            return true;
        }
        (self.untagged && profile.tag_ids.is_empty())
            || profile.tag_ids.iter().any(|id| self.tags.contains(id))
    }
}

impl AppState {
    pub fn load(root: PathBuf) -> AppLoadOutcome {
        let store = match AppStore::open(root.clone()) {
            Ok(store) => Arc::new(store),
            Err(_) => return AppLoadOutcome::Recovery(RecoveryState { root }),
        };
        let config = match store.load_config() {
            Ok(config) => config,
            Err(_) => return AppLoadOutcome::Recovery(RecoveryState { root }),
        };
        let known_hosts = match store.load_known_hosts() {
            Ok(hosts) => hosts,
            Err(_) => return AppLoadOutcome::Recovery(RecoveryState { root }),
        };

        AppLoadOutcome::Ready(Box::new(Self {
            store,
            config,
            known_hosts,
            search_query: String::new(),
            tag_filter: TagFilter::default(),
        }))
    }

    pub fn store(&self) -> &Arc<AppStore> {
        &self.store
    }

    pub fn settings(&self) -> &AppSettings {
        &self.config.settings
    }

    pub fn profiles(&self) -> &[ConnectionProfile] {
        &self.config.profiles
    }

    pub fn known_hosts(&self) -> &[KnownHostEntry] {
        &self.known_hosts.hosts
    }

    pub fn set_search_query(&mut self, query: impl Into<String>) {
        self.search_query = query.into();
    }

    pub fn filtered_profiles(&self) -> Vec<&ConnectionProfile> {
        let query = self.search_query.trim().to_lowercase();
        let mut profiles: Vec<_> = self
            .config
            .profiles
            .iter()
            .filter(|profile| {
                self.tag_filter.matches(profile)
                    && (query.is_empty()
                        || profile.name.to_lowercase().contains(&query)
                        || profile.endpoint.host.to_lowercase().contains(&query)
                        || profile.username.to_lowercase().contains(&query)
                        || profile.tag_ids.iter().any(|id| {
                            self.config
                                .tags
                                .iter()
                                .find(|tag| tag.id == *id)
                                .is_some_and(|tag| tag.name.to_lowercase().contains(&query))
                        }))
            })
            .collect();
        profiles.sort_by(|left, right| {
            left.name
                .to_lowercase()
                .cmp(&right.name.to_lowercase())
                .then_with(|| left.id.0.cmp(&right.id.0))
        });
        profiles
    }

    /// Registry entries sorted by case-insensitive name for display.
    pub fn tags(&self) -> Vec<&Tag> {
        let mut tags: Vec<_> = self.config.tags.iter().collect();
        tags.sort_by(|left, right| {
            left.name
                .to_lowercase()
                .cmp(&right.name.to_lowercase())
                .then_with(|| left.id.0.cmp(&right.id.0))
        });
        tags
    }

    pub fn tag(&self, id: TagId) -> Option<&Tag> {
        self.config.tags.iter().find(|tag| tag.id == id)
    }

    pub fn tag_filter(&self) -> &TagFilter {
        &self.tag_filter
    }

    pub fn toggle_tag_filter(&mut self, id: TagId) {
        if !self.tag_filter.tags.remove(&id) {
            self.tag_filter.tags.insert(id);
            self.tag_filter.untagged = false;
        }
    }

    pub fn toggle_untagged_filter(&mut self) {
        self.tag_filter.untagged = !self.tag_filter.untagged;
        if self.tag_filter.untagged {
            self.tag_filter.tags.clear();
        }
    }

    pub fn clear_tag_filter(&mut self) {
        self.tag_filter = TagFilter::default();
    }

    /// Profiles currently referencing the tag; used for delete confirmation.
    pub fn profiles_with_tag(&self, id: TagId) -> Vec<&ConnectionProfile> {
        self.config
            .profiles
            .iter()
            .filter(|profile| profile.tag_ids.contains(&id))
            .collect()
    }

    pub fn create_tag(
        &mut self,
        name: impl Into<String>,
        color: Option<TagColor>,
    ) -> Result<Tag, StorageError> {
        let mut tag = Tag::new(name.into().trim().to_string());
        tag.color = color;
        let mut config = self.config.clone();
        config.tags.push(tag.clone());
        self.store.save_config(&config)?;
        self.config = config;
        Ok(tag)
    }

    pub fn update_tag(
        &mut self,
        id: TagId,
        name: impl Into<String>,
        color: Option<TagColor>,
    ) -> Result<(), StorageError> {
        let mut config = self.config.clone();
        let Some(tag) = config.tags.iter_mut().find(|tag| tag.id == id) else {
            return Ok(());
        };
        tag.name = name.into().trim().to_string();
        tag.color = color;
        self.store.save_config(&config)?;
        self.config = config;
        Ok(())
    }

    /// Deletes the registry entry and removes the reference from every profile.
    pub fn delete_tag(&mut self, id: TagId) -> Result<(), StorageError> {
        let mut config = self.config.clone();
        config.tags.retain(|tag| tag.id != id);
        for profile in &mut config.profiles {
            profile.tag_ids.retain(|tag_id| *tag_id != id);
        }
        self.store.save_config(&config)?;
        self.config = config;
        self.tag_filter.tags.remove(&id);
        Ok(())
    }

    pub fn set_locale(&mut self, locale: LocaleSetting) -> Result<(), StorageError> {
        let mut config = self.config.clone();
        config.settings.locale = locale;
        self.store.save_config(&config)?;
        self.config = config;
        Ok(())
    }

    pub fn set_theme(&mut self, mode: ThemeMode) -> Result<(), StorageError> {
        let mut config = self.config.clone();
        config.settings.theme.mode = mode;
        self.store.save_config(&config)?;
        self.config = config;
        Ok(())
    }

    pub fn set_theme_file(
        &mut self,
        theme: ResolvedTheme,
        name: String,
    ) -> Result<(), StorageError> {
        let mut config = self.config.clone();
        match theme {
            ResolvedTheme::Light => config.settings.theme.light = name,
            ResolvedTheme::Dark => config.settings.theme.dark = name,
        }
        self.store.save_config(&config)?;
        self.config = config;
        Ok(())
    }

    pub fn set_new_tab_action(&mut self, action: NewTabAction) -> Result<(), StorageError> {
        let mut config = self.config.clone();
        config.settings.new_tab_action = action;
        self.store.save_config(&config)?;
        self.config = config;

        Ok(())
    }
    pub fn set_ssh_term(&mut self, term: &str) -> Result<(), StorageError> {
        let term = term.trim();
        if !oxide_ssh_core::model::valid_ssh_term(term) {
            return Ok(());
        }
        let mut config = self.config.clone();
        config.settings.ssh_term = term.to_owned();
        self.store.save_config(&config)?;
        self.config = config;
        Ok(())
    }

    pub fn set_osc52_clipboard(&mut self, setting: Osc52Clipboard) -> Result<(), StorageError> {
        let mut config = self.config.clone();
        config.settings.osc52_clipboard = setting;
        self.store.save_config(&config)?;
        self.config = config;
        Ok(())
    }
    pub fn set_terminal_settings(
        &mut self,
        font_family: String,
        font_size: u16,
        line_height: u16,
    ) -> Result<(), StorageError> {
        let mut config = self.config.clone();
        config.settings.terminal_font_family = font_family;
        config.settings.terminal_font_size = font_size;
        config.settings.terminal_line_height = line_height;
        self.store.save_config(&config)?;
        self.config = config;
        Ok(())
    }

    pub fn set_cursor_preferences(
        &mut self,
        style: CursorStyleSetting,
        blink: bool,
    ) -> Result<(), StorageError> {
        let mut config = self.config.clone();
        config.settings.cursor_style = style;
        config.settings.cursor_blink = blink;
        self.store.save_config(&config)?;
        self.config = config;
        Ok(())
    }

    pub fn set_ssh_compression(&mut self, enabled: bool) -> Result<(), StorageError> {
        let mut config = self.config.clone();
        config.settings.ssh_compression = enabled;
        self.store.save_config(&config)?;
        self.config = config;
        Ok(())
    }

    pub fn set_ansi_palette(&mut self, palette: Option<[u32; 16]>) -> Result<(), StorageError> {
        if palette.is_some_and(|colors| colors.iter().any(|color| color & 0xff00_0000 != 0)) {
            return Ok(());
        }
        let mut config = self.config.clone();
        config.settings.ansi_palette = palette;
        self.store.save_config(&config)?;
        self.config = config;
        Ok(())
    }
    pub fn set_ssh_connect_timeout_seconds(&mut self, seconds: u64) -> Result<(), StorageError> {
        if !(1..=300).contains(&seconds) {
            return Ok(());
        }
        let mut config = self.config.clone();
        config.settings.ssh_connect_timeout_seconds = seconds;
        self.store.save_config(&config)?;
        self.config = config;
        Ok(())
    }

    pub fn set_ssh_keepalive(
        &mut self,
        interval_seconds: u64,
        max_count: usize,
    ) -> Result<(), StorageError> {
        let mut config = self.config.clone();
        config.settings.ssh_keepalive_interval_seconds = interval_seconds;
        config.settings.ssh_keepalive_max_count = max_count;
        self.store.save_config(&config)?;
        self.config = config;
        Ok(())
    }

    pub fn set_confirm_before_close_with_sessions(
        &mut self,
        confirm: bool,
    ) -> Result<(), StorageError> {
        let mut config = self.config.clone();
        config.settings.confirm_before_close_with_sessions = confirm;
        self.store.save_config(&config)?;
        self.config = config;
        Ok(())
    }

    pub fn set_terminal_bell(&mut self, bell: TerminalBell) -> Result<(), StorageError> {
        let mut config = self.config.clone();
        config.settings.terminal_bell = bell;
        self.store.save_config(&config)?;
        self.config = config;
        Ok(())
    }

    pub fn set_paste_confirm_threshold(&mut self, threshold: usize) -> Result<(), StorageError> {
        let mut config = self.config.clone();
        config.settings.paste_confirm_threshold = threshold;
        self.store.save_config(&config)?;
        self.config = config;
        Ok(())
    }

    pub fn set_local_working_directory(&mut self, directory: String) -> Result<(), StorageError> {
        let mut config = self.config.clone();
        config.settings.local_working_directory = directory;
        self.store.save_config(&config)?;
        self.config = config;
        Ok(())
    }

    pub fn set_copy_on_select(&mut self, enabled: bool) -> Result<(), StorageError> {
        let mut config = self.config.clone();
        config.settings.copy_on_select = enabled;
        self.store.save_config(&config)?;
        self.config = config;
        Ok(())
    }

    pub fn set_remember_window_size(&mut self, remember: bool) -> Result<(), StorageError> {
        let mut config = self.config.clone();
        config.settings.remember_window_size = remember;
        if !remember {
            config.settings.window_size = None;
        }
        self.store.save_config(&config)?;
        self.config = config;
        Ok(())
    }

    pub fn set_window_size(&mut self, width: u32, height: u32) -> Result<(), StorageError> {
        if !self.config.settings.remember_window_size {
            return Ok(());
        }
        let mut config = self.config.clone();
        config.settings.window_size = Some(oxide_ssh_core::model::WindowSize { width, height });
        self.store.save_config(&config)?;
        self.config = config;
        Ok(())
    }

    pub fn set_terminal_scrollback_lines(&mut self, lines: usize) -> Result<(), StorageError> {
        if !(1..=1_000_000).contains(&lines) {
            return Err(StorageError::InvalidScrollbackLines);
        }
        let mut config = self.config.clone();
        config.settings.terminal_scrollback_lines = lines;
        self.store.save_config(&config)?;
        self.config = config;
        Ok(())
    }

    pub fn set_local_shell(&mut self, path: String, args: Vec<String>) -> Result<(), StorageError> {
        let mut config = self.config.clone();
        config.settings.local_shell_path = path;
        config.settings.local_shell_args = args;
        self.store.save_config(&config)?;
        self.config = config;
        Ok(())
    }

    pub fn reload_config(&mut self) -> Result<(), StorageError> {
        self.config = self.store.load_config()?;
        Ok(())
    }

    pub fn reload_known_hosts(&mut self) -> Result<(), StorageError> {
        self.known_hosts = self.store.load_known_hosts()?;
        Ok(())
    }

    pub fn delete_known_host(&mut self, endpoint: &Endpoint) -> Result<bool, StorageError> {
        let deleted = self.store.delete_known_host(endpoint)?;
        if deleted {
            self.reload_known_hosts()?;
        }
        Ok(deleted)
    }

    pub fn resolved_locale(&self, system_locale: Option<&str>) -> ResolvedLocale {
        ResolvedLocale::resolve(self.config.settings.locale, system_locale)
    }

    pub fn resolved_theme(&self, system_is_dark: bool) -> ResolvedTheme {
        ResolvedTheme::resolve(self.config.settings.theme.mode, system_is_dark)
    }

    pub fn theme_name(&self, theme: ResolvedTheme) -> &str {
        match theme {
            ResolvedTheme::Light => &self.config.settings.theme.light,
            ResolvedTheme::Dark => &self.config.settings.theme.dark,
        }
    }
}

#[cfg(test)]
mod tests {
    use std::fs;

    use oxide_ssh_core::{
        model::{AuthConfig, ProfileId, Tag, TagColor},
        storage::CONFIG_FILE_NAME,
    };
    use tempfile::tempdir;

    use super::*;

    fn profile(name: &str, host: &str, username: &str) -> ConnectionProfile {
        ConnectionProfile {
            id: ProfileId::new(),
            name: name.into(),
            endpoint: Endpoint {
                host: host.into(),
                port: 22,
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
        }
    }

    #[test]
    fn paste_confirmation_threshold_persists() {
        let directory = tempdir().unwrap();
        let AppLoadOutcome::Ready(mut state) = AppState::load(directory.path().to_path_buf())
        else {
            panic!("expected ready state");
        };
        state.set_paste_confirm_threshold(8192).unwrap();
        let AppLoadOutcome::Ready(reloaded) = AppState::load(directory.path().to_path_buf()) else {
            panic!("expected ready state");
        };
        assert_eq!(reloaded.settings().paste_confirm_threshold, 8192);
    }
    #[test]
    fn local_working_directory_setting_persists() {
        let directory = tempdir().unwrap();
        let AppLoadOutcome::Ready(mut state) = AppState::load(directory.path().to_path_buf())
        else {
            panic!("expected ready state");
        };
        state
            .set_local_working_directory("C:/work".to_owned())
            .unwrap();
        let stored = state.store().load_config().unwrap();
        assert_eq!(stored.settings.local_working_directory, "C:/work");
    }
    #[test]
    fn disabling_window_size_memory_clears_saved_dimensions() {
        let directory = tempdir().unwrap();
        let AppLoadOutcome::Ready(mut state) = AppState::load(directory.path().to_path_buf())
        else {
            panic!("fresh store should load");
        };
        state.set_remember_window_size(true).unwrap();
        state.set_window_size(1280, 800).unwrap();
        let AppLoadOutcome::Ready(state) = AppState::load(directory.path().to_path_buf()) else {
            panic!("saved store should load");
        };
        assert!(state.settings().remember_window_size);
        assert_eq!(
            state.settings().window_size,
            Some(oxide_ssh_core::model::WindowSize {
                width: 1280,
                height: 800
            })
        );
        let mut state = state;
        state.set_remember_window_size(false).unwrap();
        let AppLoadOutcome::Ready(state) = AppState::load(directory.path().to_path_buf()) else {
            panic!("saved store should load");
        };
        assert!(!state.settings().remember_window_size);
        assert_eq!(state.settings().window_size, None);
    }
    #[test]
    fn scrollback_setting_persists_and_rejects_out_of_range_values() {
        let directory = tempdir().unwrap();
        let AppLoadOutcome::Ready(mut state) = AppState::load(directory.path().to_path_buf())
        else {
            panic!("expected ready state");
        };

        assert!(state.set_terminal_scrollback_lines(0).is_err());
        assert!(state.set_terminal_scrollback_lines(1_000_001).is_err());
        assert_eq!(
            state.settings().terminal_scrollback_lines,
            oxide_ssh_core::model::DEFAULT_SCROLLBACK_LINES
        );
        state.set_terminal_scrollback_lines(250).unwrap();
        state.reload_config().unwrap();
        assert_eq!(state.settings().terminal_scrollback_lines, 250);
        assert_eq!(state.store().load_config().unwrap().schema_version(), 1);
    }
    #[test]
    fn untagged_filter_is_exclusive_with_tag_selection() {
        let directory = tempdir().unwrap();
        let store = AppStore::open(directory.path().to_path_buf()).unwrap();
        let mut config = store.load_config().unwrap();
        let prod = Tag::new("Prod");
        let db = Tag::new("Database");
        let mut web = profile("Web", "web.example", "oxide");
        web.tag_ids = vec![prod.id];
        let mut pg = profile("Postgres", "db.example", "oxide");
        pg.tag_ids = vec![prod.id, db.id];
        let plain = profile("Plain", "plain.example", "oxide");
        config.tags = vec![prod.clone(), db.clone()];
        config.profiles = vec![web, pg, plain];
        store.save_config(&config).unwrap();

        let AppLoadOutcome::Ready(mut state) = AppState::load(directory.path().to_path_buf())
        else {
            panic!("expected ready state");
        };
        assert_eq!(state.filtered_profiles().len(), 3);

        state.toggle_tag_filter(prod.id);
        assert_eq!(
            state
                .filtered_profiles()
                .iter()
                .map(|profile| profile.name.as_str())
                .collect::<Vec<_>>(),
            ["Postgres", "Web"]
        );

        // Union: adding Database keeps both, no new matches beyond Postgres.
        state.toggle_tag_filter(db.id);
        assert_eq!(state.filtered_profiles().len(), 2);

        // Untagged clears the tag selection and shows only Plain.
        state.toggle_untagged_filter();
        assert!(state.tag_filter().tags.is_empty());
        assert_eq!(
            state
                .filtered_profiles()
                .iter()
                .map(|profile| profile.name.as_str())
                .collect::<Vec<_>>(),
            ["Plain"]
        );

        // Selecting a tag while untagged is active clears untagged.
        state.toggle_tag_filter(prod.id);
        assert!(!state.tag_filter().untagged);
        assert_eq!(state.filtered_profiles().len(), 2);

        // Search intersects with the tag filter and matches tag names.
        state.clear_tag_filter();
        state.set_search_query("database");
        assert_eq!(state.filtered_profiles()[0].name, "Postgres");
        state.set_search_query("web");
        state.toggle_tag_filter(db.id);
        assert!(state.filtered_profiles().is_empty());
    }

    #[test]
    fn ansi_palette_is_persisted_and_invalid_values_are_ignored() {
        let directory = tempdir().unwrap();
        let AppLoadOutcome::Ready(mut state) = AppState::load(directory.path().to_path_buf())
        else {
            panic!("expected ready state");
        };
        let mut palette = [0x112233; 16];
        palette[1] = 0x123456;
        state.set_ansi_palette(Some(palette)).unwrap();
        assert_eq!(state.settings().ansi_palette, Some(palette));

        let mut invalid = palette;
        invalid[0] = 0x1_000000;
        state.set_ansi_palette(Some(invalid)).unwrap();
        assert_eq!(state.settings().ansi_palette, Some(palette));

        let AppLoadOutcome::Ready(reloaded) = AppState::load(directory.path().to_path_buf()) else {
            panic!("expected ready state");
        };
        assert_eq!(reloaded.settings().ansi_palette, Some(palette));
        assert_eq!(reloaded.config.schema_version(), 1);
    }

    #[test]
    fn tag_crud_cascades_delete_and_rejects_duplicate_names() {
        let directory = tempdir().unwrap();
        let store = AppStore::open(directory.path().to_path_buf()).unwrap();
        let mut config = store.load_config().unwrap();
        config.profiles = vec![profile("Web", "web.example", "oxide")];
        store.save_config(&config).unwrap();

        let AppLoadOutcome::Ready(mut state) = AppState::load(directory.path().to_path_buf())
        else {
            panic!("expected ready state");
        };
        let tag = state.create_tag(" Prod ", Some(TagColor::Green)).unwrap();
        assert_eq!(tag.name, "Prod");
        assert!(state.create_tag("prod", None).is_err());

        let profile_id = state.profiles()[0].id;
        state
            .store()
            .update_config(|config| config.profiles[0].tag_ids = vec![tag.id])
            .unwrap();
        state.reload_config().unwrap();
        assert_eq!(state.profiles_with_tag(tag.id).len(), 1);

        state.update_tag(tag.id, "Production", None).unwrap();
        assert_eq!(state.tag(tag.id).unwrap().name, "Production");
        assert_eq!(state.tag(tag.id).unwrap().color, None);

        state.toggle_tag_filter(tag.id);
        state.delete_tag(tag.id).unwrap();
        assert!(state.tags().is_empty());
        assert!(!state.tag_filter().is_active());
        let reloaded = store.load_config().unwrap();
        assert!(reloaded.tags.is_empty());
        assert_eq!(reloaded.profiles[0].id, profile_id);
        assert!(reloaded.profiles[0].tag_ids.is_empty());
    }

    #[test]
    fn corrupt_storage_opens_recovery_without_overwriting() {
        let directory = tempdir().unwrap();
        let path = directory.path().join(CONFIG_FILE_NAME);
        fs::write(&path, b"not json").unwrap();

        let outcome = AppState::load(directory.path().to_path_buf());
        assert!(matches!(outcome, AppLoadOutcome::Recovery(_)));
        assert_eq!(fs::read(path).unwrap(), b"not json");
    }

    #[test]
    fn search_is_trimmed_unicode_lowercase_and_sorted() {
        let directory = tempdir().unwrap();
        let store = AppStore::open(directory.path().to_path_buf()).unwrap();
        let mut config = store.load_config().unwrap();
        config.profiles = vec![
            profile("Zulu", "z.example", "root"),
            profile("北京", "beijing.example", "开发者"),
            profile("Alpha", "a.example", "oxide"),
        ];
        store.save_config(&config).unwrap();

        let AppLoadOutcome::Ready(mut state) = AppState::load(directory.path().to_path_buf())
        else {
            panic!("expected ready state");
        };
        assert_eq!(
            state
                .filtered_profiles()
                .into_iter()
                .map(|profile| profile.name.as_str())
                .collect::<Vec<_>>(),
            ["Alpha", "Zulu", "北京"],
        );

        state.set_search_query("  开发  ");
        assert_eq!(state.filtered_profiles()[0].name, "北京");
        state.set_search_query("EXAMPLE");
        assert_eq!(state.filtered_profiles().len(), 3);
    }

    #[test]
    fn loads_schema_one_config_without_ssh_timeout() {
        let directory = tempdir().unwrap();
        fs::write(
            directory.path().join(CONFIG_FILE_NAME),
            r#"{"schemaVersion":1,"settings":{"locale":"system","theme":"system"},"profiles":[]}"#,
        )
        .unwrap();
        let AppLoadOutcome::Ready(state) = AppState::load(directory.path().to_path_buf()) else {
            panic!("expected older config to load without recovery");
        };
        assert_eq!(state.settings().ssh_connect_timeout_seconds, 15);
    }

    #[test]
    fn ssh_connect_timeout_persists_and_old_config_defaults_to_fifteen_seconds() {
        let directory = tempdir().unwrap();
        let AppLoadOutcome::Ready(mut state) = AppState::load(directory.path().to_path_buf())
        else {
            panic!("expected ready state");
        };
        state.set_ssh_connect_timeout_seconds(45).unwrap();
        state.set_ssh_connect_timeout_seconds(0).unwrap();
        state.set_ssh_connect_timeout_seconds(301).unwrap();
        let stored = state.store().load_config().unwrap();
        assert_eq!(stored.settings.ssh_connect_timeout_seconds, 45);
        assert_eq!(stored.schema_version(), 1);
        drop(state);
        let AppLoadOutcome::Ready(state) = AppState::load(directory.path().to_path_buf()) else {
            panic!("expected ready state");
        };
        assert_eq!(state.settings().ssh_connect_timeout_seconds, 45);
    }

    #[test]
    fn settings_persist_immediately_and_resolve_system_defaults() {
        let directory = tempdir().unwrap();
        let AppLoadOutcome::Ready(mut state) = AppState::load(directory.path().to_path_buf())
        else {
            panic!("expected ready state");
        };

        state.set_locale(LocaleSetting::ZhCn).unwrap();
        state.set_theme(ThemeMode::Dark).unwrap();
        state
            .set_theme_file(ResolvedTheme::Light, "paper".into())
            .unwrap();
        state
            .set_theme_file(ResolvedTheme::Dark, "midnight".into())
            .unwrap();
        state
            .set_terminal_settings("Test Mono".into(), 15, 19)
            .unwrap();
        state
            .set_local_shell(
                "C:/Program Files/Git/bin/bash.exe".into(),
                vec!["--login".into()],
            )
            .unwrap();
        state.set_ssh_compression(true).unwrap();
        state.set_ssh_term("tmux-256color").unwrap();
        state.set_ssh_term("").unwrap();
        let stored = state.store().load_config().unwrap();
        assert_eq!(stored.schema_version(), 1);
        assert_eq!(stored.settings.locale, LocaleSetting::ZhCn);
        assert_eq!(stored.settings.theme.mode, ThemeMode::Dark);
        assert_eq!(stored.settings.theme.light, "paper");
        assert_eq!(stored.settings.theme.dark, "midnight");
        assert_eq!(stored.settings.terminal_font_family, "Test Mono");
        assert_eq!(stored.settings.terminal_font_size, 15);
        assert_eq!(stored.settings.terminal_line_height, 19);
        assert!(stored.settings.ssh_compression);
        state.set_confirm_before_close_with_sessions(false).unwrap();
        let stored = state.store().load_config().unwrap();
        assert_eq!(stored.schema_version(), 1);
        assert_eq!(stored.settings.locale, LocaleSetting::ZhCn);
        assert_eq!(stored.settings.theme.mode, ThemeMode::Dark);
        assert_eq!(
            stored.settings.local_shell_path,
            "C:/Program Files/Git/bin/bash.exe"
        );
        assert_eq!(stored.settings.local_shell_args, vec!["--login".to_owned()]);
        assert!(!stored.settings.confirm_before_close_with_sessions);
        state.set_ssh_keepalive(45, 4).unwrap();
        let stored = state.store().load_config().unwrap();
        assert_eq!(stored.settings.locale, LocaleSetting::ZhCn);
        assert_eq!(stored.settings.theme.mode, ThemeMode::Dark);
        assert_eq!(stored.settings.ssh_keepalive_interval_seconds, 45);
        assert_eq!(stored.settings.ssh_keepalive_max_count, 4);
        assert!(stored.settings.ssh_compression);
        state.set_confirm_before_close_with_sessions(false).unwrap();
        let stored = state.store().load_config().unwrap();
        assert_eq!(stored.settings.locale, LocaleSetting::ZhCn);
        assert_eq!(stored.settings.theme.mode, ThemeMode::Dark);
        assert!(!stored.settings.confirm_before_close_with_sessions);
        state.set_osc52_clipboard(Osc52Clipboard::Ask).unwrap();
        let stored = state.store().load_config().unwrap();
        assert_eq!(stored.settings.locale, LocaleSetting::ZhCn);
        assert_eq!(stored.settings.theme.mode, ThemeMode::Dark);
        assert_eq!(stored.settings.osc52_clipboard, Osc52Clipboard::Ask);
        assert_eq!(stored.schema_version(), 1);
        state.set_copy_on_select(true).unwrap();
        let stored = state.store().load_config().unwrap();
        assert_eq!(stored.settings.locale, LocaleSetting::ZhCn);
        assert_eq!(stored.settings.theme.mode, ThemeMode::Dark);
        assert!(stored.settings.copy_on_select);
        assert_eq!(stored.settings.ssh_term, "tmux-256color");
        assert_eq!(state.resolved_locale(Some("en-US")), ResolvedLocale::ZhCn);
        assert_eq!(state.resolved_theme(false), ResolvedTheme::Dark);
    }
}
