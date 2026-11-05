use oxide_ssh_terminal::{
    Key, KeyInput, MAX_PASTE_BYTES, PasteError, TerminalEngine, TerminalSize,
};

fn engine() -> TerminalEngine {
    TerminalEngine::new(TerminalSize {
        columns: 80,
        rows: 24,
        pixel_width: 800,
        pixel_height: 480,
    })
    .unwrap()
}

#[test]
fn shift_tab_encodes_backwards_tab() {
    let engine = engine();
    assert_eq!(engine.encode_key(KeyInput::new(Key::Tab)).as_slice(), b"\t");
    assert_eq!(
        engine
            .encode_key(KeyInput::new(Key::Tab).shift())
            .as_slice(),
        b"\x1b[Z"
    );
}

#[test]
fn fixed_keys_and_function_keys_use_standard_sequences() {
    let engine = engine();
    let cases = [
        (Key::Enter, b"\r".as_slice()),
        (Key::Backspace, b"\x7f"),
        (Key::Tab, b"\t"),
        (Key::Escape, b"\x1b"),
        (Key::Insert, b"\x1b[2~"),
        (Key::Delete, b"\x1b[3~"),
        (Key::PageUp, b"\x1b[5~"),
        (Key::PageDown, b"\x1b[6~"),
        (Key::Function(1), b"\x1bOP"),
        (Key::Function(4), b"\x1bOS"),
        (Key::Function(5), b"\x1b[15~"),
        (Key::Function(12), b"\x1b[24~"),
    ];

    for (key, expected) in cases {
        assert_eq!(
            engine.encode_key(KeyInput::new(key)).as_slice(),
            expected,
            "{key:?}"
        );
    }
}

#[test]
fn arrow_keys_follow_application_cursor_mode() {
    let mut engine = engine();
    let cases = [
        (Key::ArrowUp, b"\x1b[A".as_slice(), b"\x1bOA".as_slice()),
        (Key::ArrowDown, b"\x1b[B", b"\x1bOB"),
        (Key::ArrowRight, b"\x1b[C", b"\x1bOC"),
        (Key::ArrowLeft, b"\x1b[D", b"\x1bOD"),
        (Key::Home, b"\x1b[H", b"\x1bOH"),
        (Key::End, b"\x1b[F", b"\x1bOF"),
    ];

    for (key, normal, _) in cases {
        assert_eq!(engine.encode_key(KeyInput::new(key)).as_slice(), normal);
    }
    engine.process_output(b"\x1b[?1h");
    for (key, _, application) in cases {
        assert_eq!(
            engine.encode_key(KeyInput::new(key)).as_slice(),
            application
        );
    }
}

#[test]
fn text_control_and_alt_are_encoded_without_loss() {
    let engine = engine();
    assert_eq!(
        engine.encode_key(KeyInput::text("你a")).as_slice(),
        "你a".as_bytes()
    );

    for (text, expected) in [
        ("a", 0x01),
        ("Z", 0x1a),
        ("[", 0x1b),
        ("\\", 0x1c),
        ("]", 0x1d),
        ("^", 0x1e),
        ("_", 0x1f),
    ] {
        assert_eq!(
            engine.encode_key(KeyInput::text(text).control()).as_slice(),
            &[expected]
        );
    }

    assert_eq!(
        engine.encode_key(KeyInput::text("x").alt()).as_slice(),
        b"\x1bx"
    );
    assert!(engine.encode_key(KeyInput::text("你").control()).is_empty());
}

#[test]
fn paste_filters_escape_and_honors_bracketed_mode() {
    let mut engine = engine();
    let source = "one\r\ntwo\rthree\0\x1bfour";
    assert_eq!(
        engine.encode_paste(source).unwrap(),
        bytes::Bytes::from_static(b"one\rtwo\rthreefour")
    );

    engine.process_output(b"\x1b[?2004h");
    assert_eq!(
        engine.encode_paste(source).unwrap(),
        bytes::Bytes::from_static(b"\x1b[200~one\ntwo\nthreefour\x1b[201~")
    );
}

#[test]
fn paste_limit_is_inclusive() {
    let engine = engine();
    let accepted = "x".repeat(MAX_PASTE_BYTES);
    assert!(engine.encode_paste(&accepted).is_ok());

    let rejected = "x".repeat(MAX_PASTE_BYTES + 1);
    assert_eq!(engine.encode_paste(&rejected), Err(PasteError::TooLarge));
}
