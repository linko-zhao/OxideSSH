#![cfg(feature = "ghostty-vt")]

use oxide_ssh_terminal::{GhosttyTerminal, TerminalAction};

#[test]
fn terminal_writes_reads_a_cell_and_is_safely_destroyed() {
    let mut terminal = GhosttyTerminal::new(80, 24).expect("create libghostty-vt terminal");
    terminal.write(b"hello\x1b[31m world\x1b[0m");
    assert_eq!(terminal.cell(0, 0).unwrap(), "h");
    drop(terminal);
}

#[test]
fn terminal_vt_parser_preserves_state_across_arbitrary_chunks() {
    let mut whole = GhosttyTerminal::new(8, 2).unwrap();
    let mut split = GhosttyTerminal::new(8, 2).unwrap();
    let output = "a\x1b[31m你\x1b[0mz".as_bytes();
    whole.write(output);
    for byte in output {
        split.write(std::slice::from_ref(byte));
    }
    for column in 0..8 {
        assert_eq!(
            split.cell(column, 0).unwrap(),
            whole.cell(column, 0).unwrap()
        );
    }
    assert_eq!(whole.cell(0, 0).unwrap(), "a");
    assert_eq!(whole.cell(1, 0).unwrap(), "你");
    assert_eq!(whole.cell(3, 0).unwrap(), "z");
}

#[test]
fn terminal_effects_are_queued_as_actions_after_vt_write() {
    let mut terminal = GhosttyTerminal::new(80, 24).unwrap();
    let actions = terminal.process_output(b"\x1b[5n\x07");
    assert!(actions.iter().any(|action| matches!(
        action,
        TerminalAction::WriteBack(bytes) if bytes.as_ref() == b"\x1b[0n"
    )));
    assert!(actions.contains(&TerminalAction::Bell));
    assert!(actions.contains(&TerminalAction::Wakeup));
}

#[test]
fn terminal_osc52_copy_is_an_action_not_a_host_clipboard_call() {
    let mut terminal = GhosttyTerminal::new(80, 24).unwrap();
    let actions = terminal.process_output(b"\x1b]52;c;SGVsbG8=\x07");
    assert!(actions.contains(&TerminalAction::Clipboard("Hello".into())));
}

#[test]
fn ghostty_key_encoding_tracks_terminal_cursor_modes_without_kitty() {
    use oxide_ssh_terminal::{Key, KeyInput};

    let mut terminal = GhosttyTerminal::new(80, 24).unwrap();
    assert_eq!(
        terminal.encode_key(KeyInput::new(Key::ArrowUp)).as_slice(),
        b"\x1b[A"
    );
    terminal.process_output(b"\x1b[?1h");
    assert_eq!(
        terminal.encode_key(KeyInput::new(Key::ArrowUp)).as_slice(),
        b"\x1bOA"
    );
    assert_eq!(terminal.encode_key(KeyInput::text("x")).as_slice(), b"x");
}

#[test]
fn ghostty_paste_encoding_tracks_bracketed_mode_and_limits() {
    use oxide_ssh_terminal::{MAX_PASTE_BYTES, PasteError};

    let mut terminal = GhosttyTerminal::new(80, 24).unwrap();
    assert_eq!(
        terminal.encode_paste("one\r\ntwo").unwrap().as_ref(),
        b"one\rtwo"
    );
    terminal.process_output(b"\x1b[?2004h");
    assert_eq!(
        terminal.encode_paste("one\r\ntwo").unwrap().as_ref(),
        b"\x1b[200~one\ntwo\x1b[201~"
    );
    assert_eq!(
        terminal.encode_paste(&"x".repeat(MAX_PASTE_BYTES + 1)),
        Err(PasteError::TooLarge)
    );
}

#[test]
fn ghostty_key_encoder_preserves_existing_key_bytes_and_ignores_kitty_mode() {
    use oxide_ssh_terminal::{Key, KeyInput};

    let mut terminal = GhosttyTerminal::new(80, 24).unwrap();
    assert_eq!(
        terminal.encode_key(KeyInput::new(Key::Tab)).as_slice(),
        b"\t"
    );
    assert_eq!(
        terminal
            .encode_key(KeyInput::new(Key::Tab).shift())
            .as_slice(),
        b"\x1b[Z"
    );
    assert_eq!(
        terminal
            .encode_key(KeyInput::text("c").control())
            .as_slice(),
        &[3]
    );
    assert_eq!(
        terminal.encode_key(KeyInput::text("x").alt()).as_slice(),
        b"\x1bx"
    );
    terminal.process_output(b"\x1b[>1u");
    assert_eq!(
        terminal.encode_key(KeyInput::new(Key::ArrowUp)).as_slice(),
        b"\x1b[A"
    );
    let long_text = "a".repeat(256);
    assert_eq!(
        terminal.encode_key(KeyInput::text(&long_text)).as_slice(),
        long_text.as_bytes()
    );
}
