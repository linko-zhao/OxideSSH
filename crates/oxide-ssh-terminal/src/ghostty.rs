use crate::model::{
    CellRenderStyle, GhosttySnapshotData, RgbColor, SnapshotCellData, TerminalCursor,
    TerminalCursorShape, TerminalPoint, TerminalSize, TerminalSnapshot,
};
use alacritty_terminal::term::TermMode;
use bytes::Bytes;
use parking_lot::Mutex;
use smallvec::SmallVec;
use std::{ffi::c_void, ptr::NonNull};

use crate::{CellSide, Key, KeyInput, PasteError, TerminalAction};

const OPT_USERDATA: i32 = 1;
const OPT_WRITE_PTY: i32 = 2;
const OPT_BELL: i32 = 3;
const OPT_CLIPBOARD_WRITE: i32 = 27;

const OPT_KITTY_FLAGS: i32 = 6;
const DATA_MODE: i32 = 37;

#[repr(C)]
struct KeyEncoderOpaque {
    _private: [u8; 0],
}
#[repr(C)]
struct KeyEventOpaque {
    _private: [u8; 0],
}
#[repr(C)]
struct TerminalModeConfig {
    mode: u16,
    value: bool,
}

#[repr(C)]
struct GhosttyString {
    ptr: *const u8,
    len: usize,
}

#[repr(C)]
struct ClipboardContent {
    mime: GhosttyString,
    data: GhosttyString,
}

#[repr(C)]
struct ClipboardWriteReply {
    size: usize,
    result: i32,
    remember: bool,
}

type ClipboardWriteReplyFn =
    unsafe extern "C" fn(*const ClipboardWrite, *const ClipboardWriteReply);

#[repr(C)]
struct ClipboardWrite {
    location: i32,
    contents: *const ClipboardContent,
    contents_len: usize,
    name: GhosttyString,
    granted: bool,
    can_remember: bool,
    ctx: *const c_void,
    reply: Option<ClipboardWriteReplyFn>,
}

#[repr(C)]
struct TerminalOpaque {
    _private: [u8; 0],
}
#[repr(C)]
struct RenderStateOpaque {
    _private: [u8; 0],
}
#[repr(C)]
struct RowIteratorOpaque {
    _private: [u8; 0],
}
#[repr(C)]
struct RowCellsOpaque {
    _private: [u8; 0],
}
#[repr(C)]
struct GhosttyBuffer {
    ptr: *mut u8,
    cap: usize,
    len: usize,
}

#[repr(C)]
struct GhosttyCodepointBuffer {
    ptr: *mut u32,
    cap: usize,
    len: usize,
}

#[repr(C)]
#[derive(Clone, Copy, Default)]
struct GhosttyRgb {
    r: u8,
    g: u8,
    b: u8,
}

#[repr(C)]
#[derive(Clone, Copy, Default)]
struct GhosttyStyleColor {
    tag: u8,
    value: [u8; 3],
}

#[repr(C)]
#[derive(Clone, Copy, Default)]
struct GhosttyStyle {
    fg_color: GhosttyStyleColor,
    bg_color: GhosttyStyleColor,
    underline_color: GhosttyStyleColor,
    flags: u16,
    reserved: u16,
}

const RENDER_COLS: i32 = 1;
const RENDER_ROWS: i32 = 2;
const RENDER_ROWS_ITER: i32 = 4;
const RENDER_BG: i32 = 5;
const RENDER_FG: i32 = 6;
const RENDER_CURSOR_SHAPE: i32 = 10;
const RENDER_CURSOR_VISIBLE: i32 = 11;
const ROW_CELLS: i32 = 3;
const CELL_RAW: i32 = 1;
const CELL_STYLE: i32 = 2;
const CELL_BG: i32 = 5;
const CELL_FG: i32 = 6;
const CELL_TEXT_UTF8: i32 = 9;
const CELL_GRAPHEMES_LEN: i32 = 3;
const CELL_GRAPHEMES_BUF: i32 = 4;
const STYLE_BOLD: u16 = 1 << 0;
const STYLE_ITALIC: u16 = 1 << 1;
const STYLE_HIDDEN: u16 = 1 << 5;
const STYLE_STRIKEOUT: u16 = 1 << 6;
const STYLE_INVERSE: u16 = 1 << 4;
const CELL_WIDE: i32 = 3;
const TERMINAL_CURSOR_X: i32 = 3;
const TERMINAL_CURSOR_Y: i32 = 4;
const TERMINAL_ACTIVE_SCREEN: i32 = 6;
const TERMINAL_DATA_SCROLLBAR: i32 = 9;
const SCROLL_VIEWPORT_DELTA: i32 = 2;

