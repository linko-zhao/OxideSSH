use crate::model::{
    CellRenderStyle, GhosttySnapshotData, RgbColor, SnapshotCellData, TerminalColors,
    TerminalCursor, TerminalCursorShape, TerminalPoint, TerminalSize, TerminalSnapshot,
};
use bytes::Bytes;
use parking_lot::Mutex;
use smallvec::SmallVec;
use std::{cell::RefCell, ffi::c_void, ptr::NonNull};

use crate::{CellSide, Key, KeyInput, PasteError, TerminalAction};

const OPT_USERDATA: i32 = 1;
const OPT_WRITE_PTY: i32 = 2;
const OPT_BELL: i32 = 3;
const OPT_CLIPBOARD_WRITE: i32 = 27;
const OPT_COLOR_FOREGROUND: i32 = 12;
const OPT_COLOR_BACKGROUND: i32 = 13;
const OPT_COLOR_CURSOR: i32 = 14;
const OPT_COLOR_PALETTE: i32 = 15;
const OPT_SCROLLBACK_MAX_LINES: i32 = 29;

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
struct GhosttyCodepointBuffer {
    ptr: *mut u32,
    cap: usize,
    len: usize,
}

#[repr(C)]
#[derive(Clone, Copy, Default, PartialEq)]
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
fn ghostty_color(color: RgbColor) -> GhosttyRgb {
    GhosttyRgb {
        r: color.red,
        g: color.green,
        b: color.blue,
    }
}
fn xterm_component(index: usize) -> u8 {
    if index == 0 {
        0
    } else {
        (index as u8) * 40 + 55
    }
}
fn color_palette(colors: TerminalColors) -> [GhosttyRgb; 256] {
    let mut palette = [GhosttyRgb::default(); 256];
    for (index, color) in colors.ansi.iter().copied().enumerate() {
        palette[index] = ghostty_color(color);
    }
    for (index, color) in palette.iter_mut().enumerate().take(232).skip(16) {
        let value = index - 16;
        *color = GhosttyRgb {
            r: xterm_component(value / 36),
            g: xterm_component((value / 6) % 6),
            b: xterm_component(value % 6),
        };
    }
    for (index, color) in palette.iter_mut().enumerate().skip(232) {
        let gray = (8 + (index - 232) * 10) as u8;
        *color = GhosttyRgb {
            r: gray,
            g: gray,
            b: gray,
        };
    }
    palette
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

fn control_byte(text: &str) -> Option<u8> {
    match text.as_bytes() {
        [byte @ b'a'..=b'z'] => Some(byte - b'a' + 1),
        [byte @ b'A'..=b'Z'] => Some(byte - b'A' + 1),
        [b'['] => Some(0x1b),
        [b'\\'] => Some(0x1c),
        [b']'] => Some(0x1d),
        [b'^'] => Some(0x1e),
        [b'_'] => Some(0x1f),
        _ => None,
    }
}
pub(crate) struct GhosttyTerminal {
    terminal: NonNull<TerminalOpaque>,
    snapshot: RefCell<GhosttySnapshotData>,
    cell_width_px: u32,
    cell_height_px: u32,
    scrollback_lines: usize,
    pending: Box<Mutex<Vec<TerminalAction>>>,
    encoder: NonNull<KeyEncoderOpaque>,
    event: NonNull<KeyEventOpaque>,
    selection: Option<(TerminalPoint, TerminalPoint)>,
    selection_cells: std::collections::BTreeMap<(i32, usize), (char, Vec<char>)>,
    remote_cursor_style: bool,
    remote_cursor_blink: bool,
}

impl GhosttyTerminal {
    pub fn new(
        cols: u16,
        rows: u16,
        scrollback_lines: usize,
        default_colors: TerminalColors,
    ) -> Result<Self, &'static str> {
        let mut raw = std::ptr::null_mut();
        // SAFETY: output pointer is valid and C allocator defaults when NULL.
        let result = unsafe { ghostty_terminal_new(std::ptr::null(), &mut raw, cols, rows) };
        if result != 0 {
            return Err("libghostty-vt terminal creation failed");
        }
        let terminal = NonNull::new(raw).ok_or("libghostty-vt returned a null terminal")?;
        if unsafe {
            ghostty_terminal_set(
                terminal.as_ptr(),
                OPT_SCROLLBACK_MAX_LINES,
                (&scrollback_lines as *const usize).cast(),
            )
        } != 0
        {
            unsafe { ghostty_terminal_free(terminal.as_ptr()) };
            return Err("libghostty-vt scrollback configuration failed");
        }
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
        let mut terminal = Self {
            terminal,
            snapshot: RefCell::new(GhosttySnapshotData {
                size: TerminalSize {
                    columns: usize::from(cols),
                    rows: usize::from(rows),
                    pixel_width: 0,
                    pixel_height: 0,
                },
                cells: Vec::new(),
                history_size: 0,
                display_offset: 0,
                alternate_screen: false,
                cursor: TerminalCursor {
                    point: TerminalPoint { line: 0, column: 0 },
                    shape: TerminalCursorShape::Block,
                },
            }),
            cell_width_px: 0,
            cell_height_px: 0,
            scrollback_lines,
            pending,
            encoder,
            event,
            selection: None,
            selection_cells: std::collections::BTreeMap::new(),
            remote_cursor_style: false,
            remote_cursor_blink: false,
        };
        terminal.configure_colors(default_colors)?;
        Ok(terminal)
    }

    pub fn write(&mut self, bytes: &[u8]) {
        // SAFETY: live handle and bytes are valid for the call duration.
        unsafe { ghostty_terminal_vt_write(self.terminal.as_ptr(), bytes.as_ptr(), bytes.len()) };
    }

    pub fn set_default_colors(&mut self, colors: TerminalColors) {
        self.configure_colors(colors)
            .expect("valid Ghostty color options");
    }
    fn configure_colors(&mut self, colors: TerminalColors) -> Result<(), &'static str> {
        let foreground = ghostty_color(colors.foreground);
        let background = ghostty_color(colors.background);
        let cursor = ghostty_color(colors.cursor);
        let palette = color_palette(colors);
        for (option, value) in [
            (
                OPT_COLOR_FOREGROUND,
                (&foreground as *const GhosttyRgb).cast(),
            ),
            (
                OPT_COLOR_BACKGROUND,
                (&background as *const GhosttyRgb).cast(),
            ),
            (OPT_COLOR_CURSOR, (&cursor as *const GhosttyRgb).cast()),
            (OPT_COLOR_PALETTE, palette.as_ptr().cast()),
        ] {
            if unsafe { ghostty_terminal_set(self.terminal.as_ptr(), option, value) } != 0 {
                return Err("libghostty-vt color configuration failed");
            }
        }
        Ok(())
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
        let offset = self
            .snapshot()
            .map(|snapshot| snapshot.display_offset())
            .unwrap_or(0);
        let point = TerminalPoint {
            line: row as i32 - i32::try_from(offset).unwrap_or(i32::MAX),
            column,
        };
        self.selection = Some((point, point));
        self.selection_cells.clear();
        self.update_selection(row, column, _side);
    }

    pub fn update_selection(&mut self, row: usize, column: usize, _side: CellSide) {
        let offset = self
            .snapshot()
            .map(|snapshot| snapshot.display_offset())
            .unwrap_or(0);
        if let Some((_, end)) = &mut self.selection {
            *end = TerminalPoint {
                line: row as i32 - i32::try_from(offset).unwrap_or(i32::MAX),
                column,
            };
        }
        self.cache_visible_selection();
    }

    fn cache_visible_selection(&mut self) {
        let Some((start, end)) = self.selection else {
            return;
        };
        let (first, last) = if (start.line, start.column) <= (end.line, end.column) {
            (start, end)
        } else {
            (end, start)
        };
        let Ok(snapshot) = self.snapshot() else {
            return;
        };
        let offset = i32::try_from(snapshot.display_offset()).unwrap_or(i32::MAX);
        let visible: Vec<_> = snapshot
            .cells()
            .filter_map(|cell| {
                let point = (cell.point.line - offset, cell.point.column);
                ((point.0, point.1) >= (first.line, first.column)
                    && (point.0, point.1) <= (last.line, last.column))
                    .then(|| {
                        (
                            point,
                            cell.character,
                            cell.zerowidth().unwrap_or(&[]).to_vec(),
                        )
                    })
            })
            .collect();
        drop(snapshot);
        for (point, character, zerowidth) in visible {
            self.selection_cells.insert(point, (character, zerowidth));
        }
    }

    pub fn clear_selection(&mut self) {
        self.selection = None;
        self.selection_cells.clear();
    }

    pub fn selected_text(&self) -> Option<String> {
        let (start, end) = self.selection?;
        let (first, last) = if (start.line, start.column) <= (end.line, end.column) {
            (start, end)
        } else {
            (end, start)
        };
        let mut text = String::new();
        let mut previous_line = None;
        for (&(line, column), (character, zerowidth)) in &self.selection_cells {
            if (line, column) < (first.line, first.column)
                || (line, column) > (last.line, last.column)
            {
                continue;
            }
            if previous_line.is_some_and(|previous| previous != line) {
                text.push('\n');
            }
            text.push(*character);
            text.extend(zerowidth);
            previous_line = Some(line);
        }
        Some(text.trim_end().to_owned())
    }

    pub fn snapshot(&self) -> Result<TerminalSnapshot<'_>, &'static str> {
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
                    history_size: 0,
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
                    scrollbar
                        .total
                        .saturating_sub(scrollbar.offset.saturating_add(scrollbar.len)),
                )
                .unwrap_or(usize::MAX);
                // shortcut: Ghostty retains page-granularity extra lines; remove this cap if exact-line pruning is added.
                data.display_offset = data.display_offset.min(self.scrollback_lines);
                data.history_size = usize::try_from(scrollbar.total.saturating_sub(scrollbar.len))
                    .unwrap_or(usize::MAX)
                    .min(self.scrollback_lines);
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
                                    line: i32::from(y)
                                        - i32::try_from(data.display_offset).unwrap_or(i32::MAX),
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
                *self.snapshot.borrow_mut() = data;
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
        Ok(TerminalSnapshot::ghostty(self.snapshot.borrow()))
    }

    /// Process VT output and return queued effects only after the parser returns.
    pub fn process_output(&mut self, bytes: &[u8]) -> SmallVec<[TerminalAction; 4]> {
        if bytes
            .windows(5)
            .any(|window| window == b"\x1b[5 q" || window == b"\x1b[1 q" || window == b"\x1b[3 q")
        {
            self.remote_cursor_style = true;
            self.remote_cursor_blink = bytes
                .windows(5)
                .any(|window| window == b"\x1b[1 q" || window == b"\x1b[3 q");
        }
        self.write(bytes);
        let mut actions = SmallVec::new();
        actions.extend(std::mem::take(&mut *self.pending.lock()));
        if !bytes.is_empty() && !actions.contains(&TerminalAction::Wakeup) {
            actions.push(TerminalAction::Wakeup);
        }
        actions
    }
    pub fn encode_key(&self, input: KeyInput<'_>) -> SmallVec<[u8; 16]> {
        if let Key::Text(text) = input.key
            && (input.modifiers.control || input.modifiers.alt)
        {
            let mut encoded = SmallVec::new();
            if input.modifiers.alt {
                encoded.push(0x1b);
            }
            if input.modifiers.control {
                if let Some(byte) = control_byte(text) {
                    encoded.push(byte);
                } else {
                    encoded.clear();
                }
            } else {
                encoded.extend_from_slice(text.as_bytes());
            }
            return encoded;
        }
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

    pub fn encode_paste(&self, text: &str) -> Result<Bytes, PasteError> {
        let mut mode = TerminalModeConfig {
            mode: 2004,
            value: false,
        };
        let bracketed = (unsafe {
            ghostty_terminal_get(
                self.terminal.as_ptr(),
                DATA_MODE,
                (&mut mode as *mut TerminalModeConfig).cast(),
            )
        }) == 0
            && mode.value;
        crate::input::InputEncoder::paste(text, bracketed)
    }
    pub fn cursor_blinking(&self) -> bool {
        let mut mode = TerminalModeConfig {
            mode: 12,
            value: false,
        };
        (unsafe {
            ghostty_terminal_get(
                self.terminal.as_ptr(),
                DATA_MODE,
                (&mut mode as *mut TerminalModeConfig).cast(),
            )
        }) == 0
            && mode.value
    }
    pub fn has_remote_cursor_style(&self) -> bool {
        self.remote_cursor_style
    }
    pub fn has_remote_cursor_blink(&self) -> bool {
        self.remote_cursor_blink
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
