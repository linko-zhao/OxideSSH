use alacritty_terminal::term::TermMode;
use bytes::Bytes;
use smallvec::SmallVec;

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
    pub fn encode(input: KeyInput<'_>, mode: TermMode) -> SmallVec<[u8; 16]> {
        let mut encoded = SmallVec::new();
        if input.modifiers.alt {
            encoded.push(0x1b);
        }

        match input.key {
            Key::Text(text) if input.modifiers.control => {
                if let Some(byte) = control_byte(text) {
                    encoded.push(byte);
                } else {
                    encoded.clear();
                }
            }
            Key::Text(text) => encoded.extend_from_slice(text.as_bytes()),
            key => {
                if let Some(sequence) = key_sequence(key, input.modifiers.shift, mode) {
                    encoded.extend_from_slice(sequence);
                } else {
                    encoded.clear();
                }
            }
        }

        encoded
    }

    pub fn paste(text: &str, mode: TermMode) -> Result<Bytes, PasteError> {
        if text.len() > MAX_PASTE_BYTES {
            return Err(PasteError::TooLarge);
        }

        let bracketed = mode.contains(TermMode::BRACKETED_PASTE);
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

fn control_byte(text: &str) -> Option<u8> {
    let bytes = text.as_bytes();
    if bytes.len() != 1 {
        return None;
    }

    match bytes[0] {
        byte @ b'a'..=b'z' => Some(byte - b'a' + 1),
        byte @ b'A'..=b'Z' => Some(byte - b'A' + 1),
        b'[' => Some(0x1b),
        b'\\' => Some(0x1c),
        b']' => Some(0x1d),
        b'^' => Some(0x1e),
        b'_' => Some(0x1f),
        _ => None,
    }
}

fn key_sequence(key: Key<'_>, shift: bool, mode: TermMode) -> Option<&'static [u8]> {
    let application_cursor = mode.contains(TermMode::APP_CURSOR);
    match key {
        Key::Text(_) => None,
        Key::Enter => Some(b"\r"),
        Key::Backspace => Some(b"\x7f"),
        Key::Tab if shift => Some(b"\x1b[Z"),
        Key::Tab => Some(b"\t"),
        Key::Escape => Some(b"\x1b"),
        Key::ArrowUp if application_cursor => Some(b"\x1bOA"),
        Key::ArrowUp => Some(b"\x1b[A"),
        Key::ArrowDown if application_cursor => Some(b"\x1bOB"),
        Key::ArrowDown => Some(b"\x1b[B"),
        Key::ArrowRight if application_cursor => Some(b"\x1bOC"),
        Key::ArrowRight => Some(b"\x1b[C"),
        Key::ArrowLeft if application_cursor => Some(b"\x1bOD"),
        Key::ArrowLeft => Some(b"\x1b[D"),
        Key::Home if application_cursor => Some(b"\x1bOH"),
        Key::Home => Some(b"\x1b[H"),
        Key::End if application_cursor => Some(b"\x1bOF"),
        Key::End => Some(b"\x1b[F"),
        Key::Insert => Some(b"\x1b[2~"),
        Key::Delete => Some(b"\x1b[3~"),
        Key::PageUp => Some(b"\x1b[5~"),
        Key::PageDown => Some(b"\x1b[6~"),
        Key::Function(1) => Some(b"\x1bOP"),
        Key::Function(2) => Some(b"\x1bOQ"),
        Key::Function(3) => Some(b"\x1bOR"),
        Key::Function(4) => Some(b"\x1bOS"),
        Key::Function(5) => Some(b"\x1b[15~"),
        Key::Function(6) => Some(b"\x1b[17~"),
        Key::Function(7) => Some(b"\x1b[18~"),
        Key::Function(8) => Some(b"\x1b[19~"),
        Key::Function(9) => Some(b"\x1b[20~"),
        Key::Function(10) => Some(b"\x1b[21~"),
        Key::Function(11) => Some(b"\x1b[23~"),
        Key::Function(12) => Some(b"\x1b[24~"),
        Key::Function(_) => None,
    }
}