#[repr(C)]
union ScrollViewportValue {
    delta: isize,
    _padding: [u64; 2],
}

#[repr(C)]
struct ScrollViewport {
    tag: i32,
    value: ScrollViewportValue,
}

#[repr(C)]
#[derive(Default)]
struct TerminalScrollbar {
    total: u64,
    offset: u64,
    len: u64,
}

fn rgb(color: GhosttyRgb) -> RgbColor {
    RgbColor::new(color.r, color.g, color.b)
}

unsafe extern "C" {
    fn ghostty_terminal_scroll_viewport(
        terminal: *mut TerminalOpaque,
        behavior: ScrollViewport,
    ) -> i32;
}

unsafe extern "C" {
    fn ghostty_terminal_new(
        allocator: *const c_void,
        terminal: *mut *mut TerminalOpaque,
        cols: u16,
        rows: u16,
    ) -> i32;
    fn ghostty_terminal_set(
        terminal: *mut TerminalOpaque,
        option: i32,
        value: *const c_void,
    ) -> i32;
    fn ghostty_terminal_vt_write(terminal: *mut TerminalOpaque, data: *const u8, len: usize);
    fn ghostty_terminal_resize(
        terminal: *mut TerminalOpaque,
        cols: u16,
        rows: u16,
        cell_width_px: u32,
        cell_height_px: u32,
    ) -> i32;
    fn ghostty_terminal_get(
        terminal: *mut TerminalOpaque,
        data: i32,
        out: *mut std::ffi::c_void,
    ) -> i32;
    fn ghostty_terminal_free(terminal: *mut TerminalOpaque);
    fn ghostty_render_state_new(
        allocator: *const c_void,
        state: *mut *mut RenderStateOpaque,
    ) -> i32;
    fn ghostty_render_state_free(state: *mut RenderStateOpaque);
    fn ghostty_render_state_update(
        state: *mut RenderStateOpaque,
        terminal: *mut TerminalOpaque,
    ) -> i32;
    fn ghostty_render_state_row_iterator_new(
        allocator: *const c_void,
        out: *mut *mut RowIteratorOpaque,
    ) -> i32;
    fn ghostty_render_state_row_iterator_free(iterator: *mut RowIteratorOpaque);
    fn ghostty_render_state_get(state: *mut RenderStateOpaque, data: i32, out: *mut c_void) -> i32;
    fn ghostty_render_state_row_iterator_next(iterator: *mut RowIteratorOpaque) -> bool;
    fn ghostty_render_state_row_get(
        iterator: *mut RowIteratorOpaque,
        data: i32,
        out: *mut c_void,
    ) -> i32;
    fn ghostty_render_state_row_cells_new(
        allocator: *const c_void,
        out: *mut *mut RowCellsOpaque,
    ) -> i32;
    fn ghostty_render_state_row_cells_free(cells: *mut RowCellsOpaque);
    fn ghostty_render_state_row_cells_select(cells: *mut RowCellsOpaque, x: u16) -> i32;
    fn ghostty_render_state_row_cells_get(
        cells: *mut RowCellsOpaque,
        data: i32,
        out: *mut c_void,
    ) -> i32;
    fn ghostty_cell_get(cell: u64, data: i32, out: *mut std::ffi::c_void) -> i32;
    fn ghostty_key_encoder_new(allocator: *const c_void, out: *mut *mut KeyEncoderOpaque) -> i32;
    fn ghostty_key_encoder_free(encoder: *mut KeyEncoderOpaque);
    fn ghostty_key_encoder_set(
        encoder: *mut KeyEncoderOpaque,
        option: i32,
        value: *const c_void,
    ) -> i32;
    fn ghostty_key_encoder_set_from_terminal(
        encoder: *mut KeyEncoderOpaque,
        terminal: *mut TerminalOpaque,
    ) -> i32;
    fn ghostty_key_encoder_encode(
        encoder: *mut KeyEncoderOpaque,
        event: *mut KeyEventOpaque,
        buf: *mut u8,
        cap: usize,
        written: *mut usize,
    ) -> i32;
    fn ghostty_key_event_new(allocator: *const c_void, out: *mut *mut KeyEventOpaque) -> i32;
    fn ghostty_key_event_free(event: *mut KeyEventOpaque);
    fn ghostty_key_event_set_key(event: *mut KeyEventOpaque, key: i32) -> i32;
    fn ghostty_key_event_set_mods(event: *mut KeyEventOpaque, mods: u16) -> i32;
    fn ghostty_key_event_set_utf8(event: *mut KeyEventOpaque, text: GhosttyString) -> i32;
    fn ghostty_key_event_set_action(event: *mut KeyEventOpaque, action: i32) -> i32;
}

