//! Local shell sessions: a platform shell on a PTY, no SSH involved.

use std::{
    io::{Read, Write},
    sync::{
        atomic::{AtomicBool, Ordering},
        mpsc,
    },
    thread,
};

use async_channel::{Receiver, Sender, TrySendError};
use bytes::Bytes;
use parking_lot::Mutex;
use portable_pty::{ChildKiller, CommandBuilder, ExitStatus, PtySize, native_pty_system};

use crate::{
    model::{SessionId, TerminalSize},
    session::{
        EVENT_QUEUE_CAPACITY, INPUT_QUEUE_CAPACITY, SendInputError, SessionError, SessionEvent,
        SessionState,
    },
};

/// Spawns the platform default shell on a local PTY.
///
/// Shell selection: `$SHELL` (Unix, falling back to `/bin/sh`); on Windows
/// `pwsh.exe` → `powershell.exe` → `%COMSPEC%` → `cmd.exe`.
/// An empty working directory preserves the platform home directory default.
pub fn spawn_local_shell(
    initial_size: TerminalSize,
    working_directory: &str,
) -> Result<LocalSessionHandle, SessionError> {
    if !initial_size.is_valid() {
        return Err(SessionError::InvalidProfile);
    }
    let pty = native_pty_system();
    let pair = pty
        .openpty(pty_size(initial_size))
        .map_err(|_| SessionError::LocalShellFailed)?;
    let mut command = CommandBuilder::new(default_shell());
    if working_directory.is_empty() {
        if let Some(home) = home_dir() {
            command.cwd(home);
        }
    } else {
        let directory = std::path::Path::new(working_directory);
        if !directory.is_dir() || std::fs::read_dir(directory).is_err() {
            return Err(SessionError::LocalWorkingDirectoryInvalid);
        }
        command.cwd(working_directory);
    }
    let mut child = pair.slave.spawn_command(command).map_err(|_| {
        if working_directory.is_empty() {
            SessionError::LocalShellFailed
        } else {
            SessionError::LocalWorkingDirectoryInvalid
        }
    })?;
    drop(pair.slave);
    let mut reader = pair
        .master
        .try_clone_reader()
        .map_err(|_| SessionError::LocalShellFailed)?;
    let mut writer = pair
        .master
        .take_writer()
        .map_err(|_| SessionError::LocalShellFailed)?;
    let killer = child.clone_killer();

    let id = SessionId::new();
    let (event_sender, event_receiver) = async_channel::bounded(EVENT_QUEUE_CAPACITY);
    let (input_sender, input_receiver) = async_channel::bounded::<Bytes>(INPUT_QUEUE_CAPACITY);
    let (resize_sender, resize_receiver) = mpsc::channel::<TerminalSize>();

    // Reader: PTY output -> session events.
    let reader_events = event_sender.clone();
    thread::Builder::new()
        .name("oxide-ssh-local-read".into())
        .spawn(move || {
            let mut buffer = [0u8; 8192];
            loop {
                match reader.read(&mut buffer) {
                    Ok(0) | Err(_) => break,
                    Ok(n) => {
                        if reader_events
                            .send_blocking(SessionEvent::Output(Bytes::copy_from_slice(
                                &buffer[..n],
                            )))
                            .is_err()
                        {
                            break;
                        }
                    }
                }
            }
        })
        .map_err(|_| SessionError::LocalShellFailed)?;

    // Writer: session input -> PTY. Ends when the handle closes the channel.
    thread::Builder::new()
        .name("oxide-ssh-local-write".into())
        .spawn(move || {
            while let Ok(bytes) = input_receiver.recv_blocking() {
                if writer.write_all(&bytes).is_err() {
                    break;
                }
            }
        })
        .map_err(|_| SessionError::LocalShellFailed)?;

    // Resize: applies the latest queued size, draining stale ones.
    thread::Builder::new()
        .name("oxide-ssh-local-resize".into())
        .spawn(move || {
            while let Ok(size) = resize_receiver.recv() {
                let mut latest = size;
                while let Ok(newer) = resize_receiver.try_recv() {
                    latest = newer;
                }
                let _ = pair.master.resize(pty_size(latest));
            }
        })
        .map_err(|_| SessionError::LocalShellFailed)?;

    // Waiter: child exit -> Exited + Disconnected, mirroring the SSH flow.
    let waiter_events = event_sender.clone();
    thread::Builder::new()
        .name("oxide-ssh-local-wait".into())
        .spawn(move || {
            let status = child
                .wait()
                .ok()
                .map(|status: ExitStatus| status.exit_code());
            let _ = waiter_events.send_blocking(SessionEvent::Exited { status });
            let _ =
                waiter_events.send_blocking(SessionEvent::StateChanged(SessionState::Disconnected));
        })
        .map_err(|_| SessionError::LocalShellFailed)?;

    let _ = event_sender.send_blocking(SessionEvent::StateChanged(SessionState::Connected));

    Ok(LocalSessionHandle {
        id,
        events: Some(event_receiver),
        input: input_sender,
        resize: resize_sender,
        killer: Mutex::new(killer),
        disconnect_started: AtomicBool::new(false),
    })
}

