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

unsafe extern "C" {
    fn ghostty_terminal_new(
        allocator: *const std::ffi::c_void,
        terminal: *mut *mut TerminalOpaque,
        cols: u16,
        rows: u16,
    ) -> i32;
    fn ghostty_terminal_vt_write(terminal: *mut TerminalOpaque, data: *const u8, len: usize);
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
}

/// Safe owner for the pinned libghostty-vt terminal lifecycle.
pub struct GhosttyTerminal(NonNull<TerminalOpaque>);

impl GhosttyTerminal {
    pub fn new(cols: u16, rows: u16) -> Result<Self, &'static str> {
        let mut terminal = std::ptr::null_mut();
        // SAFETY: output pointer is valid and C allocator defaults when NULL.
        let result = unsafe { ghostty_terminal_new(std::ptr::null(), &mut terminal, cols, rows) };
        if result != 0 {
            return Err("libghostty-vt terminal creation failed");
        }
        NonNull::new(terminal)
            .map(Self)
            .ok_or("libghostty-vt returned a null terminal")
    }

    pub fn write(&mut self, bytes: &[u8]) {
        // SAFETY: live handle and bytes are valid for the call duration.
        unsafe { ghostty_terminal_vt_write(self.0.as_ptr(), bytes.as_ptr(), bytes.len()) };
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
                if ghostty_render_state_update(state, self.0.as_ptr()) != 0 {
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
                    9,
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
        unsafe { ghostty_terminal_free(self.0.as_ptr()) };
    }
}
