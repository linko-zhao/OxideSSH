use std::{
    collections::HashSet,
    fs,
    io::{self, Write},
    net::IpAddr,
    path::{Path, PathBuf},
};

#[cfg(unix)]
use std::fs::File;

use parking_lot::Mutex;
use serde::{Deserialize, Serialize, de::DeserializeOwned};
use tempfile::Builder;
use thiserror::Error;

use crate::model::{
    AppSettings, AuthConfig, ConnectionProfile, CredentialRef, Endpoint, LocalConnection,
    LocalConnectionId, LocalConnectionValidationError, ProfileId, ProfileValidationError, Tag,
    TagId, TagValidationError,
};

pub const CONFIG_FILE_NAME: &str = "config.json";
pub const KNOWN_HOSTS_FILE_NAME: &str = "known_hosts.json";
const SCHEMA_VERSION: u32 = 1;

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AppConfig {
    schema_version: u32,
    pub settings: AppSettings,
    pub profiles: Vec<ConnectionProfile>,
    #[serde(default)]
    pub local_connections: Vec<LocalConnection>,
    #[serde(default)]
    pub tags: Vec<Tag>,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            schema_version: SCHEMA_VERSION,
            settings: AppSettings::default(),
            profiles: Vec::new(),
            local_connections: Vec::new(),
            tags: Vec::new(),
        }
    }
}

