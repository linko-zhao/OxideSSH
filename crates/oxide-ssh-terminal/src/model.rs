use crate::{KeyInput, PasteError, ghostty::GhosttyTerminal};
use bytes::Bytes;
use smallvec::SmallVec;
use std::cell::Ref;

pub const SCROLLBACK_HISTORY_LINES: usize = 10_000;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct TerminalSize {
    pub columns: usize,
    pub rows: usize,
    pub pixel_width: u32,
    pub pixel_height: u32,
}

impl TerminalSize {
    pub fn is_valid(self) -> bool {
        self.columns >= 2
            && self.rows >= 1
            && self.columns <= u16::MAX as usize
            && self.rows <= u16::MAX as usize
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct RgbColor {
    pub red: u8,
    pub green: u8,
    pub blue: u8,
}

impl RgbColor {
    pub const fn new(red: u8, green: u8, blue: u8) -> Self {
        Self { red, green, blue }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct TerminalColors {
    pub foreground: RgbColor,
    pub background: RgbColor,
    pub cursor: RgbColor,
    pub ansi: [RgbColor; 16],
}

impl Default for TerminalColors {
    fn default() -> Self {
        Self {
            foreground: RgbColor::new(0xe6, 0xe8, 0xeb),
            background: RgbColor::new(0x11, 0x13, 0x18),
            cursor: RgbColor::new(0xe6, 0xe8, 0xeb),
            ansi: [
                RgbColor::new(0x1b, 0x1d, 0x23),
                RgbColor::new(0xe0, 0x6c, 0x75),
                RgbColor::new(0x98, 0xc3, 0x79),
                RgbColor::new(0xe5, 0xc0, 0x7b),
                RgbColor::new(0x61, 0xaf, 0xef),
                RgbColor::new(0xc6, 0x78, 0xdd),
                RgbColor::new(0x56, 0xb6, 0xc2),
                RgbColor::new(0xab, 0xb2, 0xbf),
                RgbColor::new(0x5c, 0x63, 0x70),
                RgbColor::new(0xff, 0x7b, 0x86),
                RgbColor::new(0xb3, 0xe9, 0x8c),
                RgbColor::new(0xff, 0xd6, 0x8a),
                RgbColor::new(0x76, 0xc7, 0xff),
                RgbColor::new(0xd9, 0x9b, 0xff),
                RgbColor::new(0x70, 0xe1, 0xed),
                RgbColor::new(0xf5, 0xf7, 0xfa),
            ],
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct CellRenderStyle {
    pub foreground: RgbColor,
    pub background: RgbColor,
    pub bold: bool,
    pub italic: bool,
    pub underline: bool,
    pub strikeout: bool,
    pub hidden: bool,
    pub wide: bool,
    pub wide_spacer: bool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct TerminalPoint {
    pub line: i32,
    pub column: usize,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TerminalCursorShape {
    Hidden,
    Beam,
    HollowBlock,
    Block,
    Underline,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct TerminalCursor {
    pub point: TerminalPoint,
    pub shape: TerminalCursorShape,
}

pub struct TerminalCell<'a> {
    pub point: TerminalPoint,
    pub character: char,
    pub style: CellRenderStyle,
    pub selected: bool,
    zerowidth: Option<&'a [char]>,
}

impl TerminalCell<'_> {
    pub fn zerowidth(&self) -> Option<&[char]> {
        self.zerowidth
    }
}

pub(crate) struct SnapshotCellData {
    pub point: TerminalPoint,
    pub character: char,
    pub style: CellRenderStyle,
    pub selected: bool,
    pub zerowidth: Vec<char>,
}

pub(crate) struct GhosttySnapshotData {
    pub size: TerminalSize,
    pub cells: Vec<SnapshotCellData>,
    pub history_size: usize,
    pub display_offset: usize,
    pub alternate_screen: bool,
    pub cursor: TerminalCursor,
}

pub struct TerminalSnapshot<'a>(Ref<'a, GhosttySnapshotData>);

impl<'a> TerminalSnapshot<'a> {
    pub(crate) fn ghostty(data: Ref<'a, GhosttySnapshotData>) -> Self {
        Self(data)
    }
    pub fn cells(&self) -> impl Iterator<Item = TerminalCell<'_>> + use<'_> {
        self.0.cells.iter().map(|cell| TerminalCell {
            point: cell.point,
            character: cell.character,
            style: cell.style,
            selected: cell.selected,
            zerowidth: (!cell.zerowidth.is_empty()).then_some(cell.zerowidth.as_slice()),
        })
    }
    pub fn size(&self) -> TerminalSize {
        self.0.size
    }
    pub fn display_offset(&self) -> usize {
        self.0.display_offset
    }
    pub fn is_alternate_screen(&self) -> bool {
        self.0.alternate_screen
    }
    pub fn cursor(&self) -> TerminalCursor {
        self.0.cursor
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum TerminalAction {
    WriteBack(Bytes),
    Bell,
    Clipboard(String),
    Wakeup,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TerminalError {
    InvalidSize,
}

impl std::fmt::Display for TerminalError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("invalid terminal size")
    }
}
impl std::error::Error for TerminalError {}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CellSide {
    Left,
    Right,
}

pub struct TerminalEngine {
    terminal: GhosttyTerminal,
    size: TerminalSize,
}

impl TerminalEngine {
    pub fn new(size: TerminalSize) -> Result<Self, TerminalError> {
        Self::with_colors(size, TerminalColors::default())
    }
    pub fn with_colors(
        size: TerminalSize,
        default_colors: TerminalColors,
    ) -> Result<Self, TerminalError> {
        Self::with_scrollback(size, default_colors, SCROLLBACK_HISTORY_LINES)
    }
    pub fn with_scrollback(
        size: TerminalSize,
        default_colors: TerminalColors,
        _scrollback_lines: usize,
    ) -> Result<Self, TerminalError> {
        if !size.is_valid() {
            return Err(TerminalError::InvalidSize);
        }
        let mut terminal = GhosttyTerminal::new(
            size.columns as u16,
            size.rows as u16,
            _scrollback_lines,
            default_colors,
        )
        .map_err(|_| TerminalError::InvalidSize)?;
        terminal
            .resize(
                size.columns as u16,
                size.rows as u16,
                size.pixel_width / size.columns as u32,
                size.pixel_height / size.rows as u32,
            )
            .map_err(|_| TerminalError::InvalidSize)?;
        Ok(Self { terminal, size })
    }
    pub fn process_output(&mut self, bytes: &[u8]) -> SmallVec<[TerminalAction; 4]> {
        self.terminal.process_output(bytes)
    }
    pub fn resize(&mut self, size: TerminalSize) -> Result<(), TerminalError> {
        if !size.is_valid() {
            return Err(TerminalError::InvalidSize);
        }
        self.terminal
            .resize(
                size.columns as u16,
                size.rows as u16,
                size.pixel_width / size.columns as u32,
                size.pixel_height / size.rows as u32,
            )
            .map_err(|_| TerminalError::InvalidSize)?;
        self.size = size;
        Ok(())
    }
    pub fn size(&self) -> TerminalSize {
        self.size
    }
    pub fn set_default_colors(&mut self, colors: TerminalColors) {
        self.terminal.set_default_colors(colors);
    }
    pub fn encode_key(&self, input: KeyInput<'_>) -> SmallVec<[u8; 16]> {
        self.terminal.encode_key(input)
    }
    pub fn encode_paste(&self, text: &str) -> Result<Bytes, PasteError> {
        self.terminal.encode_paste(text)
    }
    pub fn history_size(&self) -> usize {
        self.terminal
            .snapshot()
            .map(|s| s.0.history_size)
            .unwrap_or(0)
    }
    pub fn snapshot(&self) -> TerminalSnapshot<'_> {
        self.terminal
            .snapshot()
            .expect("libghostty-vt snapshot failed")
    }
    pub fn cursor_blinking(&self) -> bool {
        self.terminal.cursor_blinking()
    }
    pub fn has_remote_cursor_style(&self) -> bool {
        self.terminal.has_remote_cursor_style()
    }
    pub fn has_remote_cursor_blink(&self) -> bool {
        self.terminal.has_remote_cursor_blink()
    }
    pub fn scroll_display(&mut self, lines: i32) {
        self.terminal.scroll_display(lines);
    }
    pub fn start_selection(&mut self, row: usize, column: usize, side: CellSide) {
        self.terminal.start_selection(row, column, side);
    }
    pub fn update_selection(&mut self, row: usize, column: usize, side: CellSide) {
        self.terminal.update_selection(row, column, side);
    }
    pub fn clear_selection(&mut self) {
        self.terminal.clear_selection();
    }
    pub fn selected_text(&self) -> Option<String> {
        self.terminal.selected_text()
    }
}
