use terminal::{CHUNK, Engine, Extent, Failure, Point, Revision};

#[test]
fn capacity() {
    for (width, height, history) in [
        (1, 1, 0),
        (513, 1, 0),
        (2, 257, 0),
        (2, 1, usize::MAX),
        (512, 256, 4096),
    ] {
        assert!(matches!(
            Engine::new(Extent::new(width, height).unwrap(), history),
            Err(Failure::Capacity)
        ));
    }
    assert!(Engine::new(Extent::new(2, 1).unwrap(), 0).is_ok());
    let mut terminal = Engine::new(Extent::new(80, 24).unwrap(), 4096).unwrap();
    assert_eq!(
        terminal.feed(&vec![b'x'; CHUNK + 1]),
        Err(Failure::Capacity)
    );
    assert_eq!(terminal.revision(), Revision::default());
    assert_eq!(terminal.snapshot().contents(), "");
    assert_eq!(
        terminal.resize(Extent::new(512, 256).unwrap()),
        Err(Failure::Capacity)
    );
    assert_eq!(terminal.extent(), Extent::new(80, 24).unwrap());
}

#[test]
fn history() {
    let mut terminal = Engine::new(Extent::new(4, 2).unwrap(), 2).unwrap();
    terminal.feed(b"0\r\n1\r\n2\r\n3\r\n4").unwrap();
    let snapshot = terminal.snapshot();
    assert_eq!(snapshot.contents(), "3\n4");
    assert_eq!(snapshot.history(), 2);
    let view = snapshot.view(2).unwrap();
    assert_eq!(view.contents(), "1\n2");
    assert_eq!(view.offset(), 2);
    assert_eq!(view.revision(), snapshot.revision());
    assert!(!view.cursor().visible);
    assert!(matches!(snapshot.view(3), Err(Failure::Position)));
    terminal.feed(b"\r\n5").unwrap();
    assert_eq!(snapshot.contents(), "3\n4");
    assert_eq!(snapshot.history(), 2);
    assert_eq!(terminal.snapshot().history(), 2);
}

#[test]
fn reset() {
    let mut terminal = Engine::new(Extent::new(4, 2).unwrap(), 2).unwrap();
    terminal.feed(b"0\r\n1\r\n2\x1b[?2004h").unwrap();
    let before = terminal.snapshot();
    assert_eq!(before.history(), 1);
    terminal.feed(b"\x1b[3J").unwrap();
    assert_eq!(terminal.snapshot().history(), 0);
    assert_eq!(terminal.snapshot().contents(), before.contents());
    terminal.feed(b"\x1bc").unwrap();
    assert_eq!(terminal.snapshot().contents(), "");
    assert!(!terminal.modes().paste);
    assert!(before.modes().paste);
    assert_eq!(before.history(), 1);
}

#[test]
fn resizing() {
    let mut terminal = Engine::new(Extent::new(4, 2).unwrap(), 2).unwrap();
    terminal.feed(b"ABCD").unwrap();
    let before = terminal.snapshot();
    assert!(before.cursor().pending);
    let narrow = Extent::new(2, 2).unwrap();
    let revision = terminal.resize(narrow).unwrap();
    let after = terminal.snapshot();
    assert_eq!(after.contents(), "AB");
    assert_eq!(after.extent(), narrow);
    assert_eq!(after.revision(), revision);
    assert_eq!(before.contents(), "ABCD");
    assert_eq!(before.extent(), Extent::new(4, 2).unwrap());
    assert_eq!(terminal.resize(narrow), Ok(revision));
    assert_eq!(terminal.feed(b"").unwrap().revision, revision);
}

#[test]
fn oversized() {
    let mut terminal = Engine::new(Extent::new(8, 2).unwrap(), 0).unwrap();
    terminal.feed(b"\x1b]99;").unwrap();
    for _ in 0..32 {
        terminal.feed(&[b'x'; CHUNK]).unwrap();
    }
    let report = terminal.feed(b"\x07OK").unwrap();
    assert_eq!(report.unsupported, 1);
    assert_eq!(terminal.snapshot().contents(), "OK");
    let marks = format!("e{}", "\u{301}".repeat(1000));
    terminal.feed(marks.as_bytes()).unwrap();
    let cell = terminal.snapshot().cell(Point { x: 2, y: 0 }).unwrap();
    assert!(cell.text.starts_with('e'));
    assert!(cell.text.len() <= 22);
}

#[test]
fn malformed() {
    let mut terminal = Engine::new(Extent::new(8, 2).unwrap(), 0).unwrap();
    let report = terminal
        .feed(b"\xffA\x1b[999999999999999999999999999mB")
        .unwrap();
    assert!(report.unsupported > 0);
    assert!(terminal.snapshot().contents().contains('A'));
    assert!(terminal.snapshot().contents().contains('B'));
    terminal.feed(b"\x1bPignored\x1b\\C").unwrap();
    assert!(terminal.snapshot().contents().ends_with('C'));
}

#[test]
fn queries() {
    let mut terminal = Engine::new(Extent::new(80, 24).unwrap(), 0).unwrap();
    assert!(terminal.feed(b"\x1b[6").unwrap().replies.is_empty());
    let report = terminal.feed(b"n\x1b[5n\x1b[18t\x07\x1bg").unwrap();
    assert_eq!(report.replies, b"\x1b[1;1R\x1b[0n\x1b[8;24;80t");
    assert_eq!(report.bells, 2);
    assert_eq!(terminal.feed(b"").unwrap().bells, 0);
    assert!(terminal.feed(b"\x1b[?6n").unwrap().replies.is_empty());
    assert!(terminal.feed(b"\x1b[5;6n").unwrap().replies.is_empty());
    let flood = b"\x1b[6n".repeat(CHUNK / 4);
    let report = terminal.feed(&flood).unwrap();
    assert_eq!(report.replies.len(), CHUNK / 4 * 6);
    assert!(report.replies.len() <= CHUNK * 16);
}