pub struct LocalSessionHandle {
    id: SessionId,
    events: Option<Receiver<SessionEvent>>,
    input: Sender<Bytes>,
    resize: mpsc::Sender<TerminalSize>,
    killer: Mutex<Box<dyn ChildKiller + Send + Sync>>,
    disconnect_started: AtomicBool,
}

impl LocalSessionHandle {
    pub fn id(&self) -> SessionId {
        self.id
    }

    pub fn take_events(&mut self) -> Option<Receiver<SessionEvent>> {
        self.events.take()
    }

    pub fn try_send_input(&self, bytes: Bytes) -> Result<(), SendInputError> {
        self.input.try_send(bytes).map_err(|error| match error {
            TrySendError::Full(_) => SendInputError::QueueFull,
            TrySendError::Closed(_) => SendInputError::Closed,
        })
    }

    pub fn resize(&self, size: TerminalSize) {
        if size.is_valid() {
            // The resize thread drains to the latest value; a full queue only
            // means a resize is already pending.
            let _ = self.resize.send(size);
        }
    }

    pub fn disconnect(&self) {
        if self.disconnect_started.swap(true, Ordering::AcqRel) {
            return;
        }
        self.input.close();
        let _ = self.killer.lock().kill();
    }
}

impl Drop for LocalSessionHandle {
    fn drop(&mut self) {
        self.disconnect();
    }
}

fn pty_size(size: TerminalSize) -> PtySize {
    PtySize {
        rows: size.rows.min(u16::MAX as u32) as u16,
        cols: size.columns.min(u16::MAX as u32) as u16,
        pixel_width: size.pixel_width.min(u16::MAX as u32) as u16,
        pixel_height: size.pixel_height.min(u16::MAX as u32) as u16,
    }
}

fn default_shell() -> String {
    #[cfg(unix)]
    {
        std::env::var("SHELL").unwrap_or_else(|_| "/bin/sh".into())
    }
    #[cfg(windows)]
    {
        let path = std::env::var_os("PATH");
        for candidate in ["pwsh.exe", "powershell.exe"] {
            if path_has_exe(path.as_deref(), candidate) {
                return candidate.into();
            }
        }
        std::env::var("COMSPEC").unwrap_or_else(|_| "cmd.exe".into())
    }
}

#[cfg(windows)]
fn path_has_exe(path: Option<&std::ffi::OsStr>, exe: &str) -> bool {
    path.is_some_and(|p| std::env::split_paths(p).any(|dir| dir.join(exe).is_file()))
}

fn home_dir() -> Option<String> {
    #[cfg(unix)]
    {
        std::env::var("HOME").ok()
    }
    #[cfg(windows)]
    {
        std::env::var("USERPROFILE").ok()
    }
}

#[cfg(all(test, windows))]
mod windows_tests {
    use super::*;

