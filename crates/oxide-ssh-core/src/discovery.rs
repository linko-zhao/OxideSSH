//! Discovery of hosts from OpenSSH client configuration and installed WSL
//! distributions.
//!
//! Discovery never reads private key contents; it only checks whether an
//! identity file exists.

use std::{
    collections::HashSet,
    fs, io,
    path::{Path, PathBuf},
};

#[cfg(windows)]
use std::os::windows::process::CommandExt;

/// A host entry ready to become a connection profile.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DiscoveredSshHost {
    pub alias: String,
    pub hostname: String,
    pub username: Option<String>,
    pub port: u16,
    pub identity_file: Option<PathBuf>,
    /// True when [`Self::identity_file`] is set but does not exist.
    pub identity_file_missing: bool,
}

/// A host entry that was intentionally not discovered, with a reason for the
/// UI to display.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SkippedSshHost {
    pub alias: String,
    pub reason: SshConfigSkipReason,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum SshConfigSkipReason {
    /// `Host` patterns with `*`, `?` or `!` cannot be turned into a profile.
    WildcardHost,
    /// A conditional `Match` block is not supported.
    Match,
    /// The same alias already appeared earlier in this scan.
    DuplicateAlias,
    /// OpenSSH tries every identity file; OxideSSH supports exactly one.
    MultipleIdentityFiles,
    /// A directive that changes how the connection is made is not supported,
    /// so the host must not be silently connected directly.
    UnsupportedDirective { directive: String, value: String },
    /// A supported directive carried a value that cannot be used.
    InvalidValue { directive: String, value: String },
    /// An included file matched by `Include` could not be read.
    IncludeFailed(String),
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct SshConfigDiscovery {
    pub hosts: Vec<DiscoveredSshHost>,
    pub skipped: Vec<SkippedSshHost>,
}

/// `~/.ssh/config` under the current user's home directory.
pub fn default_ssh_config_path() -> Option<PathBuf> {
    home_dir().map(|home| home.join(".ssh").join("config"))
}

/// Parses `path` and every file it `Include`s.
///
/// A failure to read the root file is returned as an error. Problems inside
/// included files are reported through [`SshConfigDiscovery::skipped`].
pub fn discover_ssh_config(path: &Path) -> io::Result<SshConfigDiscovery> {
    let mut parser = Parser::default();
    parser.parse_file(path)?;
    parser.finish();
    Ok(SshConfigDiscovery {
        hosts: parser.hosts,
        skipped: parser.skipped,
    })
}

/// Lists installed WSL distributions via `wsl.exe --list --quiet`.
///
/// This only enumerates names; it does not start a distribution.
#[cfg(windows)]
pub fn discover_wsl_distributions() -> io::Result<Vec<String>> {
    let mut command = std::process::Command::new("wsl.exe");
    command.args(["--list", "--quiet"]);
    // A GUI application would otherwise flash a console window for wsl.exe.
    command.creation_flags(0x0800_0000); // CREATE_NO_WINDOW
    let output = command.output()?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(io::Error::other(format!(
            "wsl.exe --list failed: {}",
            stderr.trim()
        )));
    }
    parse_wsl_distribution_list(&output.stdout)
}

/// Lists installed WSL distributions via `wsl.exe --list --quiet`.
///
/// WSL only exists on Windows.
#[cfg(not(windows))]
pub fn discover_wsl_distributions() -> io::Result<Vec<String>> {
    Err(io::Error::new(
        io::ErrorKind::Unsupported,
        "WSL is only available on Windows",
    ))
}

/// Parses `wsl.exe --list --quiet` output, which is usually UTF-16LE but may
/// be UTF-8. Names are trimmed and deduplicated case-insensitively, keeping
/// the first spelling.
pub fn parse_wsl_distribution_list(bytes: &[u8]) -> io::Result<Vec<String>> {
    let text = decode_wsl_output(bytes)?;
    let mut seen = HashSet::new();
    let mut names = Vec::new();
    for name in text.lines().map(str::trim).filter(|name| !name.is_empty()) {
        if seen.insert(name.to_lowercase()) {
            names.push(name.to_owned());
        }
    }
    Ok(names)
}

