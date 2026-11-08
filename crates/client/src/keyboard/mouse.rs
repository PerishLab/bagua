use crossterm::event::{Event, KeyModifiers, MouseButton, MouseEvent, MouseEventKind};
use protocol::Failure;
pub(super) fn parse(body: &[u8], finalizer: u8) -> Result<Event, Failure> {
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
