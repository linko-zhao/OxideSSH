use super::*;

pub(super) fn system_locale() -> Option<String> {
    sys_locale::get_locale()
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum StatusSeverity {
    Success,
    Warning,
    Error,
}

pub(super) fn status_severity(message: MessageId) -> StatusSeverity {
    match message {
        MessageId::ClearCredentialsSuccess
        | MessageId::ConfigExportSuccess
        | MessageId::ConfigImportSuccess
        | MessageId::DiscoveryImportSuccess
        | MessageId::KnownHostsExported
        | MessageId::KnownHostsImported => StatusSeverity::Success,
        MessageId::ClearCredentialsFailures | MessageId::ConfigImportReloadFailure => {
            StatusSeverity::Warning
        }
        _ => StatusSeverity::Error,
    }
}

pub(super) fn is_dark(appearance: WindowAppearance) -> bool {
    matches!(
        appearance,
        WindowAppearance::Dark | WindowAppearance::VibrantDark
    )
}

pub(super) fn credential_reference(
    auth: &AuthConfig,
) -> Option<&oxide_ssh_core::model::CredentialRef> {
    match auth {
        AuthConfig::Password { credential_ref } => credential_ref.as_ref(),
        AuthConfig::PrivateKey { passphrase_ref, .. } => passphrase_ref.as_ref(),
        AuthConfig::Agent => None,
    }
}

pub(super) fn requires_one_time_secret(profile: &ConnectionProfile) -> bool {
    matches!(profile.auth, AuthConfig::Password { .. })
}

pub(super) fn session_error_message_id(error: SessionError) -> MessageId {
    match error {
        SessionError::InvalidProfile => MessageId::InvalidProfile,
        SessionError::ConnectTimeout => MessageId::ConnectTimeout,
        SessionError::ConnectFailed | SessionError::Disconnected => MessageId::ConnectFailed,
        SessionError::HostKeyRejected => MessageId::HostKeyRejected,
        SessionError::HostKeyChanged => MessageId::HostKeyChanged,
        SessionError::HostKeyStoreFailed => MessageId::HostKeyStoreFailed,
        SessionError::CredentialUnavailable => MessageId::CredentialUnavailable,
        SessionError::PrivateKeyUnreadable => MessageId::PrivateKeyUnreadable,
        SessionError::PrivateKeyPassphraseRequired => MessageId::CredentialRequired,
        SessionError::PrivateKeyPassphraseRejected => MessageId::PrivateKeyPassphraseRejected,
        SessionError::AgentUnavailable => MessageId::AgentUnavailable,
        SessionError::AgentEmpty => MessageId::AgentEmpty,
        SessionError::AuthenticationRejected => MessageId::AuthenticationRejected,
        SessionError::PtyRejected => MessageId::PtyRejected,
        SessionError::ShellRejected => MessageId::ShellRejected,
        SessionError::LocalShellFailed => MessageId::LocalShellFailed,
        SessionError::LocalWorkingDirectoryInvalid => MessageId::LocalWorkingDirectoryInvalid,
    }
}

pub(super) fn transaction_error_message_id(error: &CredentialTransactionError) -> MessageId {
    match error {
        CredentialTransactionError::Credential(error) => credential_error_message_id(*error),
        CredentialTransactionError::Storage(_) => MessageId::StorageCorrupt,
        CredentialTransactionError::InvalidProfile(_)
        | CredentialTransactionError::MissingSecret
        | CredentialTransactionError::ProfileMismatch
        | CredentialTransactionError::ProfileNotFound
        | CredentialTransactionError::RollbackFailed => MessageId::InvalidProfile,
    }
}

pub(super) fn tag_storage_error_message_id(error: &StorageError) -> MessageId {
    match error {
        StorageError::DuplicateTagName { .. } => MessageId::TagExists,
        StorageError::InvalidTag(_) => MessageId::InvalidTagName,
        _ => MessageId::StorageCorrupt,
    }
}

pub(super) fn form_error_message_id(error: FormError) -> MessageId {
    match error {
        FormError::Name => MessageId::InvalidName,
        FormError::Host => MessageId::InvalidHost,
        FormError::Username => MessageId::InvalidUsername,
        FormError::Port => MessageId::InvalidPort,
        FormError::PrivateKeyPath => MessageId::InvalidPrivateKeyPath,
        FormError::Keepalive | FormError::KeepaliveMaxCount => MessageId::InvalidKeepalive,
        FormError::TerminalFontSize => MessageId::InvalidTerminalFontSize,
        FormError::AnsiPalette => MessageId::InvalidAnsiPalette,
        FormError::EnvironmentVariable => MessageId::InvalidEnvironmentVariables,
    }
}

pub(super) fn local_error_message_id(error: TabLocalError) -> MessageId {
    match error {
        TabLocalError::InputQueueFull => MessageId::InputQueueFull,
        TabLocalError::InputClosed => MessageId::SessionClosed,
        TabLocalError::PasteTooLarge => MessageId::PasteTooLarge,
        TabLocalError::InvalidTerminalSize => MessageId::InvalidProfile,
    }
}

pub(super) fn tab_state_message_id(state: &TabState) -> MessageId {
    match state {
        TabState::Connecting => MessageId::Connecting,
        TabState::AwaitingHostKey => MessageId::AwaitingHostKey,
        TabState::AwaitingSecret => MessageId::AwaitingSecret,
        TabState::Connected => MessageId::Connected,
        TabState::Disconnected { .. } => MessageId::Disconnected,
    }
}

pub(super) fn disconnect_reason_message_id(reason: DisconnectReason) -> MessageId {
    match reason {
        DisconnectReason::Session(error) => session_error_message_id(error),
        DisconnectReason::Exit(_) | DisconnectReason::Closed => MessageId::Disconnected,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn transfer_statuses_have_matching_severity() {
        for message in [
            MessageId::ConfigImportSuccess,
            MessageId::ConfigExportSuccess,
            MessageId::KnownHostsImported,
            MessageId::KnownHostsExported,
            MessageId::DiscoveryImportSuccess,
            MessageId::ClearCredentialsSuccess,
        ] {
            assert_eq!(status_severity(message), StatusSeverity::Success);
        }
        for message in [
            MessageId::ConfigImportReloadFailure,
            MessageId::ClearCredentialsFailures,
        ] {
            assert_eq!(status_severity(message), StatusSeverity::Warning);
        }
        for message in [
            MessageId::ConfigImportFailure,
            MessageId::ConfigExportFailure,
            MessageId::KnownHostsTransferFailed,
            MessageId::DiscoveryImportFailed,
            MessageId::StorageCorrupt,
        ] {
            assert_eq!(status_severity(message), StatusSeverity::Error);
        }
    }
}
