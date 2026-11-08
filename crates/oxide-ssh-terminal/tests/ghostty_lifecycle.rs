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
