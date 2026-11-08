#![cfg(feature = "ghostty-vt")]

use oxide_ssh_terminal::{GhosttyTerminal, RgbColor, TerminalCursorShape};

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
fn terminal_resize_preserves_existing_cells_and_exposes_new_dimensions() {
    let mut terminal = GhosttyTerminal::new(4, 2).unwrap();
    terminal.write(b"kept");
    terminal.resize(8, 3, 8, 16).unwrap();
    assert_eq!(terminal.cell(0, 0).unwrap(), "k");
    let snapshot = terminal.snapshot().unwrap();
    assert_eq!(snapshot.size().columns, 8);
    assert_eq!(snapshot.size().rows, 3);
    assert_eq!(snapshot.size().pixel_width, 64);
    assert_eq!(snapshot.size().pixel_height, 48);
    assert!(snapshot.cells().any(|cell| cell.character == 'k'));
    assert_eq!(terminal.cell(7, 3), Err("cell row outside terminal"));
}

#[test]
fn ghostty_snapshot_exposes_backend_neutral_text_style_layout_cursor_and_screens() {
    let mut terminal = GhosttyTerminal::new(12, 3).unwrap();
    terminal.write("A\x1b[1;4;31m你e\u{301}\x1b[48;2;10;20;30;3;9;8mB\x1b[0m".as_bytes());
    {
        let snapshot = terminal.snapshot().unwrap();
        assert_eq!(snapshot.size().columns, 12);
        assert_eq!(snapshot.cursor().point.column, 5);
        assert_eq!(snapshot.cursor().shape, TerminalCursorShape::Block);
        assert!(!snapshot.is_alternate_screen());
        let cells: Vec<_> = snapshot.cells().collect();
        let red = cells.iter().find(|cell| cell.character == '你').unwrap();
        assert_eq!(red.style.foreground, RgbColor::new(204, 102, 102));
        assert!(red.style.bold);
        assert!(red.style.underline);
        assert!(red.style.wide);
        assert!(cells.iter().any(|cell| cell.style.wide_spacer));
        let combined = cells.iter().find(|cell| cell.character == 'e').unwrap();
        assert_eq!(combined.zerowidth(), Some(&['\u{301}'][..]));
        let rgb_background = cells.iter().find(|cell| cell.character == 'B').unwrap();
        assert_eq!(rgb_background.style.background, RgbColor::new(10, 20, 30));
        assert!(rgb_background.style.italic);
        assert!(rgb_background.style.strikeout);
        assert!(rgb_background.style.hidden);
    }

    terminal.write(b"\x1b[6 q");
    assert_eq!(
        terminal.snapshot().unwrap().cursor().shape,
        TerminalCursorShape::Beam
    );
    terminal.write(b"\x1b[?25l");
    assert_eq!(
        terminal.snapshot().unwrap().cursor().shape,
        TerminalCursorShape::Hidden
    );
    terminal.write(b"\x1b[?25h");

    terminal.write(b"\x1b[?1049hALT");
    assert!(terminal.snapshot().unwrap().is_alternate_screen());
    terminal.write(b"\x1b[2J\x1b[H");
    assert!(
        terminal
            .snapshot()
            .unwrap()
            .cells()
            .all(|cell| cell.character == ' ')
    );
    terminal.write(b"\x1b[?1049l");
    assert!(!terminal.snapshot().unwrap().is_alternate_screen());
    let snapshot = terminal.snapshot().unwrap();
    assert_eq!(
        snapshot
            .cells()
            .find(|cell| cell.character == 'A')
            .unwrap()
            .point
            .column,
        0
    );
    terminal.write(b"\x1b[2J\x1b[H");
    assert!(
        terminal
            .snapshot()
            .unwrap()
            .cells()
            .all(|cell| cell.character == ' ')
    );
}

#[test]
fn ghostty_snapshot_keeps_vt_chunking_equivalent() {
    let mut whole = GhosttyTerminal::new(8, 2).unwrap();
    let mut split = GhosttyTerminal::new(8, 2).unwrap();
    let output = "x\x1b[38;5;196m你\x1b[0m".as_bytes();
    whole.write(output);
    for byte in output {
        split.write(std::slice::from_ref(byte));
    }
    let whole_cells: Vec<_> = whole
        .snapshot()
        .unwrap()
        .cells()
        .map(|cell| (cell.point, cell.character, cell.style))
        .collect();
    let split_cells: Vec<_> = split
        .snapshot()
        .unwrap()
        .cells()
        .map(|cell| (cell.point, cell.character, cell.style))
        .collect();
    assert_eq!(split_cells, whole_cells);
}

#[test]
fn ghostty_snapshot_resolves_inverse_colors_for_backend_neutral_styles() {
    let mut normal = GhosttyTerminal::new(2, 1).unwrap();
    let default = normal.snapshot().unwrap().cells().next().unwrap().style;
    let mut inverse = GhosttyTerminal::new(2, 1).unwrap();
    inverse.write(b"\x1b[7mX");
    let snapshot = inverse.snapshot().unwrap();
    let cell = snapshot.cells().find(|cell| cell.character == 'X').unwrap();
    assert_eq!(cell.style.foreground, default.background);
    assert_eq!(cell.style.background, default.foreground);
}
