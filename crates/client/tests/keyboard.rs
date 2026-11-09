use client::Keyboard;
use crossterm::event::{Event, KeyCode, KeyModifiers};
use protocol::Failure;
use std::time::{Duration, Instant};

#[test]
fn delay() {
    let now = Instant::now();
    let mut keyboard = Keyboard::new(Duration::from_millis(10));
    keyboard.push(b"\x1b").unwrap();
    assert!(keyboard.next(now).is_none());
    assert!(keyboard.next(now + Duration::from_millis(9)).is_none());
    assert!(
        matches!(keyboard.next(now + Duration::from_millis(10)), Some(Ok(Event::Key(key))) if key.code == KeyCode::Esc)
    );
    keyboard.delay(Duration::ZERO);
    keyboard.push(b"\x1b").unwrap();
    assert!(matches!(keyboard.next(now), Some(Ok(Event::Key(key))) if key.code == KeyCode::Esc));
}

#[test]
fn streaming() {
    let mut keyboard = Keyboard::default();
    let mut events = Vec::new();
    let bytes = "é中\x1b[A\x1b[24~\x1b[<4;2;3M\x1b[200~paste\r\n\x1b[201~\x07".as_bytes();
    let now = Instant::now();
    for byte in bytes {
        keyboard.push(&[*byte]).unwrap();
        while let Some(event) = keyboard.next(now) {
            events.push(event.unwrap());
        }
    }
    assert!(matches!(&events[0],Event::Key(key) if key.code == KeyCode::Char('é')));
    assert!(matches!(&events[1],Event::Key(key) if key.code == KeyCode::Char('中')));
    assert!(matches!(&events[2],Event::Key(key) if key.code == KeyCode::Up));
    assert!(matches!(&events[3],Event::Key(key) if key.code == KeyCode::F(12)));
    assert!(
        matches!(&events[4],Event::Mouse(mouse) if mouse.column == 1 && mouse.row == 2 && mouse.modifiers == KeyModifiers::SHIFT)
    );
    assert_eq!(events[5], Event::Paste("paste\r\n".into()));
    assert!(
        matches!(&events[6],Event::Key(key) if client::translate(*key,false) == Ok(client::Gesture::Leader))
    );
}

#[test]
fn paste() {
    let mut keyboard = Keyboard::default();
    let now = Instant::now();
    keyboard.push(b"\x1b[200~").unwrap();
    assert!(keyboard.next(now).is_none());
    for _ in 0..64 {
        keyboard.push(&[b'x'; 4096]).unwrap();
        assert!(keyboard.next(now).is_none());
        assert_eq!(keyboard.room(), 8192);
    }
    keyboard.push(b"\x1b[201~a").unwrap();
    assert_eq!(keyboard.next(now), Some(Err(Failure::Capacity)));
    assert!(
        matches!(keyboard.next(now),Some(Ok(Event::Key(key))) if key.code == KeyCode::Char('a'))
    );
}

#[test]
fn escape() {
    let mut keyboard = Keyboard::default();
    let now = Instant::now();
    keyboard.push(b"\x1b").unwrap();
    assert!(keyboard.next(now).is_none());
    assert!(
        matches!(keyboard.next(now+Duration::from_millis(51)),Some(Ok(Event::Key(key))) if key.code == KeyCode::Esc)
    );
    keyboard.push(b"\x1bb").unwrap();
    assert!(
        matches!(keyboard.next(now),Some(Ok(Event::Key(key))) if key.code == KeyCode::Char('b') && key.modifiers == KeyModifiers::ALT)
    );
    assert_eq!(keyboard.push(&[0; 8193]), Err(Failure::Capacity));
}

#[test]
fn recovery() {
    let mut keyboard = Keyboard::default();
    let now = Instant::now();
    keyboard.push(b"\x1b\xc3\xa9\xe4\xb8").unwrap();
    assert!(
        matches!(keyboard.next(now),Some(Ok(Event::Key(key))) if key.code == KeyCode::Char('é') && key.modifiers == KeyModifiers::ALT)
    );
    assert!(keyboard.next(now).is_none());
    keyboard.push(b"\xad").unwrap();
    assert!(
        matches!(keyboard.next(now),Some(Ok(Event::Key(key))) if key.code == KeyCode::Char('中'))
    );
    keyboard.push(b"\x1b[").unwrap();
    keyboard.push(&[b'1'; 100]).unwrap();
    assert_eq!(keyboard.next(now), Some(Err(Failure::Unavailable)));
    assert!(keyboard.next(now).is_none());
    keyboard.push(b"Az").unwrap();
    assert!(
        matches!(keyboard.next(now),Some(Ok(Event::Key(key))) if key.code == KeyCode::Char('z'))
    );
}

#[test]
fn modifiers() {
    let now = Instant::now();
    let mut keyboard = Keyboard::default();
    for byte in b"\x1b[13;2u" {
        keyboard.push(&[*byte]).unwrap();
        if *byte != b'u' {
            assert!(keyboard.next(now).is_none());
        }
    }
    let Some(Ok(Event::Key(key))) = keyboard.next(now) else {
        panic!("missing modified key")
    };
    assert_eq!(key.code, KeyCode::Enter);
    assert_eq!(key.modifiers, KeyModifiers::SHIFT);
    assert!(matches!(
        client::translate(key, false),
        Ok(client::Gesture::Input(protocol::Input::Key(
            terminal::Key::Modified { code: 13, .. }
        )))
    ));
    for bytes in [b"\x1b[97;6u".as_slice(), b"\x1b[1;5A", b"\x1b[5;2~"] {
        keyboard.push(bytes).unwrap();
        assert!(matches!(keyboard.next(now), Some(Ok(Event::Key(_)))));
    }
    keyboard.push(b"\x1b[97;1:3u").unwrap();
    let Some(Ok(Event::Key(key))) = keyboard.next(now) else {
        panic!("missing release")
    };
    assert_eq!(
        client::translate(key, false).unwrap(),
        client::Gesture::Ignore
    );
    for bytes in [
        b"\x1b[13;9u".as_slice(),
        b"\x1b[55296;2u",
        b"\x1b[97;2;65u",
        b"\x1b[0u",
        b"\x1b[13;2:9u",
    ] {
        keyboard.push(bytes).unwrap();
        assert!(matches!(keyboard.next(now), Some(Err(_))));
    }
    keyboard.push(b"z").unwrap();
    assert!(
        matches!(keyboard.next(now), Some(Ok(Event::Key(key))) if key.code == KeyCode::Char('z'))
    );
}

#[test]
fn cancellation() {
    let now = Instant::now();
    for bytes in ["中".as_bytes(), b"\x1b[A", b"\x1b[200~never\x1b[201~"] {
        for split in 1..bytes.len() {
            let mut keyboard = Keyboard::default();
            keyboard.push(&bytes[..split]).unwrap();
            keyboard.cancel(now);
            keyboard.push(&bytes[split..]).unwrap();
            while let Some(event) = keyboard.next(now) {
                assert!(event.is_err());
            }
            keyboard.push(b"z").unwrap();
            assert!(matches!(keyboard.next(now), Some(Ok(Event::Key(key)))
                if key.code == KeyCode::Char('z')));
        }
    }
}
