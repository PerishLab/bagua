use terminal::{Color, Engine, Extent, Point};

fn engine() -> Engine {
    Engine::new(Extent::new(8, 3).unwrap(), 8).unwrap()
}

#[test]
fn fragments() {
    let bytes = "e\u{301}中\x1b[31mX\x1b[0m\r\nY\x1b[?1h".as_bytes();
    let mut reference = engine();
    reference.feed(bytes).unwrap();
    let expected = reference.snapshot();
    for boundary in 0..=bytes.len() {
        let mut actual = engine();
        actual.feed(&bytes[..boundary]).unwrap();
        actual.feed(&bytes[boundary..]).unwrap();
        let actual = actual.snapshot();
        assert_eq!(actual.contents(), expected.contents());
        assert_eq!(actual.cursor(), expected.cursor());
        assert_eq!(actual.modes(), expected.modes());
        for x in 0..8 {
            assert_eq!(
                actual.cell(Point { x, y: 0 }),
                expected.cell(Point { x, y: 0 })
            );
        }
    }
    assert_eq!(expected.cell(Point::default()).unwrap().text, "e\u{301}");
    assert_eq!(expected.cell(Point { x: 1, y: 0 }).unwrap().width, 2);
    assert_eq!(expected.cell(Point { x: 2, y: 0 }).unwrap().width, 0);
}

#[test]
fn editing() {
    let mut terminal = engine();
    terminal.feed(b"abcdef\rXY\x1b[K").unwrap();
    assert_eq!(terminal.snapshot().contents(), "XY");
    terminal
        .feed(b"\x1b[2;3H\x1b[1;38;2;1;2;3;48;5;9mZ")
        .unwrap();
    let snapshot = terminal.snapshot();
    let cell = snapshot.cell(Point { x: 2, y: 1 }).unwrap();
    assert_eq!(cell.text, "Z");
    assert_eq!(cell.style.foreground, Color::Rgb(1, 2, 3));
    assert_eq!(cell.style.background, Color::Indexed(9));
    assert!(cell.style.bold);
    assert_eq!(snapshot.cursor().position, Point { x: 3, y: 1 });
    terminal.feed(b"\x1b7\x1b[1;1Hq\x1b8!").unwrap();
    assert_eq!(
        terminal.snapshot().cell(Point { x: 3, y: 1 }).unwrap().text,
        "!"
    );
}

#[test]
fn insertion() {
    let mut terminal = engine();
    terminal.feed(b"abcd\r\x1b[2C\x1b[@").unwrap();
    assert_eq!(terminal.snapshot().contents(), "ab cd");
    terminal.feed(b"\x1b[P").unwrap();
    assert_eq!(terminal.snapshot().contents(), "abcd");
    terminal.feed(b"\x1b[2X").unwrap();
    assert_eq!(terminal.snapshot().contents(), "ab");
    terminal.feed(b"\x1b[2J").unwrap();
    assert_eq!(terminal.snapshot().contents(), "");
    assert_eq!(terminal.snapshot().cursor().position, Point { x: 2, y: 0 });
}

#[test]
fn region() {
    let mut terminal = Engine::new(Extent::new(4, 4).unwrap(), 4).unwrap();
    terminal.feed(b"AAAA\r\nBBBB\r\nCCCC\r\nDDDD").unwrap();
    terminal.feed(b"\x1b[2;3r\x1b[3;1H\n").unwrap();
    let snapshot = terminal.snapshot();
    assert_eq!(snapshot.cell(Point { x: 0, y: 0 }).unwrap().text, "A");
    assert_eq!(snapshot.cell(Point { x: 0, y: 1 }).unwrap().text, "C");
    assert_eq!(snapshot.cell(Point { x: 0, y: 2 }).unwrap().text, "");
    assert_eq!(snapshot.cell(Point { x: 0, y: 3 }).unwrap().text, "D");
    assert_eq!(snapshot.history(), 0);
}

#[test]
fn wide() {
    let mut terminal = Engine::new(Extent::new(4, 2).unwrap(), 0).unwrap();
    terminal.feed("abc中".as_bytes()).unwrap();
    let snapshot = terminal.snapshot();
    assert!(!snapshot.wrapped(0));
    assert_eq!(snapshot.contents(), "abc\n中");
    assert_eq!(snapshot.cell(Point { x: 0, y: 1 }).unwrap().width, 2);
    assert_eq!(snapshot.cell(Point { x: 1, y: 1 }).unwrap().width, 0);
    terminal.feed(b"\x1b[2;2Hx").unwrap();
    let snapshot = terminal.snapshot();
    assert_eq!(snapshot.cell(Point { x: 0, y: 1 }).unwrap().width, 1);
    assert_eq!(snapshot.cell(Point { x: 1, y: 1 }).unwrap().text, "x");
    assert!(snapshot.cell(Point { x: 4, y: 0 }).is_none());
}

#[test]
fn wrapping() {
    let mut terminal = Engine::new(Extent::new(4, 2).unwrap(), 0).unwrap();
    terminal.feed(b"abcde").unwrap();
    let snapshot = terminal.snapshot();
    assert!(snapshot.wrapped(0));
    assert_eq!(snapshot.contents(), "abcde");
    let report = terminal.feed(b"\x1b[?7l").unwrap();
    assert_eq!(report.unsupported, 1);
}

#[test]
fn alternate() {
    let mut terminal = engine();
    terminal.feed(b"primary\x1b[?1049hALT").unwrap();
    let alternate = terminal.snapshot();
    assert!(alternate.modes().alternate);
    assert_eq!(alternate.contents(), "ALT");
    terminal.feed(b"\x1b[?1049l").unwrap();
    assert!(!terminal.modes().alternate);
    assert_eq!(terminal.snapshot().contents(), "primary");
    assert_eq!(alternate.contents(), "ALT");
}

#[test]
fn styles() {
    let mut terminal = engine();
    terminal.feed(b"\x1b[2;3;4;7mX\x1b[0mY\x1b[?25l").unwrap();
    let snapshot = terminal.snapshot();
    let styled = snapshot.cell(Point::default()).unwrap().style;
    assert!(styled.dim && styled.italic && styled.underline);
    assert!(styled.inverse);
    let plain = snapshot.cell(Point { x: 1, y: 0 }).unwrap().style;
    assert!(!plain.dim && !plain.italic && !plain.underline);
    assert!(!plain.inverse);
    assert!(!snapshot.cursor().visible);
}