fn decode_wsl_output(bytes: &[u8]) -> io::Result<String> {
    if let Some(rest) = bytes.strip_prefix(&[0xFF, 0xFE]) {
        return decode_utf16_le(rest);
    }
    if let Some(rest) = bytes.strip_prefix(&[0xEF, 0xBB, 0xBF]) {
        return std::str::from_utf8(rest)
            .map(str::to_owned)
            .map_err(|error| io::Error::new(io::ErrorKind::InvalidData, error));
    }
    // UTF-8 text never contains NUL bytes; UTF-16LE text always does.
    if bytes.contains(&0) {
        return decode_utf16_le(bytes);
    }
    std::str::from_utf8(bytes)
        .map(str::to_owned)
        .map_err(|error| io::Error::new(io::ErrorKind::InvalidData, error))
}

fn decode_utf16_le(bytes: &[u8]) -> io::Result<String> {
    if bytes.len() % 2 != 0 {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "odd-length UTF-16 output",
        ));
    }
    let units: Vec<u16> = bytes
        .chunks_exact(2)
        .map(|pair| u16::from_le_bytes([pair[0], pair[1]]))
        .collect();
    String::from_utf16(&units).map_err(|error| io::Error::new(io::ErrorKind::InvalidData, error))
}

fn home_dir() -> Option<PathBuf> {
    #[cfg(windows)]
    let home = std::env::var_os("USERPROFILE").or_else(|| std::env::var_os("HOME"));
    #[cfg(not(windows))]
    let home = std::env::var_os("HOME");
    home.filter(|home| !home.is_empty()).map(PathBuf::from)
}

fn expand_tilde(path: &str) -> PathBuf {
    if let Some(home) = home_dir() {
        if path == "~" {
            return home;
        }
        if let Some(rest) = path.strip_prefix("~/").or_else(|| path.strip_prefix("~\\")) {
            return home.join(rest);
        }
    }
    PathBuf::from(path)
}

#[derive(Default)]
struct Parser {
    hosts: Vec<DiscoveredSshHost>,
    skipped: Vec<SkippedSshHost>,
    aliases: HashSet<String>,
    visited: HashSet<PathBuf>,
    block: Option<Block>,
    in_match: bool,
}

#[derive(Default)]
struct Block {
    aliases: Vec<String>,
    hostname: Option<String>,
    username: Option<String>,
    port: Option<u16>,
    identity_files: Vec<String>,
    skip: Option<SshConfigSkipReason>,
}

impl Block {
    fn skip(&mut self, reason: SshConfigSkipReason) {
        if self.skip.is_none() {
            self.skip = Some(reason);
        }
    }
}

impl Parser {
    fn parse_file(&mut self, path: &Path) -> io::Result<()> {
        let key = fs::canonicalize(path).unwrap_or_else(|_| path.to_path_buf());
        if !self.visited.insert(key) {
            return Ok(());
        }
        let text = fs::read_to_string(path)?;
        let base = path.parent().unwrap_or_else(|| Path::new(""));
        for line in text.lines() {
            let tokens = tokenize(line);
            if !tokens.is_empty() {
                self.line(&tokens, base);
            }
        }
        Ok(())
    }

    fn line(&mut self, tokens: &[String], base: &Path) {
        let keyword = tokens[0].to_ascii_lowercase();
        let args = &tokens[1..];
        match keyword.as_str() {
            "host" => {
                self.finish_block();
                self.in_match = false;
                if !args.is_empty() {
                    self.block = Some(Block {
                        aliases: args.to_vec(),
                        ..Block::default()
                    });
                }
            }
            "match" => {
                self.finish_block();
                if !self.in_match {
                    self.in_match = true;
                    self.skipped.push(SkippedSshHost {
                        alias: "Match".to_owned(),
                        reason: SshConfigSkipReason::Match,
                    });
                }
            }
            "include" => {
                if !self.in_match {
                    for pattern in args {
                        self.include(pattern, base);
                    }
                }
            }
            _ if self.in_match => {}
            _ => self.directive(&keyword, args),
        }
    }