    #[test]
    fn path_has_exe_finds_file_only_in_listed_dirs() {
        let dir = std::env::temp_dir().join(format!("oxide-path-test-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("pwsh.exe"), b"").unwrap();
        let path = std::env::join_paths([&dir]).unwrap();

        assert!(path_has_exe(Some(&path), "pwsh.exe"));
        assert!(!path_has_exe(Some(&path), "powershell.exe"));
        assert!(!path_has_exe(None, "pwsh.exe"));

        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn local_shell_starts_in_configured_working_directory() {
        let directory =
            std::env::temp_dir().join(format!("oxide-ssh-cwd-{}", SessionId::new().0));
        std::fs::create_dir(&directory).unwrap();
        let expected = directory.to_string_lossy().into_owned();
        let mut handle = spawn_local_shell(
            TerminalSize {
                columns: 80,
                rows: 24,
                pixel_width: 800,
                pixel_height: 480,
            },
            &expected,
        )
        .unwrap();
        let events = handle.take_events().unwrap();
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(10);
        while !matches!(
            events.recv_blocking().unwrap(),
            SessionEvent::StateChanged(SessionState::Connected)
        ) {}
        handle
            .try_send_input(Bytes::from_static(b"\x1b[1;1R"))
            .unwrap();
        let command: &[u8] = if default_shell().to_ascii_lowercase().contains("cmd") {
            b"cd\r"
        } else {
            b"pwd\r"
        };
        handle.try_send_input(Bytes::copy_from_slice(command)).unwrap();
        handle.try_send_input(Bytes::from_static(b"exit\r")).unwrap();
        let mut output = Vec::new();
        while std::time::Instant::now() < deadline {
            match events.try_recv() {
                Ok(SessionEvent::Output(bytes)) => output.extend_from_slice(&bytes),
                Ok(SessionEvent::Exited { .. }) => break,
                _ => std::thread::yield_now(),
            }
        }
        let output = String::from_utf8_lossy(&output);
        assert!(
            output.to_ascii_lowercase().contains(&expected.to_ascii_lowercase()),
            "shell output did not contain {expected:?}: {output:?}"
        );
        std::fs::remove_dir_all(directory).unwrap();
    }
}

#[cfg(all(test, unix))]
mod tests {
    use super::*;

    #[cfg(unix)]
    fn size() -> TerminalSize {
        TerminalSize {
            columns: 80,
            rows: 24,
            pixel_width: 800,
            pixel_height: 480,
        }
    }

    #[test]
    fn local_shell_starts_in_configured_working_directory() {
        let directory = std::env::temp_dir().join(format!("oxide-ssh-cwd-{}", SessionId::new().0));
        std::fs::create_dir(&directory).unwrap();
        let expected = directory.to_string_lossy().into_owned();
        let mut handle = spawn_local_shell(size(), &expected).unwrap();
        let events = handle.take_events().unwrap();
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(10);
        while !matches!(events.recv_blocking().unwrap(), SessionEvent::StateChanged(SessionState::Connected)) {}
        handle.try_send_input(Bytes::from_static(b"pwd; echo oxide-cwd-marker; exit\n")).unwrap();
        let mut output = Vec::new();
        while std::time::Instant::now() < deadline {
            match events.try_recv() {
                Ok(SessionEvent::Output(bytes)) => output.extend_from_slice(&bytes),
                Ok(SessionEvent::Exited { .. }) => break,
                _ => std::thread::yield_now(),
            }
        }
        assert!(output.windows(expected.len()).any(|part| part == expected.as_bytes()));
        std::fs::remove_dir_all(directory).unwrap();
    }
    #[test]
    fn local_shell_echoes_and_exits() {
        let mut handle = spawn_local_shell(size(), "");

        let events = handle.take_events().expect("events");
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(10);
        // Wait for Connected, then drive the shell.
        loop {
            match events.recv_blocking().expect("event before deadline") {
                SessionEvent::StateChanged(SessionState::Connected) => break,
                _ => assert!(std::time::Instant::now() < deadline, "timed out"),
            }
        }
        handle
            .try_send_input(Bytes::from_static(b"echo oxide-marker-42; exit\n"))
            .expect("input");
        let mut saw_marker = false;
        let mut exit_status = None;
        while std::time::Instant::now() < deadline && (!saw_marker || exit_status.is_none()) {
            match events.try_recv() {
                Ok(SessionEvent::Output(bytes)) => {
                    if bytes.windows(15).any(|w| w == b"oxide-marker-42") {
                        saw_marker = true;
                    }
                }
                Ok(SessionEvent::Exited { status }) => exit_status = Some(status),
                Ok(_) | Err(async_channel::TryRecvError::Empty) => {
                    std::thread::yield_now();
                }
                Err(async_channel::TryRecvError::Closed) => break,
            }
        }
        assert!(saw_marker, "shell output never arrived");
        assert_eq!(exit_status, Some(Some(0)));
    }
}
#[cfg(test)]
mod working_directory_tests {
    use super::*;

    #[test]
    fn rejects_missing_working_directory() {
        let directory =
            std::env::temp_dir().join(format!("oxide-ssh-missing-{}", SessionId::new().0));
        let size = TerminalSize {
            columns: 80,
            rows: 24,
            pixel_width: 800,
            pixel_height: 480,
        };
        assert!(matches!(
            spawn_local_shell(size, directory.to_str().unwrap()),
            Err(SessionError::LocalWorkingDirectoryInvalid)
        ));
    }

    #[test]
    fn rejects_file_as_working_directory() {
        let file = std::env::temp_dir().join(format!("oxide-ssh-file-{}", SessionId::new().0));
        std::fs::write(&file, b"not a directory").unwrap();
        let size = TerminalSize {
            columns: 80,
            rows: 24,
            pixel_width: 800,
            pixel_height: 480,
        };
        assert!(matches!(
            spawn_local_shell(size, file.to_str().unwrap()),
            Err(SessionError::LocalWorkingDirectoryInvalid)
        ));
        std::fs::remove_file(file).unwrap();
    }
}
