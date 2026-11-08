//! Portable terminal state, input encoding, selection, and render snapshots.

#[cfg(feature = "ghostty-vt")]
mod ghostty;
mod input;
mod model;

#[cfg(feature = "ghostty-vt")]
pub use ghostty::GhosttyTerminal;

pub use input::{Key, KeyInput, MAX_PASTE_BYTES, Modifiers, PasteError};
pub use model::{
    CellRenderStyle, CellSide, RgbColor, SCROLLBACK_HISTORY_LINES, TerminalAction, TerminalCell,
    TerminalColors, TerminalCursor, TerminalCursorShape, TerminalEngine, TerminalError,
    TerminalPoint, TerminalSize, TerminalSnapshot,
};