    fn directive(&mut self, keyword: &str, args: &[String]) {
        let Some(block) = self.block.as_mut() else {
            return;
        };
        let Some(value) = args.first() else {
            return;
        };
        match keyword {
            "hostname" => {
                block.hostname.get_or_insert_with(|| value.clone());
            }
            "user" => {
                block.username.get_or_insert_with(|| value.clone());
            }
            "port" => match value.parse::<u16>() {
                Ok(port) if port != 0 => {
                    block.port.get_or_insert(port);
                }
                _ => block.skip(SshConfigSkipReason::InvalidValue {
                    directive: "Port".to_owned(),
                    value: value.clone(),
                }),
            },
            "identityfile" => block.identity_files.extend(args.iter().cloned()),
            "proxyjump" | "proxycommand" => {
                if !args.iter().all(|arg| arg.eq_ignore_ascii_case("none")) {
                    block.skip(SshConfigSkipReason::UnsupportedDirective {
                        directive: keyword.to_owned(),
                        value: args.join(" "),
                    });
                }
            }
            _ => {}
        }
    }

    fn include(&mut self, pattern: &str, base: &Path) {
        let expanded = expand_tilde(pattern);
        let path = if expanded.is_absolute() {
            expanded
        } else {
            base.join(expanded)
        };
        // shortcut: a directory name containing `*`, `?` or `[` is read as a
        // glob; escape it if that ever matters for real configs.
        let glob_pattern = path.to_string_lossy().into_owned();
        if glob_pattern.contains(['*', '?', '[']) {
            let paths = match glob::glob(&glob_pattern) {
                Ok(paths) => paths,
                Err(error) => {
                    self.include_failed(pattern, error.to_string());
                    return;
                }
            };
            let mut paths: Vec<PathBuf> = paths.filter_map(Result::ok).collect();
            paths.sort();
            for included in paths {
                if let Err(error) = self.parse_file(&included) {
                    self.include_failed(pattern, error.to_string());
                }
            }
        } else if path.is_file() {
            if let Err(error) = self.parse_file(&path) {
                self.include_failed(pattern, error.to_string());
            }
        }
    }

    fn include_failed(&mut self, pattern: &str, message: String) {
        self.skipped.push(SkippedSshHost {
            alias: pattern.to_owned(),
            reason: SshConfigSkipReason::IncludeFailed(message),
        });
    }

    fn finish(&mut self) {
        self.finish_block();
    }

    fn finish_block(&mut self) {
        let Some(block) = self.block.take() else {
            return;
        };
        if block.aliases.is_empty() {
            return;
        }
        if let Some(reason) = block.skip {
            for alias in block.aliases {
                self.skipped.push(SkippedSshHost {
                    alias,
                    reason: reason.clone(),
                });
            }
            return;
        }
        if block.identity_files.len() > 1 {
            for alias in block.aliases {
                self.skipped.push(SkippedSshHost {
                    alias,
                    reason: SshConfigSkipReason::MultipleIdentityFiles,
                });
            }
            return;
        }
        let identity_file = block.identity_files.first().map(|path| expand_tilde(path));
        let identity_file_missing = identity_file.as_ref().is_some_and(|path| !path.is_file());
        for alias in block.aliases {
            if alias.contains(['*', '?', '!']) {
                self.skipped.push(SkippedSshHost {
                    alias,
                    reason: SshConfigSkipReason::WildcardHost,
                });
                continue;
            }
            if !self.aliases.insert(alias.to_lowercase()) {
                self.skipped.push(SkippedSshHost {
                    alias,
                    reason: SshConfigSkipReason::DuplicateAlias,
                });
                continue;
            }
            self.hosts.push(DiscoveredSshHost {
                hostname: block.hostname.clone().unwrap_or_else(|| alias.clone()),
                alias,
                username: block.username.clone(),
                port: block.port.unwrap_or(22),
                identity_file: identity_file.clone(),
                identity_file_missing,
            });
        }
    }
}

