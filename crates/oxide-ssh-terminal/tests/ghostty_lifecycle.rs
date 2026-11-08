#![cfg(feature = "ghostty-vt")]

use oxide_ssh_terminal::GhosttyTerminal;

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
    assert!(terminal.cell(7, 2).is_ok());
    assert_eq!(terminal.cell(7, 3), Err("cell row outside terminal"));
}
