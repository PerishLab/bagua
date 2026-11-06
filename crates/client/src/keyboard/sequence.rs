use crossterm::event::{
    Event, KeyCode, KeyEvent, KeyModifiers, MouseButton, MouseEvent, MouseEventKind,
};
use protocol::Failure;

pub(super) fn parse(bytes: &[u8]) -> Option<(usize, Result<Event, Failure>)> {
    let prefix = *bytes.get(1)?;
    if prefix != b'[' && prefix != b'O' {
        if prefix == 27 {
            return Some((
                1,
                Ok(Event::Key(KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE))),
            ));
        }
        let length = match prefix {
            0..=127 => 1,
            0xc2..=0xdf => 2,
            0xe0..=0xef => 3,
            0xf0..=0xf4 => 4,
            _ => return Some((2, Err(Failure::Payload))),
        };
        let value = bytes.get(1..1 + length)?;
        let character = match std::str::from_utf8(value) {
            Ok(text) => text.chars().next()?,
            Err(_) => return Some((1 + length, Err(Failure::Payload))),
        };
        if character.is_control() {
            return Some((1 + length, Err(Failure::Unavailable)));
        }
        return Some((
            1 + character.len_utf8(),
            Ok(Event::Key(KeyEvent::new(
                KeyCode::Char(character),
                KeyModifiers::ALT,
            ))),
        ));
    }
    let end = bytes
        .iter()
        .enumerate()
        .skip(2)
        .find(|(_, byte)| (0x40..=0x7e).contains(*byte))?
        .0;
    if bytes.starts_with(b"\x1b[M") {
        return (bytes.len() >= 6).then_some((6, Err(Failure::Unavailable)));
    }
    let body = &bytes[2..end];
    let finalizer = bytes[end];
    let event = if body.starts_with(b"<") {
        mouse(body, finalizer)
    } else {
        key(body, finalizer, prefix)
    };
    Some((end + 1, event))
}

fn key(body: &[u8], finalizer: u8, prefix: u8) -> Result<Event, Failure> {
    let mut modifiers = KeyModifiers::NONE;
    let code = match (body, finalizer) {
        (b"", b'A') => KeyCode::Up,
        (b"", b'B') => KeyCode::Down,
        (b"", b'C') => KeyCode::Right,
        (b"", b'D') => KeyCode::Left,
        (b"", b'H') => KeyCode::Home,
        (b"", b'F') => KeyCode::End,
        (b"", b'Z') => {
            modifiers = KeyModifiers::SHIFT;
            KeyCode::BackTab
        }
        (b"", b'P'..=b'S') if prefix == b'O' => KeyCode::F(finalizer - b'P' + 1),
        (_, b'~') => match body {
            b"1" | b"7" => KeyCode::Home,
            b"2" => KeyCode::Insert,
            b"3" => KeyCode::Delete,
            b"4" | b"8" => KeyCode::End,
            b"5" => KeyCode::PageUp,
            b"6" => KeyCode::PageDown,
            _ => function(body)?,
        },
        _ => return Err(Failure::Unavailable),
    };
    Ok(Event::Key(KeyEvent::new(code, modifiers)))
}

fn function(body: &[u8]) -> Result<KeyCode, Failure> {
    let value: u8 = std::str::from_utf8(body)
        .map_err(|_| Failure::Payload)?
        .parse()
        .map_err(|_| Failure::Payload)?;
    [11, 12, 13, 14, 15, 17, 18, 19, 20, 21, 23, 24]
        .iter()
        .position(|v| *v == value)
        .map(|index| KeyCode::F(index as u8 + 1))
        .ok_or(Failure::Unavailable)
}

fn mouse(body: &[u8], finalizer: u8) -> Result<Event, Failure> {
    let text = std::str::from_utf8(&body[1..]).map_err(|_| Failure::Payload)?;
    let values: Vec<u16> = text
        .split(';')
        .map(|p| p.parse().map_err(|_| Failure::Payload))
        .collect::<Result<_, _>>()?;
    let [code, column, row] = values.as_slice() else {
        return Err(Failure::Payload);
    };
    if *code > 127 || *column == 0 || *row == 0 {
        return Err(Failure::Payload);
    }
    let mut modifiers = KeyModifiers::NONE;
    for (mask, flag) in [
        (4, KeyModifiers::SHIFT),
        (8, KeyModifiers::ALT),
        (16, KeyModifiers::CONTROL),
    ] {
        if code & mask != 0 {
            modifiers |= flag;
        }
    }
    let button = match code & 3 {
        0 => MouseButton::Left,
        1 => MouseButton::Middle,
        _ => MouseButton::Right,
    };
    let release = match finalizer {
        b'M' => false,
        b'm' => true,
        _ => return Err(Failure::Payload),
    };
    let report = Report {
        wheel: code & 64 != 0,
        motion: code & 32 != 0,
        button: code & 3,
        release,
    };
    let kind = match report {
        Report {
            wheel: true,
            motion: false,
            button: 0,
            release: false,
        } => MouseEventKind::ScrollUp,
        Report {
            wheel: true,
            motion: false,
            button: 1,
            release: false,
        } => MouseEventKind::ScrollDown,
        Report {
            wheel: false,
            motion: true,
            button: 3,
            release: false,
        } => MouseEventKind::Moved,
        Report {
            wheel: false,
            motion: true,
            button: 0..=2,
            release: false,
        } => MouseEventKind::Drag(button),
        Report {
            wheel: false,
            motion: false,
            button: 0..=2,
            release: false,
        } => MouseEventKind::Down(button),
        Report {
            wheel: false,
            motion: false,
            button: 0..=2,
            release: true,
        } => MouseEventKind::Up(button),
        _ => return Err(Failure::Unavailable),
    };
    Ok(Event::Mouse(MouseEvent {
        kind,
        column: column - 1,
        row: row - 1,
        modifiers,
    }))
}

struct Report {
    wheel: bool,
    motion: bool,
    button: u16,
    release: bool,
}