fn queue(userdata: *mut c_void) -> &'static Mutex<Vec<TerminalAction>> {
    // SAFETY: userdata points to the boxed queue owned by the live terminal.
    unsafe { &*(userdata.cast::<Mutex<Vec<TerminalAction>>>()) }
}

unsafe extern "C" fn write_pty(
    _terminal: *mut TerminalOpaque,
    userdata: *mut c_void,
    data: *const u8,
    len: usize,
) {
    if len == 0 || !data.is_null() {
        let bytes = if len == 0 {
            &[]
        } else {
            // SAFETY: libghostty guarantees callback bytes are valid for this call.
            unsafe { std::slice::from_raw_parts(data, len) }
        };
        queue(userdata)
            .lock()
            .push(TerminalAction::WriteBack(Bytes::copy_from_slice(bytes)));
    }
}

unsafe extern "C" fn bell(_terminal: *mut TerminalOpaque, userdata: *mut c_void) {
    queue(userdata).lock().push(TerminalAction::Bell);
}

unsafe extern "C" fn clipboard_write(
    _terminal: *mut TerminalOpaque,
    userdata: *mut c_void,
    write: *const ClipboardWrite,
) {
    if let Some(write) = unsafe { write.as_ref() } {
        if write.location == 0 && write.contents_len != 0 && !write.contents.is_null() {
            // SAFETY: request contents are borrowed for the callback duration.
            let content = unsafe { &*write.contents };
            if content.mime.len == 10
                && !content.mime.ptr.is_null()
                && unsafe { std::slice::from_raw_parts(content.mime.ptr, content.mime.len) }
                    == b"text/plain"
                && (content.data.len == 0 || !content.data.ptr.is_null())
            {
                let bytes = if content.data.len == 0 {
                    &[]
                } else {
                    // SAFETY: request content is borrowed for this callback.
                    unsafe { std::slice::from_raw_parts(content.data.ptr, content.data.len) }
                };
                queue(userdata).lock().push(TerminalAction::Clipboard(
                    String::from_utf8_lossy(bytes).into_owned(),
                ));
            }
        }
        if let Some(reply) = write.reply {
            let response = ClipboardWriteReply {
                size: std::mem::size_of::<ClipboardWriteReply>(),
                result: 0,
                remember: false,
            };
            // SAFETY: reply is explicitly valid only during this callback.
            unsafe { reply(write, &response) };
        }
    }
}

/// Safe owner for the pinned libghostty-vt terminal lifecycle.
pub struct GhosttyTerminal {
    terminal: NonNull<TerminalOpaque>,
    snapshot: GhosttySnapshotData,
    cell_width_px: u32,
    cell_height_px: u32,
    pending: Box<Mutex<Vec<TerminalAction>>>,
    encoder: NonNull<KeyEncoderOpaque>,
    event: NonNull<KeyEventOpaque>,
    selection: Option<(TerminalPoint, TerminalPoint)>,
}

