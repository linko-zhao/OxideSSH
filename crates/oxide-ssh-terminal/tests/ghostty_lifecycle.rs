use oxide_ssh_terminal::{CellSide, Key, KeyInput, TerminalAction, TerminalEngine, TerminalSize};

fn engine(columns: usize, rows: usize) -> TerminalEngine {
    TerminalEngine::new(TerminalSize {
        columns,
        rows,
        pixel_width: (columns * 8) as u32,
        pixel_height: (rows * 16) as u32,
    })
    .unwrap()
}

#[test]
fn default_engine_processes_vt_resize_and_exposes_neutral_snapshot() {
    let mut terminal = engine(8, 2);
    let actions = terminal.process_output(b"hello\x07");
    assert!(actions.contains(&TerminalAction::Bell));
    assert!(
        terminal
            .snapshot()
            .cells()
            .any(|cell| cell.character == 'h')
    );

    terminal
        .resize(TerminalSize {
            columns: 10,
            rows: 3,
            pixel_width: 80,
            pixel_height: 48,
        })
        .unwrap();
    assert_eq!(terminal.snapshot().size().columns, 10);
    assert_eq!(terminal.snapshot().size().rows, 3);
}

#[test]
fn public_engine_selection_and_mode_aware_input_work() {
    let mut terminal = engine(8, 2);
    terminal.process_output(b"hello\x1b[?1h\x1b[?2004h");
    assert_eq!(
        terminal.encode_key(KeyInput::new(Key::ArrowUp)).as_slice(),
        b"\x1bOA"
    );
    assert_eq!(
        terminal.encode_paste("a\nb").unwrap().as_ref(),
        b"\x1b[200~a\nb\x1b[201~"
    );

    terminal.start_selection(0, 0, CellSide::Left);
    terminal.update_selection(0, 4, CellSide::Right);
    assert_eq!(terminal.selected_text().as_deref(), Some("hello"));
    assert!(
        terminal
            .snapshot()
            .cells()
            .filter(|cell| cell.selected)
            .count()
            >= 5
    );
}
#[test]
fn public_snapshot_is_chunking_equivalent_and_preserves_inverse_style() {
    let output = b"A\x1b[7mC";
    let mut whole = engine(12, 2);
    let mut chunked = engine(12, 2);
    whole.process_output(output);
    for byte in output {
        chunked.process_output(std::slice::from_ref(byte));
    }
    let whole_snapshot = whole.snapshot();
    let whole_cells: Vec<_> = whole_snapshot
        .cells()
        .map(|cell| (cell.character, cell.style, cell.point))
        .collect();
    let chunked_snapshot = chunked.snapshot();
    let chunked_cells: Vec<_> = chunked_snapshot
        .cells()
        .map(|cell| (cell.character, cell.style, cell.point))
        .collect();
    assert_eq!(whole_cells, chunked_cells);
    let baseline = whole_cells
        .iter()
        .find(|(character, _, _)| *character == 'A')
        .unwrap()
        .1;
    let inverse = whole_cells
        .iter()
        .find(|(character, _, _)| *character == 'C')
        .unwrap()
        .1;
    assert_eq!(inverse.foreground, baseline.background);
    assert_eq!(inverse.background, baseline.foreground);
}

#[test]
fn resize_keeps_existing_output_and_osc52_is_a_clipboard_action() {
    let mut terminal = engine(8, 2);
    terminal.process_output(b"retained");
    terminal
        .resize(TerminalSize {
            columns: 12,
            rows: 3,
            pixel_width: 96,
            pixel_height: 48,
        })
        .unwrap();
    assert!(
        terminal
            .snapshot()
            .cells()
            .any(|cell| cell.character == 'r')
    );

    let actions = terminal.process_output(b"\x1b]52;c;SGVsbG8=\x07");
    assert!(actions.contains(&TerminalAction::Clipboard("Hello".into())));
}
#[test]
fn scrolled_snapshots_keep_terminal_relative_rows_beyond_viewport_height() {
    let mut terminal = engine(12, 2);
    for line in 0..40 {
        terminal.process_output(format!("line{line:02}\r\n").as_bytes());
    }
    terminal.scroll_display(27);
    let snapshot = terminal.snapshot();
    let offset = snapshot.display_offset();
    assert!(offset >= snapshot.size().rows);
    let first = snapshot.cells().find(|cell| cell.point.column == 0).unwrap();
    assert_eq!(first.point.line, -(offset as i32));
}

#[test]
fn selection_endpoints_preserve_cell_sides_for_copy() {
    let mut terminal = engine(8, 2);
    terminal.process_output(b"abcde");
    terminal.start_selection(0, 1, CellSide::Right);
    terminal.update_selection(0, 3, CellSide::Left);
    assert_eq!(terminal.selected_text().as_deref(), Some("c"));
    let selected: String = terminal
        .snapshot()
        .cells()
        .filter(|cell| cell.selected)
        .map(|cell| cell.character)
        .collect();
    assert_eq!(selected, "c");
    terminal.clear_selection();
    terminal.start_selection(0, 1, CellSide::Left);
    terminal.update_selection(0, 1, CellSide::Left);
    assert_eq!(terminal.selected_text().as_deref(), Some("b"));
}

#[test]
fn copied_selection_includes_unvisited_history_rows_and_soft_wraps() {
    let mut terminal = engine(4, 2);
    terminal.process_output(b"one\r\ntwo\r\nthree\r\nfour\r\nfive");
    terminal.scroll_display(3);
    terminal.start_selection(0, 0, CellSide::Left);
    terminal.scroll_display(-3);
    terminal.update_selection(1, 3, CellSide::Right);
    assert_eq!(
        terminal.selected_text().as_deref(),
        Some("two\nthree\nfour\nfive")
    );
    assert_eq!(terminal.snapshot().display_offset(), 0);

    let mut wrapped = engine(4, 2);
    wrapped.process_output("ab你z".as_bytes());
    wrapped.start_selection(0, 0, CellSide::Left);
    wrapped.update_selection(1, 0, CellSide::Right);
    assert_eq!(wrapped.selected_text().as_deref(), Some("ab你z"));
}

#[test]
fn cursor_style_commands_survive_chunking_and_reset_remote_ownership() {
    let mut terminal = engine(8, 2);
    for (command, blinks) in [
        (b"\x1b[0 q".as_slice(), true),
        (b"\x1b[1 q", true),
        (b"\x1b[2 q", false),
        (b"\x1b[3 q", true),
        (b"\x1b[4 q", false),
        (b"\x1b[5 q", true),
        (b"\x1b[6 q", false),
    ] {
        for byte in command {
            terminal.process_output(std::slice::from_ref(byte));
        }
        assert!(terminal.has_remote_cursor_style());
        assert_eq!(terminal.has_remote_cursor_blink(), blinks);
    }
    for reset in [b"\x1b[!p".as_slice(), b"\x1bc"] {
        for byte in reset {
            terminal.process_output(std::slice::from_ref(byte));
        }
        assert!(!terminal.has_remote_cursor_style());
        assert!(!terminal.has_remote_cursor_blink());
    }
}

#[test]
fn modify_other_keys_does_not_change_public_key_encoding() {
    let input = KeyInput::new(Key::ArrowUp).control();
    let expected = engine(8, 2).encode_key(input);
    let mut terminal = engine(8, 2);
    terminal.process_output(b"\x1b[>4;2m");
    assert_eq!(terminal.encode_key(input), expected);
}
