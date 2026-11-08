use bytes::Bytes;
use parking_lot::Mutex;
use smallvec::SmallVec;
use std::{ffi::c_void, ptr::NonNull};

use crate::TerminalAction;

const OPT_USERDATA: i32 = 1;
const OPT_WRITE_PTY: i32 = 2;
const OPT_BELL: i32 = 3;
const OPT_CLIPBOARD_WRITE: i32 = 27;

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
pub struct GhosttyTerminal(NonNull<TerminalOpaque>, Box<Mutex<Vec<TerminalAction>>>);

impl GhosttyTerminal {
    pub fn new(cols: u16, rows: u16) -> Result<Self, &'static str> {
        let mut raw = std::ptr::null_mut();
        // SAFETY: output pointer is valid and C allocator defaults when NULL.
        let result = unsafe { ghostty_terminal_new(std::ptr::null(), &mut raw, cols, rows) };
        if result != 0 {
            return Err("libghostty-vt terminal creation failed");
        }
        let terminal = NonNull::new(raw).ok_or("libghostty-vt returned a null terminal")?;
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
        Ok(Self(terminal, pending))
    }

    pub fn write(&mut self, bytes: &[u8]) {
        // SAFETY: live handle and bytes are valid for the call duration.
        unsafe { ghostty_terminal_vt_write(self.0.as_ptr(), bytes.as_ptr(), bytes.len()) };
    }

    /// Process VT output and return queued effects only after the parser returns.
    pub fn process_output(&mut self, bytes: &[u8]) -> SmallVec<[TerminalAction; 4]> {
        self.write(bytes);
        let mut actions = SmallVec::new();
        actions.extend(std::mem::take(&mut *self.1.lock()));
        if !bytes.is_empty() && !actions.contains(&TerminalAction::Wakeup) {
            actions.push(TerminalAction::Wakeup);
        }
        actions
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
