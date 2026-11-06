use terminal::{CHUNK, Engine, Extent, Failure, Input, Key};

fn engine() -> Engine {
    Engine::new(Extent::new(80, 24).unwrap(), 0).unwrap()
}

#[test]
fn cursor() {
    let mut terminal = engine();
    assert_eq!(terminal.encode(Input::Key(Key::Up)).unwrap(), b"\x1b[A");
    terminal.feed(b"\x1b[?1h").unwrap();
    assert_eq!(terminal.encode(Input::Key(Key::Up)).unwrap(), b"\x1bOA");
    assert_eq!(terminal.encode(Input::Key(Key::Home)).unwrap(), b"\x1bOH");
    assert_eq!(terminal.encode(Input::Key(Key::End)).unwrap(), b"\x1bOF");
    terminal.feed(b"\x1b[?1l").unwrap();
    assert_eq!(terminal.encode(Input::Key(Key::Right)).unwrap(), b"\x1b[C");
}

#[test]
fn paste() {
    let mut terminal = engine();
    assert_eq!(terminal.encode(Input::Paste("a\nb")).unwrap(), b"a\nb");
    terminal.feed(b"\x1b[?2004h").unwrap();
    assert_eq!(
        terminal.encode(Input::Paste("中\nb")).unwrap(),
        "\x1b[200~中\nb\x1b[201~".as_bytes()
    );
    assert_eq!(
        terminal.encode(Input::Paste("\x1b[201~")),
        Err(Failure::Input)
    );
    assert_eq!(terminal.encode(Input::Paste("\0")), Err(Failure::Input));
    terminal.feed(b"\x1b[?2004l").unwrap();
    assert_eq!(terminal.encode(Input::Paste("text")).unwrap(), b"text");
}

#[test]
fn text() {
    let terminal = engine();
    assert_eq!(
        terminal.encode(Input::Text("e\u{301}中")).unwrap(),
        "e\u{301}中".as_bytes()
    );
    assert_eq!(terminal.encode(Input::Text("\x1b")), Err(Failure::Input));
    assert_eq!(terminal.encode(Input::Text("\n")), Err(Failure::Input));
    assert_eq!(
        terminal.encode(Input::Text(&"x".repeat(CHUNK + 1))),
        Err(Failure::Capacity)
    );
    assert_eq!(terminal.encode(Input::Key(Key::Control('c'))).unwrap(), [3]);
    assert_eq!(terminal.encode(Input::Key(Key::Enter)).unwrap(), b"\r");
    assert_eq!(
        terminal.encode(Input::Key(Key::Control('中'))),
        Err(Failure::Unsupported)
    );
}

#[test]
fn navigation() {
    let terminal = engine();
    assert_eq!(
        terminal.encode(Input::Key(Key::Previous)).unwrap(),
        b"\x1b[5~"
    );
    assert_eq!(terminal.encode(Input::Key(Key::Next)).unwrap(), b"\x1b[6~");
    assert_eq!(
        terminal.encode(Input::Key(Key::Function(1))).unwrap(),
        b"\x1bOP"
    );
    assert_eq!(
        terminal.encode(Input::Key(Key::Function(12))).unwrap(),
        b"\x1b[24~"
    );
    assert_eq!(
        terminal.encode(Input::Key(Key::Function(13))),
        Err(Failure::Unsupported)
    );
}

#[test]
fn negotiation() {
    let mut terminal = engine();
    let report = terminal.feed(b"\x1b[>1u\x1b[?u").unwrap();
    assert_eq!(report.unsupported, 2);
    assert!(report.replies.is_empty());
    assert_eq!(terminal.encode(Input::Key(Key::Up)).unwrap(), b"\x1b[A");
}
