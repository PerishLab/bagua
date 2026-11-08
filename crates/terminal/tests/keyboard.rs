use terminal::{Engine, Extent, Failure, Input, Key, Modifiers};

fn engine() -> Engine {
    Engine::new(Extent::new(80, 24).unwrap(), 0).unwrap()
}

fn key(code: u32, shift: bool) -> Input<'static> {
    Input::Key(Key::Modified {
        code,
        modifiers: Modifiers {
            shift,
            ..Default::default()
        },
    })
}

#[test]
fn distinction() {
    let mut engine = engine();
    assert_eq!(engine.encode(key(13, true)), Err(Failure::Unsupported));
    assert_eq!(engine.encode(Input::Key(Key::Enter)).unwrap(), b"\r");
    for byte in b"\x1b[>1u" {
        assert_eq!(engine.feed(&[*byte]).unwrap().unsupported, 0);
    }
    assert_eq!(engine.encode(key(13, true)).unwrap(), b"\x1b[13;2u");
    assert_eq!(engine.encode(Input::Key(Key::Enter)).unwrap(), b"\r");
    assert_eq!(
        engine.encode(Input::Key(Key::Control('q'))).unwrap(),
        b"\x1b[113;5u"
    );
    assert_eq!(
        engine.encode(Input::Key(Key::Control('@'))).unwrap(),
        b"\x1b[64;5u"
    );
    assert_eq!(
        engine.encode(Input::Key(Key::Alt('x'))).unwrap(),
        b"\x1b[120;3u"
    );
    assert_eq!(
        engine.encode(Input::Key(Key::Escape)).unwrap(),
        b"\x1b[27;1u"
    );
    assert_eq!(
        engine.encode(Input::Key(Key::Backtab)).unwrap(),
        b"\x1b[9;2u"
    );
    assert_eq!(engine.encode(key(0x1b, true)).unwrap(), b"\x1b[27;2u");
    assert_eq!(engine.encode(key(0x9f, true)), Err(Failure::Unsupported));
    assert_eq!(engine.encode(key(0xd800, true)), Err(Failure::Unsupported));
    assert_eq!(engine.encode(key(57358, true)), Err(Failure::Unsupported));
    assert_eq!(engine.encode(Input::Text("中A")).unwrap(), "中A".as_bytes());
    engine.feed(b"\x1b[<u").unwrap();
    assert_eq!(engine.encode(key(13, true)), Err(Failure::Unsupported));
    assert_eq!(engine.encode(Input::Key(Key::Control('q'))).unwrap(), [17]);
    assert_eq!(engine.encode(key(57352, true)).unwrap(), b"\x1b[1;2A");
}

#[test]
fn lifetimes() {
    let mut engine = engine();
    let report = engine.feed(b"\x1b[?u\x1b[c\x1b[>1u\x1b[?u").unwrap();
    assert_eq!(report.unsupported, 0);
    assert_eq!(report.replies, b"\x1b[?0u\x1b[?1;2c\x1b[?1u");
    assert_eq!(engine.snapshot().modes().keyboard, 1);
    engine.feed(b"\x1b[?1049h").unwrap();
    assert_eq!(engine.modes().keyboard, 0);
    engine.feed(b"\x1b[>1u\x1b[>0u").unwrap();
    assert_eq!(engine.modes().keyboard, 0);
    engine.feed(b"\x1b[<u").unwrap();
    assert_eq!(engine.modes().keyboard, 1);
    engine.feed(b"\x1b[?1049l").unwrap();
    assert_eq!(engine.modes().keyboard, 1);
    engine.feed(b"\x1b[=1;3u").unwrap();
    assert_eq!(engine.modes().keyboard, 0);
    engine.feed(b"\x1b[=1;2u").unwrap();
    assert_eq!(engine.modes().keyboard, 1);
    let report = engine.feed(b"\x1b[>31u\x1b[=1;4u\x1b[?u").unwrap();
    assert_eq!(report.unsupported, 2);
    assert_eq!(report.replies, b"\x1b[?1u");
    for _ in 0..100 {
        engine.feed(b"\x1b[>1u").unwrap();
    }
    engine.feed(b"\x1b[<65535u").unwrap();
    assert_eq!(engine.modes().keyboard, 0);
    engine.feed(b"\x1bc\x1b[?1049h").unwrap();
    assert_eq!(engine.modes().keyboard, 0);
    engine.feed(b"\x1b[?1049l").unwrap();
    assert_eq!(engine.modes().keyboard, 0);
}
