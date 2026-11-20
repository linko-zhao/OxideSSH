use oxide_ssh_terminal::{RgbColor, TerminalEngine, TerminalSize};

fn engine(columns: usize, rows: usize) -> TerminalEngine {
    TerminalEngine::new(TerminalSize {
        columns,
        rows,
        pixel_width: (columns * 10) as u32,
        pixel_height: (rows * 20) as u32,
    })
    .unwrap()
}

#[test]
fn snapshot_exposes_text_styles_and_wide_cells() {
    let mut engine = engine(10, 2);
    engine.process_output("A\x1b[38;5;196m你".as_bytes());

    let snapshot = engine.snapshot();
    assert_eq!(snapshot.cursor().point.column, 3);
    let cells: Vec<_> = snapshot.cells().collect();
    let a = cells.iter().find(|cell| cell.character == 'A').unwrap();
    assert_eq!(a.point.line, 0);
    assert_eq!(a.point.column, 0);
    assert_eq!(a.style.foreground, RgbColor::new(0xe6, 0xe8, 0xeb));

    let cjk = cells.iter().find(|cell| cell.character == '你').unwrap();
    assert!(cjk.style.wide);
    assert_eq!(cjk.style.foreground, RgbColor::new(255, 0, 0));
    assert!(cells.iter().any(|cell| cell.style.wide_spacer));
}

#[test]
fn snapshot_exposes_combining_text_selection_and_cursor() {
    let mut engine = engine(10, 2);
    engine.process_output("e\u{301}".as_bytes());
    engine.start_selection(0, 0, oxide_ssh_terminal::CellSide::Left);
    engine.update_selection(0, 1, oxide_ssh_terminal::CellSide::Right);

    let snapshot = engine.snapshot();
    let cursor = snapshot.cursor();
    assert_eq!(cursor.point.line, 0);
    assert_eq!(cursor.point.column, 1);
    assert_eq!(cursor.shape, oxide_ssh_terminal::TerminalCursorShape::Block);
    let cell = snapshot.cells().find(|cell| cell.character == 'e').unwrap();
    assert!(cell.selected);
    assert_eq!(cell.zerowidth(), Some(&['\u{301}'][..]));
}

#[test]
fn snapshot_exposes_emoji_sequences_and_grid_columns() {
    let mut engine = engine(10, 2);
    engine.process_output("✈️A👩\u{200d}💻B".as_bytes());

    let snapshot = engine.snapshot();
    assert_eq!(snapshot.cursor().point.column, 7);
    let cells: Vec<_> = snapshot.cells().collect();
    let airplane = cells.iter().find(|cell| cell.character == '✈').unwrap();
    assert_eq!(airplane.point.column, 0);
    assert!(!airplane.style.wide);
    assert_eq!(airplane.zerowidth(), Some(&['\u{fe0f}'][..]));
    let ascii = cells.iter().find(|cell| cell.character == 'A').unwrap();
    assert_eq!(ascii.point.column, 1);
    let woman = cells.iter().find(|cell| cell.character == '👩').unwrap();
    assert_eq!(woman.point.column, 2);
    assert!(woman.style.wide);
    assert_eq!(woman.zerowidth(), Some(&['\u{200d}'][..]));
    let laptop = cells.iter().find(|cell| cell.character == '💻').unwrap();
    assert_eq!(laptop.point.column, 4);
    assert!(laptop.style.wide);
    let trailing = cells.iter().find(|cell| cell.character == 'B').unwrap();
    assert_eq!(trailing.point.column, 6);
}

#[test]
fn snapshot_exposes_alternate_screen_cursor_shape_and_scroll_offset() {
    let mut engine = engine(10, 2);
    engine.process_output(b"one\r\ntwo\r\nthree\x1b[5 q\x1b[?1049halt");
    let snapshot = engine.snapshot();
    assert!(snapshot.is_alternate_screen());
    assert_eq!(
        snapshot.cursor().shape,
        oxide_ssh_terminal::TerminalCursorShape::Beam
    );
    assert!(snapshot.cells().any(|cell| cell.character == 'a'));
    drop(snapshot);
    engine.process_output(b"\x1b[?1049l");
    engine.scroll_display(1);
    assert_eq!(engine.snapshot().display_offset(), 1);
}
