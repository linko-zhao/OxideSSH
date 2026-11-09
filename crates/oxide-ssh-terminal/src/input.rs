use bytes::Bytes;

pub const MAX_PASTE_BYTES: usize = 1024 * 1024;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Key<'a> {
    Text(&'a str),
    Enter,
    Backspace,
    Tab,
    Escape,
    ArrowUp,
    ArrowDown,
    ArrowRight,
    ArrowLeft,
    Home,
    End,
    Insert,
    Delete,
    PageUp,
    PageDown,
    Function(u8),
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct Modifiers {
    pub control: bool,
    pub alt: bool,
    pub shift: bool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct KeyInput<'a> {
    pub key: Key<'a>,
    pub modifiers: Modifiers,
}

impl<'a> KeyInput<'a> {
    pub const fn new(key: Key<'a>) -> Self {
        Self {
            key,
            modifiers: Modifiers {
                control: false,
                alt: false,
                shift: false,
            },
        }
    }
    pub const fn text(text: &'a str) -> Self {
        Self::new(Key::Text(text))
    }
    pub const fn control(mut self) -> Self {
        self.modifiers.control = true;
        self
    }
    pub const fn alt(mut self) -> Self {
        self.modifiers.alt = true;
        self
    }
    pub const fn shift(mut self) -> Self {
        self.modifiers.shift = true;
        self
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MouseAction {
    Press,
    Release,
    Motion,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MouseButton {
    Left,
    Right,
    Middle,
    WheelUp,
    WheelDown,
    WheelLeft,
    WheelRight,
}

#[derive(Clone, Copy, Debug)]
pub struct MouseInput {
    pub action: MouseAction,
    pub button: Option<MouseButton>,
    pub modifiers: Modifiers,
    pub x: f32,
    pub y: f32,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PasteError {
    TooLarge,
}

impl std::fmt::Display for PasteError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("paste exceeds 1 MiB")
    }
}
impl std::error::Error for PasteError {}

pub(crate) struct InputEncoder;

impl InputEncoder {
    pub fn paste(text: &str, bracketed: bool) -> Result<Bytes, PasteError> {
        if text.len() > MAX_PASTE_BYTES {
            return Err(PasteError::TooLarge);
        }
        let extra = if bracketed { 12 } else { 0 };
        let mut output = Vec::with_capacity(text.len() + extra);
        if bracketed {
            output.extend_from_slice(b"\x1b[200~");
        }
        let bytes = text.as_bytes();
        let mut index = 0;
        while index < bytes.len() {
            match bytes[index] {
                0 | 0x1b => {}
                b'\r' => {
                    output.push(if bracketed { b'\n' } else { b'\r' });
                    if bytes.get(index + 1) == Some(&b'\n') {
                        index += 1;
                    }
                }
                b'\n' => output.push(if bracketed { b'\n' } else { b'\r' }),
                byte => output.push(byte),
            }
            index += 1;
        }
        if bracketed {
            output.extend_from_slice(b"\x1b[201~");
        }
        Ok(Bytes::from(output))
    }
}
