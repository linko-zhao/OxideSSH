#![cfg(feature = "ghostty-vt")]

use oxide_ssh_terminal::GhosttyTerminal;

#[test]
fn terminal_writes_reads_a_cell_and_is_safely_destroyed() {
    let mut terminal = GhosttyTerminal::new(80, 24).expect("create libghostty-vt terminal");
    terminal.write(b"hello\x1b[31m world\x1b[0m");
    assert_eq!(terminal.cell(0, 0).unwrap(), "h");
    drop(terminal);
}