/// Splits one config line into whitespace-separated values. Double quotes
/// group spaces; `#` starts a comment when it begins a token.
fn tokenize(line: &str) -> Vec<String> {
    let mut tokens = Vec::new();
    let mut current = String::new();
    let mut quoted = false;
    let mut chars = line.chars().peekable();
    while let Some(c) = chars.next() {
        if c == '"' {
            quoted = !quoted;
        } else if quoted {
            if c == '\\' && matches!(chars.peek(), Some('"') | Some('\\')) {
                current.push(chars.next().expect("peeked character"));
            } else {
                current.push(c);
            }
        } else if c.is_whitespace() {
            if !current.is_empty() {
                tokens.push(std::mem::take(&mut current));
            }
        } else if c == '#' && current.is_empty() {
            break;
        } else {
            current.push(c);
        }
    }
    if !current.is_empty() {
        tokens.push(current);
    }
    tokens
}

#[cfg(test)]
mod tests {
    use super::*;

    fn write(dir: &Path, name: &str, contents: &str) -> PathBuf {
        let path = dir.join(name);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).unwrap();
        }
        fs::write(&path, contents).unwrap();
        path
    }

    #[test]
    fn discovers_hosts_and_follows_includes() {
        let dir = tempfile::tempdir().unwrap();
        let key = write(dir.path(), "id_delta", "");
        let root = write(
            dir.path(),
            "root.conf",
            &format!(
                "# comment\n\
                 Host alpha\n\
                 \tHostName alpha.example.com\n\
                 \tUser alice\n\
                 \tPort 2222\n\
                 \tIdentityFile ~/.ssh/oxide-ssh-missing-key\n\
                 \n\
                 Host beta gamma\n\
                 \tUser bob\n\
                 \n\
                 Include conf.d/*.conf\n\
                 Host quoted\n\
                 \tHostName \"quoted.example.com\"\n",
            ),
        );
        write(
            dir.path(),
            "conf.d/extra.conf",
            &format!(
                "Host delta\n  HostName delta.example.com\n  IdentityFile {}\n",
                key.display()
            ),
        );

        let discovery = discover_ssh_config(&root).unwrap();
        assert!(discovery.skipped.is_empty(), "{:?}", discovery.skipped);
        let aliases: Vec<&str> = discovery.hosts.iter().map(|h| h.alias.as_str()).collect();
        assert_eq!(aliases, ["alpha", "beta", "gamma", "delta", "quoted"]);

        let alpha = &discovery.hosts[0];
        assert_eq!(alpha.hostname, "alpha.example.com");
        assert_eq!(alpha.username.as_deref(), Some("alice"));
        assert_eq!(alpha.port, 2222);
        assert_eq!(
            alpha.identity_file.as_deref(),
            Some(
                home_dir()
                    .unwrap()
                    .join(".ssh/oxide-ssh-missing-key")
                    .as_path()
            )
        );
        assert!(alpha.identity_file_missing);

        let beta = &discovery.hosts[1];
        assert_eq!(beta.hostname, "beta");
        assert_eq!(beta.username.as_deref(), Some("bob"));
        assert_eq!(beta.port, 22);
        assert_eq!(beta.identity_file, None);
        assert!(!beta.identity_file_missing);
        assert_eq!(discovery.hosts[2].hostname, "gamma");

        let delta = &discovery.hosts[3];
        assert_eq!(delta.hostname, "delta.example.com");
        assert_eq!(delta.identity_file.as_deref(), Some(key.as_path()));
        assert!(!delta.identity_file_missing);
        assert_eq!(discovery.hosts[4].hostname, "quoted.example.com");
    }

    #[test]
    fn skips_blocks_that_would_change_connection_semantics() {
        let dir = tempfile::tempdir().unwrap();
        let root = write(
            dir.path(),
            "config",
            "Host *.example.com\n\
             \tUser wild\n\
             Host proxied\n\
             \tProxyJump bastion\n\
             Host multi\n\
             \tIdentityFile /a\n\
             \tIdentityFile /b\n\
             Host direct\n\
             \tProxyCommand none\n\
             \tHostName direct.example.com\n\
             Host matchy\n\
             \tUser before\n\
             Match host foo\n\
             \tUser after\n\
             Host after\n\
             \tUser later\n\
             Host direct\n\
             \tPort 2200\n\
             Host broken\n\
             \tPort nope\n",
        );

        let discovery = discover_ssh_config(&root).unwrap();
        let discovered: Vec<&str> = discovery.hosts.iter().map(|h| h.alias.as_str()).collect();
        assert_eq!(discovered, ["direct", "matchy", "after"]);

        let direct = &discovery.hosts[0];
        assert_eq!(direct.hostname, "direct.example.com");
        assert_eq!(direct.port, 22);

        let skipped: Vec<(&str, &SshConfigSkipReason)> = discovery
            .skipped
            .iter()
            .map(|s| (s.alias.as_str(), &s.reason))
            .collect();
        assert_eq!(
            skipped,
            [
                ("*.example.com", &SshConfigSkipReason::WildcardHost),
                (
                    "proxied",
                    &SshConfigSkipReason::UnsupportedDirective {
                        directive: "proxyjump".to_owned(),
                        value: "bastion".to_owned(),
                    },
                ),
                ("multi", &SshConfigSkipReason::MultipleIdentityFiles),
                ("Match", &SshConfigSkipReason::Match),
                ("direct", &SshConfigSkipReason::DuplicateAlias),
                (
                    "broken",
                    &SshConfigSkipReason::InvalidValue {
                        directive: "Port".to_owned(),
                        value: "nope".to_owned(),
                    },
                ),
            ]
        );
    }

    #[test]
    fn include_cycles_and_missing_globs_do_not_loop() {
        let dir = tempfile::tempdir().unwrap();
        let root = write(
            dir.path(),
            "root.conf",
            "Include a.conf\nInclude missing/*.conf\nHost root\n",
        );
        write(dir.path(), "a.conf", "Include root.conf\nHost a\n");

        let discovery = discover_ssh_config(&root).unwrap();
        let aliases: Vec<&str> = discovery.hosts.iter().map(|h| h.alias.as_str()).collect();
        assert_eq!(aliases, ["a", "root"]);
        assert!(discovery.skipped.is_empty(), "{:?}", discovery.skipped);
    }

    #[test]
    fn missing_root_config_is_an_error() {
        let dir = tempfile::tempdir().unwrap();
        assert!(discover_ssh_config(&dir.path().join("absent")).is_err());
    }

    #[test]
    fn parses_wsl_output_and_deduplicates_distributions() {
        let mut utf16 = vec![0xFF, 0xFE];
        for unit in "Ubuntu\r\nDebian\r\nubuntu\r\n\r\n".encode_utf16() {
            utf16.extend_from_slice(&unit.to_le_bytes());
        }
        assert_eq!(
            parse_wsl_distribution_list(&utf16).unwrap(),
            ["Ubuntu", "Debian"]
        );
        assert_eq!(
            parse_wsl_distribution_list(b"Debian\nUbuntu\n").unwrap(),
            ["Debian", "Ubuntu"]
        );
        assert!(parse_wsl_distribution_list(&[0x41, 0x00, 0x42]).is_err());
    }
}