impl AppConfig {
    pub fn schema_version(&self) -> u32 {
        self.schema_version
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct KnownHosts {
    schema_version: u32,
    pub hosts: Vec<KnownHostEntry>,
}

impl Default for KnownHosts {
    fn default() -> Self {
        Self {
            schema_version: SCHEMA_VERSION,
            hosts: Vec::new(),
        }
    }
}

impl KnownHosts {
    pub fn schema_version(&self) -> u32 {
        self.schema_version
    }
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum KnownHostsConflictPolicy {
    KeepExisting,
    ReplaceExisting,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct KnownHostEntry {
    pub host: String,
    pub port: u16,
    pub algorithm: String,
    pub public_key: String,
    pub fingerprint_sha256: String,
    pub accepted_at_unix: i64,
}

impl KnownHostEntry {
    pub fn endpoint(&self) -> Endpoint {
        Endpoint {
            host: self.host.clone(),
            port: self.port,
        }
    }
}

#[derive(Debug, Error)]
pub enum StorageError {
    #[error("unsupported schema version {found} in {path}")]
    UnsupportedSchema { path: PathBuf, found: u64 },
    #[error("corrupt storage file at {path}")]
    Corrupt {
        path: PathBuf,
        #[source]
        source: serde_json::Error,
    },
    #[error("unable to access storage file at {path}")]
    Io {
        path: PathBuf,
        #[source]
        source: io::Error,
    },
    #[error("unable to serialize storage file at {path}")]
    Serialize {
        path: PathBuf,
        #[source]
        source: serde_json::Error,
    },
    #[error("invalid host endpoint")]
    InvalidEndpoint,
    #[error("invalid terminal scrollback line count")]
    InvalidScrollbackLines,
    #[error("duplicate profile id {id:?}")]
    DuplicateProfile { id: ProfileId },
    #[error("duplicate local connection id {id:?}")]
    DuplicateLocalConnection { id: LocalConnectionId },
    #[error("duplicate WSL distribution {name:?}")]
    DuplicateLocalDistribution { name: String },
    #[error("duplicate tag id {id:?}")]
    DuplicateTag { id: TagId },
    #[error("duplicate tag name {name:?}")]
    DuplicateTagName { name: String },
    #[error("profile {profile:?} references unknown tag {tag:?}")]
    UnknownTagRef { profile: ProfileId, tag: TagId },
    #[error("invalid credential reference in profile {id:?}")]
    InvalidCredentialRef { id: ProfileId },
    #[error("known hosts import contains an invalid host key or fingerprint")]
    InvalidKnownHostImport,
    #[error("known hosts import contains duplicate endpoints")]
    DuplicateKnownHost,
    #[error("known hosts import file has an unsupported schema version {found}")]
    UnsupportedKnownHostsImportSchema { found: u64 },
    #[error("known hosts import file is corrupt")]
    CorruptKnownHostsImport(#[source] serde_json::Error),
    #[error("unable to access known hosts import/export file at {path}")]
    KnownHostsImportExportIo {
        path: PathBuf,
        #[source]
        source: io::Error,
    },
    #[error("host key changed while it was being accepted")]
    HostKeyConflict,
    #[error(transparent)]
    InvalidProfile(#[from] ProfileValidationError),
    #[error(transparent)]
    InvalidLocalConnection(#[from] LocalConnectionValidationError),
    #[error(transparent)]
    InvalidTag(#[from] TagValidationError),
}

pub struct AppStore {
    root: PathBuf,
    config_lock: Mutex<()>,
    known_hosts_lock: Mutex<()>,
}

impl AppStore {
    pub fn open(root: PathBuf) -> Result<Self, StorageError> {
        fs::create_dir_all(&root).map_err(|source| StorageError::Io {
            path: root.clone(),
            source,
        })?;
        Ok(Self {
            root,
            config_lock: Mutex::new(()),
            known_hosts_lock: Mutex::new(()),
        })
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    pub fn config_path(&self) -> PathBuf {
        self.root.join(CONFIG_FILE_NAME)
    }

    pub fn known_hosts_path(&self) -> PathBuf {
        self.root.join(KNOWN_HOSTS_FILE_NAME)
    }

    pub fn load_config(&self) -> Result<AppConfig, StorageError> {
        let _guard = self.config_lock.lock();
        let mut config = read_versioned(&self.config_path(), AppConfig::default)?;
        prune_dangling_tag_refs(&mut config);
        validate_config(&config)?;
        Ok(config)
    }

    pub fn export_config(&self, path: &Path) -> Result<(), StorageError> {
        let mut config = self.load_config()?;
        clear_credential_refs(&mut config);
        let root = path
            .parent()
            .filter(|parent| !parent.as_os_str().is_empty())
            .unwrap_or_else(|| Path::new("."));
        atomic_write(root, path, &config)
    }

    pub fn import_config(&self, path: &Path) -> Result<(), StorageError> {
        let bytes = fs::read(path).map_err(|source| StorageError::Io {
            path: path.to_path_buf(),
            source,
        })?;
        let document: serde_json::Value =
            serde_json::from_slice(&bytes).map_err(|source| StorageError::Corrupt {
                path: path.to_path_buf(),
                source,
            })?;
        let palette = document
            .get("settings")
            .and_then(|settings| settings.get("ansiPalette"));
        if let Some(palette) = palette {
            serde_json::from_value::<Option<[u32; 16]>>(palette.clone()).map_err(|source| {
                StorageError::Corrupt {
                    path: path.to_path_buf(),
                    source,
                }
            })?;
        }
        let mut config: AppConfig =
            serde_json::from_value(document).map_err(|source| StorageError::Corrupt {
                path: path.to_path_buf(),
                source,
            })?;
        validate_config(&config)?;
        clear_credential_refs(&mut config);
        self.save_config(&config)
    }

    pub fn backup_config(&self, path: &Path) -> Result<(), StorageError> {
        self.export_config(path)
    }

    pub fn import_discovered(
        &self,
        profiles: Vec<ConnectionProfile>,
        local_connections: Vec<LocalConnection>,
    ) -> Result<(), StorageError> {
        self.update_config(|config| {
            config.profiles.extend(profiles);
            config.local_connections.extend(local_connections);
        })
    }

    pub fn save_config(&self, config: &AppConfig) -> Result<(), StorageError> {
        let _guard = self.config_lock.lock();
        let path = self.config_path();
        guard_existing(&path, AppConfig::default)?;
        validate_config(config)?;
        atomic_write(&self.root, &path, config)
    }

    pub fn update_config(&self, update: impl FnOnce(&mut AppConfig)) -> Result<(), StorageError> {
        let _guard = self.config_lock.lock();
        let path = self.config_path();
        let mut config = read_versioned(&path, AppConfig::default)?;
        update(&mut config);
        validate_config(&config)?;
        atomic_write(&self.root, &path, &config)
    }

    pub fn export_known_hosts(&self, path: &Path) -> Result<(), StorageError> {
        let known_hosts = self.load_known_hosts()?;
        let directory = path
            .parent()
            .filter(|parent| !parent.as_os_str().is_empty())
            .unwrap_or_else(|| Path::new("."));
        atomic_write(directory, path, &known_hosts)
    }

    pub fn import_known_hosts(
        &self,
        path: &Path,
        policy: KnownHostsConflictPolicy,
    ) -> Result<(), StorageError> {
        let bytes = fs::read(path).map_err(|source| StorageError::KnownHostsImportExportIo {
            path: path.to_path_buf(),
            source,
        })?;
        let imported: KnownHosts =
            serde_json::from_slice(&bytes).map_err(StorageError::CorruptKnownHostsImport)?;
        if imported.schema_version != SCHEMA_VERSION {
            return Err(StorageError::UnsupportedKnownHostsImportSchema {
                found: u64::from(imported.schema_version),
            });
        }
        let mut incoming = Vec::with_capacity(imported.hosts.len());
        let mut seen = HashSet::new();
        for mut host in imported.hosts {
            let endpoint = canonicalize_endpoint(&host.endpoint())?;
            host.host = endpoint.host;
            host.port = endpoint.port;
            if !seen.insert((host.host.clone(), host.port)) {
                return Err(StorageError::DuplicateKnownHost);
            }
            if host.algorithm.is_empty()
                || host.public_key.is_empty()
                || host.fingerprint_sha256.is_empty()
            {
                return Err(StorageError::InvalidKnownHostImport);
            }
            let key = russh::keys::PublicKey::from_openssh(&host.public_key)
                .map_err(|_| StorageError::InvalidKnownHostImport)?;
            if key.algorithm().to_string() != host.algorithm
                || key.fingerprint(russh::keys::HashAlg::Sha256).to_string()
                    != host.fingerprint_sha256
            {
                return Err(StorageError::InvalidKnownHostImport);
            }
            incoming.push(host);
        }
        let _guard = self.known_hosts_lock.lock();
        let destination = self.known_hosts_path();
        let mut current = read_versioned(&destination, KnownHosts::default)?;
        for host in incoming {
            if let Some(existing) = current
                .hosts
                .iter_mut()
                .find(|known| known.host == host.host && known.port == host.port)
            {
                if existing.algorithm == host.algorithm
                    && existing.public_key == host.public_key
                    && existing.fingerprint_sha256 == host.fingerprint_sha256
                {
                    continue;
                }
                match policy {
                    KnownHostsConflictPolicy::KeepExisting => continue,
                    KnownHostsConflictPolicy::ReplaceExisting => *existing = host,
                }
            } else {
                current.hosts.push(host);
            }
        }
        atomic_write(&self.root, &destination, &current)
    }

    pub fn load_known_hosts(&self) -> Result<KnownHosts, StorageError> {
        let _guard = self.known_hosts_lock.lock();
        read_versioned(&self.known_hosts_path(), KnownHosts::default)
    }

    pub fn known_host(&self, endpoint: &Endpoint) -> Result<Option<KnownHostEntry>, StorageError> {
        let endpoint = canonicalize_endpoint(endpoint)?;
        let hosts = self.load_known_hosts()?;
        Ok(hosts
            .hosts
            .into_iter()
            .find(|host| host.host == endpoint.host && host.port == endpoint.port))
    }

    pub fn accept_host_key(&self, mut entry: KnownHostEntry) -> Result<(), StorageError> {
        let _guard = self.known_hosts_lock.lock();
        let path = self.known_hosts_path();
        let mut hosts = read_versioned(&path, KnownHosts::default)?;
        let endpoint = canonicalize_endpoint(&entry.endpoint())?;
        entry.host = endpoint.host;
        entry.port = endpoint.port;
        if let Some(existing) = hosts
            .hosts
            .iter()
            .find(|host| host.host == entry.host && host.port == entry.port)
        {
            if existing.algorithm == entry.algorithm
                && existing.public_key == entry.public_key
                && existing.fingerprint_sha256 == entry.fingerprint_sha256
            {
                return Ok(());
            }
            return Err(StorageError::HostKeyConflict);
        }
        hosts.hosts.push(entry);
        atomic_write(&self.root, &path, &hosts)
    }

    pub fn delete_known_host(&self, endpoint: &Endpoint) -> Result<bool, StorageError> {
        let _guard = self.known_hosts_lock.lock();
        let path = self.known_hosts_path();
        let mut hosts = read_versioned(&path, KnownHosts::default)?;
        let endpoint = canonicalize_endpoint(endpoint)?;
        let original_len = hosts.hosts.len();
        hosts
            .hosts
            .retain(|host| host.host != endpoint.host || host.port != endpoint.port);
        if hosts.hosts.len() == original_len {
            return Ok(false);
        }
        atomic_write(&self.root, &path, &hosts)?;
        Ok(true)
    }
}

pub fn canonicalize_endpoint(endpoint: &Endpoint) -> Result<Endpoint, StorageError> {
    if endpoint.port == 0 {
        return Err(StorageError::InvalidEndpoint);
    }

    let unbracketed = endpoint
        .host
        .strip_prefix('[')
        .and_then(|host| host.strip_suffix(']'))
        .unwrap_or(&endpoint.host);
    let bytes = unbracketed.as_bytes();
    if bytes.is_empty()
        || bytes.len() > 253
        || !bytes.is_ascii()
        || bytes
            .iter()
            .any(|byte| byte.is_ascii_whitespace() || matches!(byte, 0 | b'/' | b'\\'))
    {
        return Err(StorageError::InvalidEndpoint);
    }

    let host = match unbracketed.parse::<IpAddr>() {
        Ok(address) => address.to_string(),
        Err(_) => unbracketed.to_ascii_lowercase(),
    };
    Ok(Endpoint {
        host,
        port: endpoint.port,
    })
}

fn validate_config(config: &AppConfig) -> Result<(), StorageError> {
    if config.schema_version != SCHEMA_VERSION {
        return Err(StorageError::UnsupportedSchema {
            path: PathBuf::from(CONFIG_FILE_NAME),
            found: u64::from(config.schema_version),
        });
    }
    let mut tag_ids = HashSet::new();
    let mut tag_names = HashSet::new();
    for tag in &config.tags {
        if !tag_ids.insert(tag.id) {
            return Err(StorageError::DuplicateTag { id: tag.id });
        }
        tag.validate()?;
        if !tag_names.insert(tag.name.trim().to_lowercase()) {
            return Err(StorageError::DuplicateTagName {
                name: tag.name.clone(),
            });
        }
    }
    let mut local_ids = HashSet::new();
    let mut distributions = HashSet::new();
    for connection in &config.local_connections {
        if !local_ids.insert(connection.id) {
            return Err(StorageError::DuplicateLocalConnection { id: connection.id });
        }
        connection.validate()?;
        if !distributions.insert(connection.distribution.to_lowercase()) {
            return Err(StorageError::DuplicateLocalDistribution {
                name: connection.distribution.clone(),
            });
        }
    }
    let mut seen = HashSet::new();
    for profile in &config.profiles {
        if !seen.insert(profile.id) {
            return Err(StorageError::DuplicateProfile { id: profile.id });
        }
        for tag_id in &profile.tag_ids {
            if !tag_ids.contains(tag_id) {
                return Err(StorageError::UnknownTagRef {
                    profile: profile.id,
                    tag: *tag_id,
                });
            }
        }
        profile.validate()?;
        match &profile.auth {
            AuthConfig::Password { credential_ref } => {
                if let Some(reference) = credential_ref
                    && *reference != CredentialRef::password(profile.id)
                {
                    return Err(StorageError::InvalidCredentialRef { id: profile.id });
                }
            }
            AuthConfig::PrivateKey { passphrase_ref, .. } => {
                if let Some(reference) = passphrase_ref
                    && *reference != CredentialRef::private_key_passphrase(profile.id)
                {
                    return Err(StorageError::InvalidCredentialRef { id: profile.id });
                }
            }
            AuthConfig::Agent => {}
        }
    }
    Ok(())
}

/// Drops tag references that have no registry entry; used when loading
/// configs written before the registry entry was removed externally.
fn prune_dangling_tag_refs(config: &mut AppConfig) {
    let valid: HashSet<TagId> = config.tags.iter().map(|tag| tag.id).collect();
    for profile in &mut config.profiles {
        profile.tag_ids.retain(|id| valid.contains(id));
    }
}

fn clear_credential_refs(config: &mut AppConfig) {
    for profile in &mut config.profiles {
        match &mut profile.auth {
            AuthConfig::Password { credential_ref } => *credential_ref = None,
            AuthConfig::PrivateKey { passphrase_ref, .. } => *passphrase_ref = None,
            AuthConfig::Agent => {}
        }
    }
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct SchemaProbe {
    schema_version: u64,
}

fn guard_existing<T>(path: &Path, default: impl FnOnce() -> T) -> Result<(), StorageError>
where
    T: DeserializeOwned,
{
    if path.try_exists().map_err(|source| StorageError::Io {
        path: path.to_path_buf(),
        source,
    })? {
        let _: T = read_versioned(path, default)?;
    }
    Ok(())
}

fn read_versioned<T>(path: &Path, default: impl FnOnce() -> T) -> Result<T, StorageError>
where
    T: DeserializeOwned,
{
    let bytes = match fs::read(path) {
        Ok(bytes) => bytes,
        Err(source) if source.kind() == io::ErrorKind::NotFound => return Ok(default()),
        Err(source) => {
            return Err(StorageError::Io {
                path: path.to_path_buf(),
                source,
            });
        }
    };
    let probe: SchemaProbe =
        serde_json::from_slice(&bytes).map_err(|source| StorageError::Corrupt {
            path: path.to_path_buf(),
            source,
        })?;
    if probe.schema_version != u64::from(SCHEMA_VERSION) {
        return Err(StorageError::UnsupportedSchema {
            path: path.to_path_buf(),
            found: probe.schema_version,
        });
    }
    serde_json::from_slice(&bytes).map_err(|source| StorageError::Corrupt {
        path: path.to_path_buf(),
        source,
    })
}

fn atomic_write<T: Serialize>(root: &Path, path: &Path, value: &T) -> Result<(), StorageError> {
    let mut temporary = Builder::new()
        .prefix(".oxide-ssh-")
        .tempfile_in(root)
        .map_err(|source| StorageError::Io {
            path: root.to_path_buf(),
            source,
        })?;

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        temporary
            .as_file()
            .set_permissions(fs::Permissions::from_mode(0o600))
            .map_err(|source| StorageError::Io {
                path: temporary.path().to_path_buf(),
                source,
            })?;
    }

    serde_json::to_writer_pretty(temporary.as_file_mut(), value).map_err(|source| {
        StorageError::Serialize {
            path: path.to_path_buf(),
            source,
        }
    })?;
    temporary
        .as_file_mut()
        .write_all(b"\n")
        .map_err(|source| StorageError::Io {
            path: temporary.path().to_path_buf(),
            source,
        })?;
    temporary
        .as_file()
        .sync_all()
        .map_err(|source| StorageError::Io {
            path: temporary.path().to_path_buf(),
            source,
        })?;
    temporary.persist(path).map_err(|error| StorageError::Io {
        path: path.to_path_buf(),
        source: error.error,
    })?;

    #[cfg(unix)]
    File::open(root)
        .and_then(|directory| directory.sync_all())
        .map_err(|source| StorageError::Io {
            path: root.to_path_buf(),
            source,
        })?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use std::{fs, path::PathBuf, sync::Arc, thread};

    use tempfile::tempdir;

    use super::*;
    use crate::model::{
        AuthConfig, ConnectionProfile, CredentialRef, EnvironmentVariable, LocalConnection,
        LocaleSetting, ProfileId, Tag, TagId, ThemeMode,
    };

    fn profile(name: &str) -> ConnectionProfile {
        let id = ProfileId::new();
        ConnectionProfile {
            id,
            name: name.into(),
            endpoint: Endpoint {
                host: "EXAMPLE.COM".into(),
                port: 22,
            },
            username: "oxide".into(),
            auth: AuthConfig::Password {
                credential_ref: Some(CredentialRef::password(id)),
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

    fn host(host: &str, port: u16) -> KnownHostEntry {
        KnownHostEntry {
            host: host.into(),
            port,
            algorithm: "ssh-ed25519".into(),
            public_key: "ssh-ed25519 AAAAC3NzaC1lZDI1NTE5AAAAITest".into(),
            fingerprint_sha256: "SHA256:test".into(),
            accepted_at_unix: 1,
        }
    }

    #[test]
    fn invalid_discovery_batch_does_not_partially_import() {
        let directory = tempdir().unwrap();
        let store = AppStore::open(directory.path().to_path_buf()).unwrap();
        let mut config = AppConfig::default();
        config.profiles.push(profile("existing"));
        store.save_config(&config).unwrap();
        let persisted = fs::read(store.config_path()).unwrap();

        let result = store.import_discovered(
            vec![profile("new host")],
            vec![LocalConnection::new("", "Ubuntu")],
        );

        assert!(matches!(
            result,
            Err(StorageError::InvalidLocalConnection(
                LocalConnectionValidationError::Name
            ))
        ));
        assert_eq!(fs::read(store.config_path()).unwrap(), persisted);
    }

    #[test]
    fn config_export_import_preserves_data_but_drops_credentials() {
        let source_dir = tempdir().unwrap();
        let target_dir = tempdir().unwrap();
        let source = AppStore::open(source_dir.path().to_path_buf()).unwrap();
        let mut config = AppConfig::default();
        config.settings.locale = LocaleSetting::ZhCn;
        config.settings.theme.mode = ThemeMode::Dark;
        let tag = Tag::new("Migration");
        config.tags.push(tag.clone());
        config
            .local_connections
            .push(LocalConnection::new("Ubuntu", "Ubuntu"));
        let mut migration_profile = profile("migration");
        migration_profile.tag_ids.push(tag.id);
        config.profiles.push(migration_profile);
        let mut key_profile = profile("private key");
        let key_profile_id = key_profile.id;
        key_profile.auth = AuthConfig::PrivateKey {
            path: PathBuf::from("keys/id_ed25519"),
            passphrase_ref: Some(CredentialRef::private_key_passphrase(key_profile_id)),
        };
        config.profiles.push(key_profile);
        source.save_config(&config).unwrap();
        let file = target_dir.path().join("export.json");
        source.export_config(&file).unwrap();
        let exported: serde_json::Value =
            serde_json::from_slice(&fs::read(&file).unwrap()).unwrap();
        assert!(exported["profiles"][0]["auth"]["credentialRef"].is_null());
        assert!(exported["profiles"][1]["auth"]["passphraseRef"].is_null());
        let backup = target_dir.path().join("backup.json");
        source.backup_config(&backup).unwrap();
        assert_eq!(fs::read(&backup).unwrap(), fs::read(&file).unwrap());
        let target = AppStore::open(target_dir.path().join("store")).unwrap();
        target.import_config(&file).unwrap();
        let reopened = AppStore::open(target_dir.path().join("store")).unwrap();
        let imported = reopened.load_config().unwrap();
        assert_eq!(imported.profiles.len(), 2);
        assert_eq!(imported.profiles[0].name, "migration");
        assert_eq!(imported.settings.locale, LocaleSetting::ZhCn);
        assert_eq!(imported.settings.theme.mode, ThemeMode::Dark);
        assert_eq!(imported.tags, vec![tag.clone()]);
        assert_eq!(imported.local_connections.len(), 1);
        assert_eq!(imported.local_connections[0].name, "Ubuntu");
        assert_eq!(imported.local_connections[0].distribution, "Ubuntu");
        assert_eq!(imported.profiles[0].tag_ids, vec![tag.id]);
        assert!(matches!(
            imported.profiles[0].auth,
            AuthConfig::Password {
                credential_ref: None
            }
        ));
        assert!(matches!(
            imported.profiles[1].auth,
            AuthConfig::PrivateKey {
                passphrase_ref: None,
                ..
            }
        ));
        let legacy = target_dir.path().join("legacy.json");
        fs::write(&legacy, serde_json::to_vec(&config).unwrap()).unwrap();
        target.import_config(&legacy).unwrap();
        assert!(matches!(
            target.load_config().unwrap().profiles[0].auth,
            AuthConfig::Password {
                credential_ref: None
            }
        ));
    }

    #[test]
    fn invalid_import_does_not_replace_existing_config() {
        let directory = tempdir().unwrap();
        let store = AppStore::open(directory.path().join("store")).unwrap();
        let mut config = AppConfig::default();
        config.profiles.push(profile("keep"));
        store.save_config(&config).unwrap();
        let bad = directory.path().join("bad.json");
        let mut incompatible = serde_json::to_value(&config).unwrap();
        incompatible["schemaVersion"] = serde_json::json!(99);
        fs::write(&bad, serde_json::to_vec(&incompatible).unwrap()).unwrap();
        assert!(matches!(
            store.import_config(&bad),
            Err(StorageError::UnsupportedSchema { found: 99, .. })
        ));
        fs::write(&bad, b"not json").unwrap();
        assert!(store.import_config(&bad).is_err());
        assert_eq!(store.load_config().unwrap().profiles[0].name, "keep");
        let persisted = fs::read(store.config_path()).unwrap();
        let mut malformed = serde_json::to_value(AppConfig::default()).unwrap();
        malformed["settings"]["ansiPalette"] = serde_json::json!([1]);
        fs::write(&bad, serde_json::to_vec(&malformed).unwrap()).unwrap();
        assert!(matches!(
            store.import_config(&bad),
            Err(StorageError::Corrupt { .. })
        ));
        assert_eq!(fs::read(store.config_path()).unwrap(), persisted);
    }

    #[test]
    fn import_rejects_unknown_tag_references_without_replacing_config() {
        let directory = tempdir().unwrap();
        let store = AppStore::open(directory.path().join("store")).unwrap();
        let mut existing = AppConfig::default();
        existing.profiles.push(profile("keep"));
        store.save_config(&existing).unwrap();
        let persisted = fs::read(store.config_path()).unwrap();

        let mut invalid = AppConfig::default();
        let mut invalid_profile = profile("invalid");
        invalid_profile.tag_ids.push(TagId::new());
        invalid.profiles.push(invalid_profile);
        let import = directory.path().join("invalid.json");
        fs::write(&import, serde_json::to_vec(&invalid).unwrap()).unwrap();

        assert!(matches!(
            store.import_config(&import),
            Err(StorageError::UnknownTagRef { .. })
        ));
        assert_eq!(fs::read(store.config_path()).unwrap(), persisted);
    }

    #[test]
    fn config_round_trip_uses_atomic_replacement() {
        let directory = tempdir().unwrap();
        let store = AppStore::open(directory.path().to_path_buf()).unwrap();
        let mut config = AppConfig::default();
        config.settings.locale = LocaleSetting::ZhCn;
        config.settings.theme.mode = ThemeMode::Dark;
        let mut first = profile("first");
        first.startup_command = Some("tmux attach".into());
        first.environment_variables.push(EnvironmentVariable {
            name: "BUILD_ID".into(),
            value: "full value = preserved".into(),
        });
        config.profiles.push(first);
        store.save_config(&config).unwrap();
        config.profiles.push(profile("second"));
        store.save_config(&config).unwrap();

        assert_eq!(store.load_config().unwrap(), config);
        let names: Vec<_> = fs::read_dir(directory.path())
            .unwrap()
            .map(|entry| entry.unwrap().file_name())
            .collect();
        assert_eq!(names, ["config.json"]);
    }

    #[test]
    fn config_without_profile_additions_loads_with_schema_one() {
        let directory = tempdir().unwrap();
        let profile = profile("legacy");
        let mut config = AppConfig::default();
        config.profiles.push(profile);
        // Simulate a config written before profile additions existed.
        let text = serde_json::to_string(&config)
            .unwrap()
            .replace(",\"tagIds\":[]", "")
            .replace(",\"environmentVariables\":[]", "")
            .replace(",\"tags\":[]", "");
        assert!(!text.contains("tagIds") && !text.contains("environmentVariables"));
        fs::write(directory.path().join(CONFIG_FILE_NAME), &text).unwrap();

        let store = AppStore::open(directory.path().to_path_buf()).unwrap();
        let config = store.load_config().unwrap();
        assert_eq!(config.schema_version(), 1);
        assert!(config.tags.is_empty());
        assert_eq!(config.profiles.len(), 1);
        assert!(config.profiles[0].tag_ids.is_empty());
        assert!(config.profiles[0].environment_variables.is_empty());
    }

    #[test]
    fn duplicate_tag_name_is_rejected_case_insensitively() {
        let directory = tempdir().unwrap();
        let store = AppStore::open(directory.path().to_path_buf()).unwrap();
        let mut config = AppConfig::default();
        config.tags.push(Tag::new("Prod"));
        config.tags.push(Tag::new(" prod "));

        assert!(matches!(
            store.save_config(&config),
            Err(StorageError::DuplicateTagName { .. })
        ));
    }

    #[test]
    fn unknown_tag_ref_is_rejected_on_save() {
        let directory = tempdir().unwrap();
        let store = AppStore::open(directory.path().to_path_buf()).unwrap();
        let mut config = AppConfig::default();
        let mut profile = profile("broken");
        profile.tag_ids.push(TagId::new());
        config.profiles.push(profile);

        assert!(matches!(
            store.save_config(&config),
            Err(StorageError::UnknownTagRef { .. })
        ));
    }

    #[test]
    fn dangling_tag_refs_are_pruned_on_load() {
        let directory = tempdir().unwrap();
        let store = AppStore::open(directory.path().to_path_buf()).unwrap();
        let tag = Tag::new("kept");
        let mut config = AppConfig::default();
        let mut profile = profile("mixed");
        profile.tag_ids = vec![tag.id, TagId::new()];
        config.tags.push(tag.clone());
        config.profiles.push(profile);
        // Bypass save-time validation to simulate an externally edited file.
        fs::write(
            directory.path().join(CONFIG_FILE_NAME),
            serde_json::to_string(&config).unwrap(),
        )
        .unwrap();

        let loaded = store.load_config().unwrap();
        assert_eq!(loaded.profiles[0].tag_ids, vec![tag.id]);
    }

    #[test]
    fn old_config_without_scrollback_keeps_legacy_capacity() {
        let directory = tempdir().unwrap();
        fs::write(
            directory.path().join(CONFIG_FILE_NAME),
            r#"{"schemaVersion":1,"settings":{"locale":"system","theme":"system"},"profiles":[]}"#,
        )
        .unwrap();
        let store = AppStore::open(directory.path().to_path_buf()).unwrap();
        let config = store.load_config().unwrap();
        assert_eq!(config.schema_version(), 1);
        assert_eq!(
            config.settings.terminal_scrollback_lines,
            crate::model::DEFAULT_SCROLLBACK_LINES
        );
    }

    #[test]
    fn unsupported_schema_is_rejected() {
        let directory = tempdir().unwrap();
        fs::write(
            directory.path().join(CONFIG_FILE_NAME),
            r#"{"schemaVersion":2,"settings":{},"profiles":[]}"#,
        )
        .unwrap();
        let store = AppStore::open(directory.path().to_path_buf()).unwrap();

        assert!(matches!(
            store.load_config(),
            Err(StorageError::UnsupportedSchema { found: 2, .. })
        ));
    }

    #[test]
    fn corrupt_config_is_not_overwritten() {
        let directory = tempdir().unwrap();
        let path = directory.path().join(CONFIG_FILE_NAME);
        let corrupt = b"{ definitely not json";
        fs::write(&path, corrupt).unwrap();
        let store = AppStore::open(directory.path().to_path_buf()).unwrap();

        assert!(matches!(
            store.update_config(|config| config.profiles.push(profile("new"))),
            Err(StorageError::Corrupt { .. })
        ));
        assert_eq!(fs::read(path).unwrap(), corrupt);
    }

    #[test]
    fn host_endpoint_is_canonicalized() {
        let directory = tempdir().unwrap();
        let store = AppStore::open(directory.path().to_path_buf()).unwrap();

        store.accept_host_key(host("EXAMPLE.COM", 22)).unwrap();
        store
            .accept_host_key(host("[2001:0DB8:0:0:0:0:0:1]", 2200))
            .unwrap();

        let hosts = store.load_known_hosts().unwrap().hosts;
        assert_eq!(hosts[0].host, "example.com");
        assert_eq!(hosts[1].host, "2001:db8::1");
        assert!(
            store
                .known_host(&Endpoint {
                    host: "Example.Com".into(),
                    port: 22,
                })
                .unwrap()
                .is_some()
        );
    }

    #[test]
    fn replacing_a_host_key_requires_deletion_first() {
        let directory = tempdir().unwrap();
        let store = AppStore::open(directory.path().to_path_buf()).unwrap();
        store.accept_host_key(host("example.com", 22)).unwrap();

        let mut replacement = host("EXAMPLE.COM", 22);
        replacement.fingerprint_sha256 = "SHA256:replacement".into();
        assert!(matches!(
            store.accept_host_key(replacement),
            Err(StorageError::HostKeyConflict)
        ));

        store
            .delete_known_host(&Endpoint {
                host: "example.com".into(),
                port: 22,
            })
            .unwrap();
        let mut fresh = host("example.com", 22);
        fresh.fingerprint_sha256 = "SHA256:replacement".into();
        store.accept_host_key(fresh).unwrap();

        let hosts = store.load_known_hosts().unwrap().hosts;
        assert_eq!(hosts.len(), 1);
        assert_eq!(hosts[0].fingerprint_sha256, "SHA256:replacement");
    }

    #[test]
    fn known_hosts_import_export_preserves_data_and_requires_conflict_policy() {
        let directory = tempdir().unwrap();
        let store = AppStore::open(directory.path().to_path_buf()).unwrap();
        store.accept_host_key(host("existing.example", 22)).unwrap();
        let mut incoming = KnownHosts {
            hosts: vec![host("existing.example", 22), host("new.example", 2200)],
            ..Default::default()
        };
        for entry in &mut incoming.hosts {
            entry.public_key =
                "ssh-ed25519 AAAAC3NzaC1lZDI1NTE5AAAAILagOJFgwaMNhBWQINinKOXmqS4Gh5NgxgriXwdOoINJ"
                    .into();
            entry.fingerprint_sha256 = "SHA256:ldyiXa1JQakitNU5tErauu8DvWQ1dZ7aXu+rm7KQuog".into();
        }
        let import_path = directory.path().join("import.json");
        fs::write(&import_path, serde_json::to_vec(&incoming).unwrap()).unwrap();

        store
            .import_known_hosts(&import_path, KnownHostsConflictPolicy::KeepExisting)
            .unwrap();
        let hosts = store.load_known_hosts().unwrap().hosts;
        assert_eq!(hosts.len(), 2);
        assert_eq!(hosts[0].fingerprint_sha256, "SHA256:test");
        assert_eq!(hosts[1].host, "new.example");

        store
            .import_known_hosts(&import_path, KnownHostsConflictPolicy::ReplaceExisting)
            .unwrap();
        assert_eq!(
            store.load_known_hosts().unwrap().hosts[0].fingerprint_sha256,
            "SHA256:ldyiXa1JQakitNU5tErauu8DvWQ1dZ7aXu+rm7KQuog"
        );
        let export_path = directory.path().join("export.json");
        store.export_known_hosts(&export_path).unwrap();
        let exported: KnownHosts = serde_json::from_slice(&fs::read(export_path).unwrap()).unwrap();
        assert_eq!(exported, store.load_known_hosts().unwrap());
    }

    #[test]
    fn invalid_known_hosts_import_does_not_change_existing_data() {
        let directory = tempdir().unwrap();
        let store = AppStore::open(directory.path().to_path_buf()).unwrap();
        store.accept_host_key(host("existing.example", 22)).unwrap();
        let before = fs::read(store.known_hosts_path()).unwrap();
        let import_path = directory.path().join("invalid.json");
        fs::write(&import_path, "not json").unwrap();

        assert!(matches!(
            store.import_known_hosts(&import_path, KnownHostsConflictPolicy::ReplaceExisting),
            Err(StorageError::CorruptKnownHostsImport(_))
        ));
        assert_eq!(fs::read(store.known_hosts_path()).unwrap(), before);

        let mut invalid_key = KnownHosts::default();
        invalid_key.hosts.push(host("tampered.example", 22));
        fs::write(&import_path, serde_json::to_vec(&invalid_key).unwrap()).unwrap();
        assert!(matches!(
            store.import_known_hosts(&import_path, KnownHostsConflictPolicy::KeepExisting),
            Err(StorageError::InvalidKnownHostImport)
        ));
        assert_eq!(fs::read(store.known_hosts_path()).unwrap(), before);

        fs::write(&import_path, r#"{"schemaVersion":2,"hosts":[]}"#).unwrap();
        assert!(matches!(
            store.import_known_hosts(&import_path, KnownHostsConflictPolicy::ReplaceExisting),
            Err(StorageError::UnsupportedKnownHostsImportSchema { found: 2 })
        ));
        assert_eq!(fs::read(store.known_hosts_path()).unwrap(), before);

        assert!(matches!(
            store.import_known_hosts(
                &directory.path().join("missing.json"),
                KnownHostsConflictPolicy::ReplaceExisting
            ),
            Err(StorageError::KnownHostsImportExportIo { .. })
        ));
        assert_eq!(fs::read(store.known_hosts_path()).unwrap(), before);
    }

    #[test]
    fn reaccepting_the_same_key_is_idempotent() {
        let directory = tempdir().unwrap();
        let store = AppStore::open(directory.path().to_path_buf()).unwrap();

        store.accept_host_key(host("example.com", 22)).unwrap();
        store.accept_host_key(host("EXAMPLE.COM", 22)).unwrap();

        let hosts = store.load_known_hosts().unwrap().hosts;
        assert_eq!(hosts.len(), 1);
    }

    #[test]
    fn config_with_foreign_credential_reference_is_rejected_on_load() {
        let directory = tempdir().unwrap();
        let tampered = r#"{"schemaVersion":1,"settings":{"locale":"system","theme":"system"},"profiles":[{"id":"00000000-0000-0000-0000-000000000000","name":"borrowed","endpoint":{"host":"example.com","port":22},"username":"oxide","auth":{"method":"password","credentialRef":"profile/11111111-1111-1111-1111-111111111111/password"}}]}"#;
        fs::write(directory.path().join(CONFIG_FILE_NAME), tampered).unwrap();
        let store = AppStore::open(directory.path().to_path_buf()).unwrap();

        assert!(matches!(
            store.load_config(),
            Err(StorageError::InvalidCredentialRef { .. })
        ));
    }

    #[test]
    fn config_with_duplicate_profile_ids_is_rejected_on_load() {
        let directory = tempdir().unwrap();
        let tampered = r#"{"schemaVersion":1,"settings":{"locale":"system","theme":"system"},"profiles":[{"id":"00000000-0000-0000-0000-000000000000","name":"first","endpoint":{"host":"example.com","port":22},"username":"oxide","auth":{"method":"agent"}},{"id":"00000000-0000-0000-0000-000000000000","name":"second","endpoint":{"host":"example.org","port":22},"username":"oxide","auth":{"method":"agent"}}]}"#;
        fs::write(directory.path().join(CONFIG_FILE_NAME), tampered).unwrap();
        let store = AppStore::open(directory.path().to_path_buf()).unwrap();

        assert!(matches!(
            store.load_config(),
            Err(StorageError::DuplicateProfile { .. })
        ));
    }

    #[test]
    fn concurrent_config_updates_do_not_lose_profiles() {
        let directory = tempdir().unwrap();
        let store = Arc::new(AppStore::open(directory.path().to_path_buf()).unwrap());
        let writers: Vec<_> = (0..16)
            .map(|index| {
                let store = store.clone();
                thread::spawn(move || {
                    store
                        .update_config(|config| {
                            config.profiles.push(profile(&format!("profile-{index}")))
                        })
                        .unwrap();
                })
            })
            .collect();

        for writer in writers {
            writer.join().unwrap();
        }

        assert_eq!(store.load_config().unwrap().profiles.len(), 16);
    }

    #[test]
    fn storage_never_serializes_secrets() {
        let directory = tempdir().unwrap();
        let store = AppStore::open(directory.path().to_path_buf()).unwrap();
        let input_password = "test-only-password-never-persist";
        let mut config = AppConfig::default();
        config.profiles.push(profile("secret check"));

        store.save_config(&config).unwrap();

        let serialized = fs::read_to_string(directory.path().join(CONFIG_FILE_NAME)).unwrap();
        assert!(!serialized.contains(input_password));
        assert!(serialized.contains("credentialRef"));
        assert!(!serialized.contains("password-never-persist"));
    }

    #[test]
    fn empty_store_uses_schema_one_defaults_without_writing() {
        let directory = tempdir().unwrap();
        let store = AppStore::open(directory.path().to_path_buf()).unwrap();

        assert_eq!(store.load_config().unwrap(), AppConfig::default());
        assert_eq!(store.load_known_hosts().unwrap(), KnownHosts::default());
        assert_eq!(fs::read_dir(directory.path()).unwrap().count(), 0);
    }

    #[test]
    fn private_key_paths_round_trip_without_interpretation() {
        let directory = tempdir().unwrap();
        let store = AppStore::open(directory.path().to_path_buf()).unwrap();
        let mut config = AppConfig::default();
        let mut key_profile = profile("key");
        key_profile.auth = AuthConfig::PrivateKey {
            path: PathBuf::from("keys/id_ed25519"),
            passphrase_ref: None,
        };
        config.profiles.push(key_profile);

        store.save_config(&config).unwrap();

        assert_eq!(store.load_config().unwrap(), config);
    }
}
