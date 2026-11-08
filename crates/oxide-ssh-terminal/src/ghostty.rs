use crate::model::{
    CellRenderStyle, GhosttySnapshotData, RgbColor, SnapshotCellData, TerminalCursor,
    TerminalCursorShape, TerminalPoint, TerminalSize, TerminalSnapshot,
};
use std::ptr::NonNull;

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

fn rgb(color: GhosttyRgb) -> RgbColor {
    RgbColor::new(color.r, color.g, color.b)
}

unsafe extern "C" {
    fn ghostty_terminal_new(
        allocator: *const std::ffi::c_void,
        terminal: *mut *mut TerminalOpaque,
        cols: u16,
        rows: u16,
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
        allocator: *const std::ffi::c_void,
        state: *mut *mut RenderStateOpaque,
    ) -> i32;
    fn ghostty_render_state_free(state: *mut RenderStateOpaque);
    fn ghostty_render_state_update(
        state: *mut RenderStateOpaque,
        terminal: *mut TerminalOpaque,
    ) -> i32;
    fn ghostty_render_state_row_iterator_new(
        allocator: *const std::ffi::c_void,
        out: *mut *mut RowIteratorOpaque,
    ) -> i32;
    fn ghostty_render_state_row_iterator_free(iterator: *mut RowIteratorOpaque);
    fn ghostty_render_state_get(
        state: *mut RenderStateOpaque,
        data: i32,
        out: *mut std::ffi::c_void,
    ) -> i32;
    fn ghostty_render_state_row_iterator_next(iterator: *mut RowIteratorOpaque) -> bool;
    fn ghostty_render_state_row_get(
        iterator: *mut RowIteratorOpaque,
        data: i32,
        out: *mut std::ffi::c_void,
    ) -> i32;
    fn ghostty_render_state_row_cells_new(
        allocator: *const std::ffi::c_void,
        out: *mut *mut RowCellsOpaque,
    ) -> i32;
    fn ghostty_render_state_row_cells_free(cells: *mut RowCellsOpaque);
    fn ghostty_render_state_row_cells_select(cells: *mut RowCellsOpaque, x: u16) -> i32;
    fn ghostty_render_state_row_cells_get(
        cells: *mut RowCellsOpaque,
        data: i32,
        out: *mut std::ffi::c_void,
    ) -> i32;
    fn ghostty_cell_get(cell: u64, data: i32, out: *mut std::ffi::c_void) -> i32;
}

/// Safe owner for the pinned libghostty-vt terminal lifecycle.
pub struct GhosttyTerminal {
    terminal: NonNull<TerminalOpaque>,
    snapshot: GhosttySnapshotData,
    cell_width_px: u32,
    cell_height_px: u32,
}

impl GhosttyTerminal {
    pub fn new(cols: u16, rows: u16) -> Result<Self, &'static str> {
        let mut terminal = std::ptr::null_mut();
        // SAFETY: output pointer is valid and C allocator defaults when NULL.
        let result = unsafe { ghostty_terminal_new(std::ptr::null(), &mut terminal, cols, rows) };
        if result != 0 {
            return Err("libghostty-vt terminal creation failed");
        }
        let terminal = NonNull::new(terminal).ok_or("libghostty-vt returned a null terminal")?;
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
        unsafe { ghostty_terminal_free(self.terminal.as_ptr()) };
    }
}