impl GhosttyTerminal {
    pub fn new(cols: u16, rows: u16) -> Result<Self, &'static str> {
        let mut raw = std::ptr::null_mut();
        // SAFETY: output pointer is valid and C allocator defaults when NULL.
        let result = unsafe { ghostty_terminal_new(std::ptr::null(), &mut raw, cols, rows) };
        if result != 0 {
            return Err("libghostty-vt terminal creation failed");
        }
        let terminal = NonNull::new(raw).ok_or("libghostty-vt returned a null terminal")?;
        let mut raw_encoder = std::ptr::null_mut();
        if unsafe { ghostty_key_encoder_new(std::ptr::null(), &mut raw_encoder) } != 0 {
            unsafe { ghostty_terminal_free(terminal.as_ptr()) };
            return Err("libghostty-vt key encoder creation failed");
        }
        let encoder =
            NonNull::new(raw_encoder).expect("successful encoder creation returns a handle");
        let mut raw_event = std::ptr::null_mut();
        if unsafe { ghostty_key_event_new(std::ptr::null(), &mut raw_event) } != 0 {
            unsafe {
                ghostty_key_encoder_free(encoder.as_ptr());
                ghostty_terminal_free(terminal.as_ptr());
            }
            return Err("libghostty-vt key event creation failed");
        }
        let event =
            NonNull::new(raw_event).expect("successful key event creation returns a handle");
        let pending = Box::new(Mutex::new(Vec::new()));
        let userdata = (&*pending as *const Mutex<Vec<TerminalAction>>).cast::<c_void>();
        // SAFETY: userdata stays stable for the lifetime of the terminal; callbacks only enqueue.
        unsafe {
            ghostty_terminal_set(terminal.as_ptr(), OPT_USERDATA, userdata);
            ghostty_terminal_set(
                terminal.as_ptr(),
                OPT_WRITE_PTY,
                write_pty as *const () as *const c_void,
            );
            ghostty_terminal_set(
                terminal.as_ptr(),
                OPT_BELL,
                bell as *const () as *const c_void,
            );
            ghostty_terminal_set(
                terminal.as_ptr(),
                OPT_CLIPBOARD_WRITE,
                clipboard_write as *const () as *const c_void,
            );
        }
        Ok(Self {
            terminal,
            snapshot: GhosttySnapshotData {
                size: TerminalSize {
                    columns: usize::from(cols),
                    rows: usize::from(rows),
                    pixel_width: 0,
                    pixel_height: 0,
                },
                cells: Vec::new(),
                display_offset: 0,
                alternate_screen: false,
                cursor: TerminalCursor {
                    point: TerminalPoint { line: 0, column: 0 },
                    shape: TerminalCursorShape::Block,
                },
            },
            cell_width_px: 0,
            cell_height_px: 0,
            pending,
            encoder,
            event,
            selection: None,
        })
    }

    pub fn write(&mut self, bytes: &[u8]) {
        // SAFETY: live handle and bytes are valid for the call duration.
        unsafe { ghostty_terminal_vt_write(self.terminal.as_ptr(), bytes.as_ptr(), bytes.len()) };
    }

    pub fn resize(
        &mut self,
        cols: u16,
        rows: u16,
        cell_width_px: u32,
        cell_height_px: u32,
    ) -> Result<(), &'static str> {
        // SAFETY: the terminal is live and the dimensions are plain values.
        let result = unsafe {
            ghostty_terminal_resize(
                self.terminal.as_ptr(),
                cols,
                rows,
                cell_width_px,
                cell_height_px,
            )
        };
        if result == 0 {
            self.cell_width_px = cell_width_px;
            self.cell_height_px = cell_height_px;
            Ok(())
        } else {
            Err("libghostty-vt terminal resize failed")
        }
    }
    pub fn scroll_display(&mut self, lines: i32) {
        let behavior = ScrollViewport {
            tag: SCROLL_VIEWPORT_DELTA,
            value: ScrollViewportValue {
                delta: lines.saturating_neg() as isize,
            },
        };
        // SAFETY: the terminal is live and the tagged value matches its C ABI.
        unsafe {
            ghostty_terminal_scroll_viewport(self.terminal.as_ptr(), behavior);
        }
    }
    pub fn start_selection(&mut self, row: usize, column: usize, _side: CellSide) {
        let point = TerminalPoint {
            line: row as i32,
            column,
        };
        self.selection = Some((point, point));
    }

    pub fn update_selection(&mut self, row: usize, column: usize, _side: CellSide) {
        if let Some((_, end)) = &mut self.selection {
            *end = TerminalPoint {
                line: row as i32,
                column,
            };
        }
    }

    pub fn clear_selection(&mut self) {
        self.selection = None;
    }

    pub fn selected_text(&mut self) -> Option<String> {
        self.snapshot().ok()?;
        let (start, end) = self.selection?;
        let (first, last) = if (start.line, start.column) <= (end.line, end.column) {
            (start, end)
        } else {
            (end, start)
        };
        let mut text = String::new();
        for cell in &self.snapshot.cells {
            let point = cell.point;
            if (point.line, point.column) >= (first.line, first.column)
                && (point.line, point.column) <= (last.line, last.column)
            {
                text.push(cell.character);
                text.extend(cell.zerowidth.iter());
            }
        }
        Some(text.trim_end().to_owned())
    }
    pub fn select_word(&mut self, row: usize, column: usize) {
        let Ok(snapshot) = self.snapshot() else {
            return;
        };
        let mut cells: Vec<_> = snapshot
            .cells()
            .filter(|cell| cell.point.line == row as i32)
            .collect();
        let Some(index) = cells.iter().position(|cell| cell.point.column == column) else {
            return;
        };
        let is_word = |character: char| character.is_alphanumeric() || character == '_';
        if !is_word(cells[index].character) {
            return;
        }
        let mut first = index;
        let mut last = index;
        while first > 0 && is_word(cells[first - 1].character) {
            first -= 1;
        }
        while last + 1 < cells.len() && is_word(cells[last + 1].character) {
            last += 1;
        }
        let start = TerminalPoint {
            line: row as i32,
            column: cells[first].point.column,
        };
        let end = TerminalPoint {
            line: row as i32,
            column: cells[last].point.column,
        };
        drop(cells);
        self.selection = Some((start, end));
    }


    pub fn snapshot(&mut self) -> Result<TerminalSnapshot<'_>, &'static str> {
        let (mut state, mut iterator, mut cells) = (
            std::ptr::null_mut(),
            std::ptr::null_mut(),
            std::ptr::null_mut(),
        );
        // SAFETY: each output handle is initialized by its constructor and freed below.
        unsafe {
            if ghostty_render_state_new(std::ptr::null(), &mut state) != 0 {
                return Err("render state allocation failed");
            }
            let result = (|| {
                if ghostty_render_state_update(state, self.terminal.as_ptr()) != 0 {
                    return Err("render state update failed");
                }
                let mut columns = 0u16;
                let mut rows = 0u16;
                let mut foreground = GhosttyRgb::default();
                let mut background = GhosttyRgb::default();
                if ghostty_render_state_get(state, RENDER_COLS, (&mut columns as *mut u16).cast())
                    != 0
                    || ghostty_render_state_get(state, RENDER_ROWS, (&mut rows as *mut u16).cast())
                        != 0
                    || ghostty_render_state_get(
                        state,
                        RENDER_FG,
                        (&mut foreground as *mut GhosttyRgb).cast(),
                    ) != 0
                    || ghostty_render_state_get(
                        state,
                        RENDER_BG,
                        (&mut background as *mut GhosttyRgb).cast(),
                    ) != 0
                {
                    return Err("render state metadata lookup failed");
                }
                if ghostty_render_state_row_iterator_new(std::ptr::null(), &mut iterator) != 0
                    || ghostty_render_state_get(
                        state,
                        RENDER_ROWS_ITER,
                        (&mut iterator as *mut *mut RowIteratorOpaque).cast(),
                    ) != 0
                    || ghostty_render_state_row_cells_new(std::ptr::null(), &mut cells) != 0
                {
                    return Err("render state row allocation failed");
                }
                let mut data = GhosttySnapshotData {
                    size: TerminalSize {
                        columns: usize::from(columns),
                        rows: usize::from(rows),
                        pixel_width: u32::from(columns) * self.cell_width_px,
                        pixel_height: u32::from(rows) * self.cell_height_px,
                    },
                    cells: Vec::with_capacity(usize::from(columns) * usize::from(rows)),
                    display_offset: 0,
                    alternate_screen: false,
                    cursor: TerminalCursor {
                        point: TerminalPoint { line: 0, column: 0 },
                        shape: TerminalCursorShape::Block,
                    },
                };
                let mut scrollbar = TerminalScrollbar::default();
                if ghostty_terminal_get(
                    self.terminal.as_ptr(),
                    TERMINAL_DATA_SCROLLBAR,
                    (&mut scrollbar as *mut TerminalScrollbar).cast(),
                ) != 0
                {
                    return Err("terminal scrollbar lookup failed");
                }
                data.display_offset = usize::try_from(
                    scrollbar.total.saturating_sub(scrollbar.offset.saturating_add(scrollbar.len)),
                )
                .unwrap_or(usize::MAX);
                let mut cursor_x = 0u16;
                let mut cursor_y = 0u16;
                let mut active_screen = 0i32;
                let mut cursor_visible = false;
                let mut cursor_shape = 1i32;
                if ghostty_terminal_get(
                    self.terminal.as_ptr(),
                    TERMINAL_CURSOR_X,
                    (&mut cursor_x as *mut u16).cast(),
                ) != 0
                    || ghostty_terminal_get(
                        self.terminal.as_ptr(),
                        TERMINAL_CURSOR_Y,
                        (&mut cursor_y as *mut u16).cast(),
                    ) != 0
                    || ghostty_terminal_get(
                        self.terminal.as_ptr(),
                        TERMINAL_ACTIVE_SCREEN,
                        (&mut active_screen as *mut i32).cast(),
                    ) != 0
                    || ghostty_render_state_get(
                        state,
                        RENDER_CURSOR_VISIBLE,
                        (&mut cursor_visible as *mut bool).cast(),
                    ) != 0
                    || ghostty_render_state_get(
                        state,
                        RENDER_CURSOR_SHAPE,
                        (&mut cursor_shape as *mut i32).cast(),
                    ) != 0
                {
                    return Err("terminal cursor lookup failed");
                }
                data.alternate_screen = active_screen == 1;
                data.cursor = TerminalCursor {
                    point: TerminalPoint {
                        line: i32::from(cursor_y),
                        column: usize::from(cursor_x),
                    },
                    shape: if !cursor_visible {
                        TerminalCursorShape::Hidden
                    } else {
                        match cursor_shape {
                            0 => TerminalCursorShape::Beam,
                            2 => TerminalCursorShape::Underline,
                            3 => TerminalCursorShape::HollowBlock,
                            _ => TerminalCursorShape::Block,
                        }
                    },
                };
                let mut codepoints = Vec::<u32>::new();
                for y in 0..rows {
                    if !ghostty_render_state_row_iterator_next(iterator)
                        || ghostty_render_state_row_get(
                            iterator,
                            ROW_CELLS,
                            (&mut cells as *mut *mut RowCellsOpaque).cast(),
                        ) != 0
                    {
                        return Err("render state row lookup failed");
                    }
                    for x in 0..columns {
                        if ghostty_render_state_row_cells_select(cells, x) != 0 {
                            return Err("render state cell lookup failed");
                        }
                        let mut grapheme_len = 0u32;
                        if ghostty_render_state_row_cells_get(
                            cells,
                            CELL_GRAPHEMES_LEN,
                            (&mut grapheme_len as *mut u32).cast(),
                        ) != 0
                        {
                            return Err("render state grapheme length lookup failed");
                        }
                        codepoints.resize(grapheme_len as usize, 0);
                        if !codepoints.is_empty() {
                            let mut buffer = GhosttyCodepointBuffer {
                                ptr: codepoints.as_mut_ptr(),
                                cap: codepoints.len(),
                                len: 0,
                            };
                            if ghostty_render_state_row_cells_get(
                                cells,
                                CELL_GRAPHEMES_BUF,
                                (&mut buffer as *mut GhosttyCodepointBuffer).cast(),
                            ) != 0
                            {
                                return Err("render state grapheme read failed");
                            }
                        }
                        let character = codepoints
                            .first()
                            .and_then(|codepoint| char::from_u32(*codepoint))
                            .unwrap_or(' ');
                        let zerowidth = if codepoints.len() > 1 {
                            codepoints[1..]
                                .iter()
                                .map(|codepoint| char::from_u32(*codepoint).unwrap_or('\u{fffd}'))
                                .collect()
                        } else {
                            Vec::new()
                        };
                        let mut raw_cell = 0u64;
                        let mut style = GhosttyStyle::default();
                        let mut wide = 0i32;
                        if ghostty_render_state_row_cells_get(
                            cells,
                            CELL_RAW,
                            (&mut raw_cell as *mut u64).cast(),
                        ) != 0
                            || ghostty_render_state_row_cells_get(
                                cells,
                                CELL_STYLE,
                                (&mut style as *mut GhosttyStyle).cast(),
                            ) != 0
                            || ghostty_cell_get(raw_cell, CELL_WIDE, (&mut wide as *mut i32).cast())
                                != 0
                        {
                            return Err("render state cell style lookup failed");
                        }
                        let mut fg = GhosttyRgb::default();
                        let mut bg = GhosttyRgb::default();
                        let fg_result = ghostty_render_state_row_cells_get(
                            cells,
                            CELL_FG,
                            (&mut fg as *mut GhosttyRgb).cast(),
                        );
                        let bg_result = ghostty_render_state_row_cells_get(
                            cells,
                            CELL_BG,
                            (&mut bg as *mut GhosttyRgb).cast(),
                        );
                        if fg_result == -4 {
                            fg = foreground;
                        } else if fg_result != 0 {
                            return Err("render state cell foreground lookup failed");
                        }
                        if bg_result == -4 {
                            bg = background;
                        } else if bg_result != 0 {
                            return Err("render state cell background lookup failed");
                        }
                        let flags = style.flags;
                        if flags & STYLE_INVERSE != 0 {
                            std::mem::swap(&mut fg, &mut bg);
                        }
                        data.cells.push(SnapshotCellData {
                            point: TerminalPoint {
                                line: i32::from(y),
                                column: usize::from(x),
                            },
                            character,
                            style: CellRenderStyle {
                                foreground: rgb(fg),
                                background: rgb(bg),
                                bold: flags & STYLE_BOLD != 0,
                                italic: flags & STYLE_ITALIC != 0,
                                underline: flags & 0x0700 != 0,
                                strikeout: flags & STYLE_STRIKEOUT != 0,
                                hidden: flags & STYLE_HIDDEN != 0,
                                wide: wide == 1,
                                wide_spacer: wide == 2 || wide == 3,
                            },
                            selected: self.selection.is_some_and(|(start, end)| {
                                let point = TerminalPoint {
                                    line: i32::from(y),
                                    column: usize::from(x),
                                };
                                let (first, last) =
                                    if (start.line, start.column) <= (end.line, end.column) {
                                        (start, end)
                                    } else {
                                        (end, start)
                                    };
                                (point.line, point.column) >= (first.line, first.column)
                                    && (point.line, point.column) <= (last.line, last.column)
                            }),
                            zerowidth,
                        });
                }
                }
                self.snapshot = data;
                Ok(())
            })();
            if !cells.is_null() {
                ghostty_render_state_row_cells_free(cells);
            }
            if !iterator.is_null() {
                ghostty_render_state_row_iterator_free(iterator);
            }
            ghostty_render_state_free(state);
            result?;
        }
        Ok(TerminalSnapshot::ghostty(&self.snapshot))
    }

    /// Process VT output and return queued effects only after the parser returns.
    pub fn process_output(&mut self, bytes: &[u8]) -> SmallVec<[TerminalAction; 4]> {
        self.write(bytes);
        let mut actions = SmallVec::new();
        actions.extend(std::mem::take(&mut *self.pending.lock()));
        if !bytes.is_empty() && !actions.contains(&TerminalAction::Wakeup) {
            actions.push(TerminalAction::Wakeup);
        }
        actions
    }
    pub fn encode_key(&mut self, input: KeyInput<'_>) -> SmallVec<[u8; 16]> {
        let (key, text) = match input.key {
            Key::Text(text) => (0, text.as_bytes()),
            Key::Enter => (58, &[][..]),
            Key::Backspace => (53, &[][..]),
            Key::Tab => (64, &[][..]),
            Key::Escape => (120, &[][..]),
            Key::ArrowDown => (75, &[][..]),
            Key::ArrowLeft => (76, &[][..]),
            Key::ArrowRight => (77, &[][..]),
            Key::ArrowUp => (78, &[][..]),
            Key::Home => (71, &[][..]),
            Key::End => (69, &[][..]),
            Key::Insert => (72, &[][..]),
            Key::Delete => (68, &[][..]),
            Key::PageDown => (73, &[][..]),
            Key::PageUp => (74, &[][..]),
            Key::Function(n @ 1..=12) => (120 + n as i32, &[][..]),
            Key::Function(_) => return SmallVec::new(),
        };
        let modifiers = u16::from(input.modifiers.shift)
            | (u16::from(input.modifiers.control) << 1)
            | (u16::from(input.modifiers.alt) << 2);
        // The terminal is authoritative for application cursor/keypad modes. Kitty stays disabled.
        let kitty_disabled = 0u8;
        let event = self.event.as_ptr();
        let encoder = self.encoder.as_ptr();
        let mut written = 0;
        let mut buffer = [0u8; 128];
        let mut overflow = Vec::new();
        let result = unsafe {
            if ghostty_key_encoder_set_from_terminal(encoder, self.terminal.as_ptr()) != 0
                || ghostty_key_encoder_set(
                    encoder,
                    OPT_KITTY_FLAGS,
                    (&kitty_disabled as *const u8).cast(),
                ) != 0
                || ghostty_key_event_set_action(event, 1) != 0
                || ghostty_key_event_set_key(event, key) != 0
                || ghostty_key_event_set_mods(event, modifiers) != 0
                || ghostty_key_event_set_utf8(
                    event,
                    GhosttyString {
                        ptr: text.as_ptr(),
                        len: text.len(),
                    },
                ) != 0
            {
                -1
            } else {
                let result = ghostty_key_encoder_encode(
                    encoder,
                    event,
                    buffer.as_mut_ptr(),
                    buffer.len(),
                    &mut written,
                );
                if result == -3 {
                    overflow.resize(written, 0);
                    let status = ghostty_key_encoder_encode(
                        encoder,
                        event,
                        overflow.as_mut_ptr(),
                        overflow.len(),
                        &mut written,
                    );
                    if status != 0 {
                        overflow.clear();
                    } else {
                        overflow.truncate(written);
                    }
                }
                result
            }
        };
        // Clear borrowed key text before returning to avoid retaining the caller's buffer.
        unsafe {
            ghostty_key_event_set_utf8(
                event,
                GhosttyString {
                    ptr: std::ptr::null(),
                    len: 0,
                },
            )
        };
        let mut encoded = SmallVec::new();
        if result == 0 {
            encoded.extend_from_slice(&buffer[..written]);
        } else if result == -3 {
            encoded.extend_from_slice(&overflow);
        }
        encoded
    }

    pub fn encode_paste(&mut self, text: &str) -> Result<Bytes, PasteError> {
        let mut mode = TerminalModeConfig {
            mode: 2004,
            value: false,
        };
        let bracketed = unsafe {
            ghostty_terminal_get(
                self.terminal.as_ptr(),
                DATA_MODE,
                (&mut mode as *mut TerminalModeConfig).cast(),
            )
        } == 0
            && mode.value;
        let mode = if bracketed {
            TermMode::BRACKETED_PASTE
        } else {
            TermMode::empty()
        };
        crate::input::InputEncoder::paste(text, mode)
    }

    pub fn cell(&mut self, x: u16, y: u16) -> Result<String, &'static str> {
        let (mut state, mut iterator, mut cells) = (
            std::ptr::null_mut(),
            std::ptr::null_mut(),
            std::ptr::null_mut(),
        );
        // SAFETY: out-pointers are valid, and each successful handle is released below.
        unsafe {
            if ghostty_render_state_new(std::ptr::null(), &mut state) != 0 {
                return Err("render state allocation failed");
            }
            let result = (|| {
                if ghostty_render_state_update(state, self.terminal.as_ptr()) != 0 {
                    return Err("render state update failed");
                }
                if ghostty_render_state_row_iterator_new(std::ptr::null(), &mut iterator) != 0 {
                    return Err("row iterator allocation failed");
                }
                if ghostty_render_state_get(
                    state,
                    4,
                    (&mut iterator as *mut *mut RowIteratorOpaque).cast(),
                ) != 0
                {
                    return Err("row iterator lookup failed");
                }
                if ghostty_render_state_row_cells_new(std::ptr::null(), &mut cells) != 0 {
                    return Err("row cells allocation failed");
                }
                for _ in 0..=y {
                    if !ghostty_render_state_row_iterator_next(iterator) {
                        return Err("cell row outside terminal");
                    }
                }
                if ghostty_render_state_row_get(
                    iterator,
                    3,
                    (&mut cells as *mut *mut RowCellsOpaque).cast(),
                ) != 0
                {
                    return Err("cell row lookup failed");
                }
                if ghostty_render_state_row_cells_select(cells, x) != 0 {
                    return Err("cell column outside terminal");
                }
                let mut bytes = [0u8; 32];
                let mut buffer = GhosttyBuffer {
                    ptr: bytes.as_mut_ptr(),
                    cap: bytes.len(),
                    len: 0,
                };
                if ghostty_render_state_row_cells_get(
                    cells,
                    CELL_TEXT_UTF8,
                    (&mut buffer as *mut GhosttyBuffer).cast(),
                ) != 0
                {
                    return Err("cell text lookup failed");
                }
                Ok(String::from_utf8_lossy(&bytes[..buffer.len]).into_owned())
            })();
            if !cells.is_null() {
                ghostty_render_state_row_cells_free(cells);
            }
            if !iterator.is_null() {
                ghostty_render_state_row_iterator_free(iterator);
            }
            ghostty_render_state_free(state);
            result
        }
    }
}

impl Drop for GhosttyTerminal {
    fn drop(&mut self) {
        // SAFETY: this owner uniquely owns the handle, and Drop runs once.
        unsafe {
            ghostty_key_event_free(self.event.as_ptr());
            ghostty_key_encoder_free(self.encoder.as_ptr());
            ghostty_terminal_free(self.terminal.as_ptr());
        }
    }
}
