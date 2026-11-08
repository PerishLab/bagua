use crossterm::event::{Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers};
use protocol::Failure;

pub(super) fn parse(body: &[u8]) -> Result<Event, Failure> {
    let text = std::str::from_utf8(body).map_err(|_| Failure::Payload)?;
    let mut fields = text.split(';');
    let value = fields
        .next()
        .ok_or(Failure::Payload)?
        .parse::<u32>()
        .map_err(|_| Failure::Payload)?;
    let detail = fields.next().unwrap_or("1");
    if fields.next().is_some() {
        return Err(Failure::Unavailable);
    }
    let mut detail = detail.split(':');
    let modifiers = modifiers(detail.next().unwrap_or("1"))?;
    let kind = match detail.next().unwrap_or("1") {
        "1" => KeyEventKind::Press,
        "2" => KeyEventKind::Repeat,
        "3" => KeyEventKind::Release,
        _ => return Err(Failure::Unavailable),
    };
    if detail.next().is_some() {
        return Err(Failure::Unavailable);
    }
    let mut code = code(value)?;
    if code == KeyCode::Tab && modifiers.contains(KeyModifiers::SHIFT) {
        code = KeyCode::BackTab;
    }
    Ok(Event::Key(KeyEvent::new_with_kind(code, modifiers, kind)))
}

pub(super) fn modifiers(text: &str) -> Result<KeyModifiers, Failure> {
    let value = text.parse::<u8>().map_err(|_| Failure::Payload)?;
    if !(1..=8).contains(&value) {
        return Err(Failure::Unavailable);
    }
    let value = value - 1;
    let mut modifiers = KeyModifiers::NONE;
    for (mask, flag) in [
        (1, KeyModifiers::SHIFT),
        (2, KeyModifiers::ALT),
        (4, KeyModifiers::CONTROL),
    ] {
        if value & mask != 0 {
            modifiers |= flag;
        }
    }
    Ok(modifiers)
}

fn code(value: u32) -> Result<KeyCode, Failure> {
    Ok(match value {
        9 => KeyCode::Tab,
        13 => KeyCode::Enter,
        27 => KeyCode::Esc,
        127 => KeyCode::Backspace,
        57348 => KeyCode::Insert,
        57349 => KeyCode::Delete,
        57350 => KeyCode::Left,
        57351 => KeyCode::Right,
        57352 => KeyCode::Up,
        57353 => KeyCode::Down,
        57354 => KeyCode::PageUp,
        57355 => KeyCode::PageDown,
        57356 => KeyCode::Home,
        57357 => KeyCode::End,
        57364..=57375 => KeyCode::F((value - 57363) as u8),
        value => {
            let c = char::from_u32(value).ok_or(Failure::Payload)?;
            if c.is_control() || (0xe000..=0xf8ff).contains(&value) {
                return Err(Failure::Unavailable);
            }
            KeyCode::Char(c)
        }
    })
}
