use super::MessageId;

pub(super) fn text(id: MessageId) -> &'static str {
    match id {
        MessageId::AppName => "OxideSSH",
        MessageId::SearchConnections => "Search connections",
        MessageId::AddConnection => "Add connection",
        MessageId::NoConnections => "No connections yet",
        MessageId::NoSearchResults => "No matching connections",
        MessageId::Workspace => "Workspace",
        MessageId::AllHosts => "All hosts",
        MessageId::Tags => "Tags",
        MessageId::Untagged => "Untagged",
        MessageId::ManageTags => "Manage tags",
        MessageId::NewTag => "New tag name",
        MessageId::Create => "Create",
        MessageId::Hosts => "Hosts",
        MessageId::Keychain => "Keychain",
        MessageId::PortForwarding => "Port forwarding",
        MessageId::Snippets => "Snippets",
        MessageId::KnownHosts => "Known Hosts",
        MessageId::Logs => "Logs",
        MessageId::NewHost => "New Host",
        MessageId::LocalTerminal => "Local Terminal",
        MessageId::NewTabButton => "New tab button",
        MessageId::Settings => "Settings",
        MessageId::General => "General",
        MessageId::Appearance => "Appearance",
        MessageId::Security => "Security",
        MessageId::Language => "Language",
        MessageId::LanguageDescription => "Application interface language",
        MessageId::ScrollbackLines => "Scrollback lines",
        MessageId::ScrollbackLinesDescription => {
            "Maximum number of lines retained for new terminals (1–1,000,000)"
        }
        MessageId::InvalidScrollbackLines => "Enter a whole number from 1 to 1,000,000.",
        MessageId::Theme => "Theme",
        MessageId::AnsiPalette => "ANSI palette",
        MessageId::AnsiPaletteDescription => {
            "Enter 16 comma-separated RGB hex colors in index order (0–7 standard, 8–15 bright)."
        }
        MessageId::Reset => "Reset",
        MessageId::CopyOnSelect => "Copy on select",
        MessageId::CopyOnSelectDescription => "Copy selected terminal text to the clipboard",
        MessageId::ThemeDescription => "Application color scheme",
        MessageId::TrustedHosts => "Trusted hosts",
        MessageId::ConfirmBeforeCloseWithSessions => "Confirm before closing sessions",
        MessageId::ConfirmBeforeCloseWithSessionsDescription => {
            "Ask before closing tabs, windows, or the app while sessions are active"
        }
        MessageId::NewTabButtonDescription => "Action performed by the new tab button",
        MessageId::RememberWindowSize => "Remember window size",
        MessageId::RememberWindowSizeDescription => {
            "Restore the last window size on the next launch"
        }
        MessageId::SshCompression => "SSH compression",
        MessageId::SshCompressionDescription => {
            "Compress SSH traffic when available; peers without compression fall back to uncompressed traffic."
        }
        MessageId::SshConnectTimeout => "SSH connection timeout",
        MessageId::SshConnectTimeoutDescription => "Time allowed for a new SSH connection",
        MessageId::Timeout15Seconds => "15 seconds",
        MessageId::Timeout30Seconds => "30 seconds",
        MessageId::Timeout60Seconds => "60 seconds",
        MessageId::Timeout120Seconds => "120 seconds",
        MessageId::SshKeepaliveInterval => "SSH keepalive interval (seconds)",
        MessageId::SshKeepaliveMaxCount => "SSH keepalive maximum missed replies",
        MessageId::ProfileKeepaliveInterval => {
            "Profile keepalive interval override (seconds; blank inherits)"
        }
        MessageId::ProfileKeepaliveMaxCount => {
            "Profile keepalive maximum missed replies (blank inherits)"
        }
        MessageId::InvalidKeepalive => "Keepalive values must be positive whole numbers.",
        MessageId::LocalWorkingDirectory => "Local terminal working directory",
        MessageId::LocalWorkingDirectoryDescription => "Leave empty to use your home directory.",
        MessageId::Osc52Clipboard => "Remote clipboard",
        MessageId::Osc52ClipboardDescription => {
            "Allow remote applications to write to the system clipboard"
        }
        MessageId::Osc52ClipboardOff => "Off",
        MessageId::Osc52ClipboardAsk => "Ask",
        MessageId::Osc52ClipboardOn => "On",
        MessageId::Osc52ClipboardConfirm => {
            "Allow this remote application to replace the system clipboard?"
        }
        MessageId::Allow => "Allow",
        MessageId::SshTerm => "SSH TERM",
        MessageId::SshTermDescription => "Terminal type requested for remote PTYs",
        MessageId::System => "System",
        MessageId::Light => "Light",
        MessageId::English => "English",
        MessageId::SimplifiedChinese => "Simplified Chinese",
        MessageId::Dark => "Dark",
        MessageId::Delete => "Delete",
        MessageId::Retry => "Retry",
        MessageId::OpenConfigDirectory => "Open configuration directory",
        MessageId::Quit => "Quit",
        MessageId::NewWindow => "New Window",
        MessageId::StorageCorrupt => "A configuration file is corrupt and was not overwritten.",
        MessageId::ExportKnownHosts => "Export trusted hosts",
        MessageId::ImportKnownHosts => "Import trusted hosts",
        MessageId::ImportKeepExisting => "Import (keep existing keys)",
        MessageId::ImportReplaceExisting => "Import (replace conflicting keys)",
        MessageId::KnownHostsTransferFailed => "Could not import or export trusted hosts.",
        MessageId::KnownHostsExported => "Trusted hosts exported.",
        MessageId::KnownHostsImported => "Trusted hosts imported.",
        MessageId::NoTrustedHosts => "No trusted hosts",
        MessageId::Close => "Close",
        MessageId::Copy => "Copy",
        MessageId::Name => "Name",
        MessageId::Host => "Host",
        MessageId::Port => "Port",
        MessageId::Username => "Username",
        MessageId::AuthMethod => "Authentication method",
        MessageId::Password => "Password",
        MessageId::EnvironmentVariables => {
            "Remote environment requests (JSON array; server may reject)"
        }
        MessageId::PrivateKey => "Private key",
        MessageId::StartupCommand => "Startup command",
        MessageId::Agent => "SSH agent",
        MessageId::Remember => "Remember in system credential store",
        MessageId::Passphrase => "Passphrase",
        MessageId::Browse => "Browse",
        MessageId::Cancel => "Cancel",
        MessageId::Save => "Save",
        MessageId::AgentDescription => "Use identities from the system SSH agent.",
        MessageId::CredentialRequired => "Enter the credential for this connection.",
        MessageId::Connect => "Connect",
        MessageId::ConnectOnce => "Connect once",
        MessageId::Edit => "Edit",
        MessageId::ClearAllCredentials => "Clear all saved credentials",
        MessageId::ConfirmClearAllCredentials => {
            "This permanently deletes every OxideSSH credential from the system credential store, including credentials not referenced by a profile. This cannot be undone."
        }
        MessageId::ClearCredentialsSuccess => "All OxideSSH credentials were deleted.",
        MessageId::ClearCredentialsFailures => "Some OxideSSH credentials could not be deleted:",
        MessageId::ConfirmDeleteProfile => "Delete this connection?",
        MessageId::ConfirmDeleteHost => "Delete this trusted host?",
        MessageId::ConfirmDeleteTag => "Delete this tag?",
        MessageId::ConfirmCloseTab => "Disconnect and close this tab?",
        MessageId::ConfirmQuit => "Disconnect all sessions and quit?",
        MessageId::Connecting => "Connecting",
        MessageId::VerifyingHostKey => "Verifying host key",
        MessageId::Authenticating => "Authenticating",
        MessageId::OpeningShell => "Opening shell",
        MessageId::Connected => "Connected",
        MessageId::Disconnected => "Disconnected",
        MessageId::AwaitingHostKey => "Waiting for host key decision",
        MessageId::AwaitingSecret => "Waiting for a credential",
        MessageId::UnknownHostKey => "Unknown host key",
        MessageId::ChangedHostKey => "The host key has changed",
        MessageId::ExpectedFingerprint => "Expected fingerprint",
        MessageId::PresentedFingerprint => "Presented fingerprint",
        MessageId::Reject => "Reject",
        MessageId::AcceptAndStore => "Accept and store",
        MessageId::OpenTrustedHosts => "Open trusted hosts",
        MessageId::Bell => "Bell",
        MessageId::TerminalBell => "Terminal bell",
        MessageId::BellVisual => "Visual",
        MessageId::BellSound => "Sound",
        MessageId::BellOff => "Off",
        MessageId::BellDescription => "Choose how remote terminal bell events are reported.",
        MessageId::InputQueueFull => "Input is paused because the session queue is full.",
        MessageId::PasteTooLarge => "Paste exceeds the 1 MiB limit.",
        MessageId::ConfirmLargePaste => "This paste is large. Send all of it to the terminal?",
        MessageId::Paste => "Send",
        MessageId::PasteConfirmThreshold => "Large paste confirmation",
        MessageId::PasteConfirmThresholdDescription => {
            "Ask before sending pasted text longer than this size."
        }
        MessageId::PasteThreshold1KiB => "1 KiB",
        MessageId::PasteThreshold4KiB => "4 KiB",
        MessageId::PasteThreshold16KiB => "16 KiB",
        MessageId::CredentialUnavailable => "The system credential store is unavailable.",
        MessageId::CredentialAccessDenied => "Access to the system credential store was denied.",
        MessageId::CredentialInvalidReference => "The credential reference is invalid.",
        MessageId::CredentialCorrupt => "The saved credential is unreadable.",
        MessageId::CredentialAmbiguous => "More than one saved credential matches this connection.",
        MessageId::CredentialUnsupported => "This credential operation is not supported.",
        MessageId::InvalidProfile => "The connection profile is invalid.",
        MessageId::TagExists => "A tag with this name already exists.",
        MessageId::InvalidTagName => "Tag names must be 1-32 characters.",
        MessageId::ConnectTimeout => "The connection timed out.",
        MessageId::ConnectFailed => "The connection failed.",
        MessageId::HostKeyRejected => "The host key was rejected.",
        MessageId::HostKeyChanged => "The saved and presented host keys do not match.",
        MessageId::HostKeyStoreFailed => "The accepted host key could not be saved.",
        MessageId::PrivateKeyUnreadable => "The private key file could not be read.",
        MessageId::PrivateKeyPassphraseRejected => "The private key passphrase is incorrect.",
        MessageId::AgentUnavailable => "The SSH agent is unavailable.",
        MessageId::AgentEmpty => "The SSH agent has no identities.",
        MessageId::AuthenticationRejected => "The server rejected authentication.",
        MessageId::PtyRejected => "The server rejected the terminal request.",
        MessageId::SessionClosed => "The session input channel is closed.",
        MessageId::ShellRejected => "The server rejected the shell request.",
        MessageId::LocalShellFailed => "Failed to start the local shell.",
        MessageId::TerminalFontFamily => "Terminal font",
        MessageId::TerminalFontSize => "Terminal font size",
        MessageId::TerminalLineHeight => "Terminal line height",
        MessageId::LocalWorkingDirectoryInvalid => {
            "The configured local working directory does not exist or cannot be accessed."
        }
    }
}
