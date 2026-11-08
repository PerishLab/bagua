use super::*;
pub(super) fn input(event: KeyEvent) -> Result<Gesture, Failure> {
    if event.kind == KeyEventKind::Release {
        return Ok(Gesture::Ignore);
    }
    let key = match event.code {
        KeyCode::Char(c) => return character(c, event.modifiers),
        KeyCode::BackTab => Key::Backtab,
        KeyCode::Up => Key::Up,
        KeyCode::Down => Key::Down,
        KeyCode::Left => Key::Left,
        KeyCode::Right => Key::Right,
        KeyCode::Home => Key::Home,
        KeyCode::End => Key::End,
        KeyCode::PageUp => Key::Previous,
        KeyCode::PageDown => Key::Next,
        KeyCode::Insert => Key::Insert,
        KeyCode::Delete => Key::Delete,
        KeyCode::Enter => Key::Enter,
        KeyCode::Tab => Key::Tab,
        KeyCode::Backspace => Key::Backspace,
        KeyCode::Esc => Key::Escape,
        KeyCode::F(number) if (1..=12).contains(&number) => Key::Function(number),
        _ => return Err(Failure::Unavailable),
    };
    let permitted = match event.code {
        KeyCode::BackTab => KeyModifiers::SHIFT,
        _ => KeyModifiers::NONE,
    };
    if event.modifiers != permitted {
        return modified(event);
    }
    Ok(Gesture::Input(Input::Key(key)))
}

fn character(c: char, modifiers: KeyModifiers) -> Result<Gesture, Failure> {
    let input = match modifiers {
        KeyModifiers::CONTROL => Input::Key(Key::Control(c)),
        KeyModifiers::ALT => Input::Key(Key::Alt(c)),
        KeyModifiers::NONE if !c.is_control() => Input::Text(c.into()),
        _ => return modified(KeyEvent::new(KeyCode::Char(c), modifiers)),
    };
    Ok(Gesture::Input(input))
}

fn modified(event: KeyEvent) -> Result<Gesture, Failure> {
    if event.modifiers.bits() & !7 != 0 {
        return Err(Failure::Unavailable);
    }
    let code = match event.code {
        KeyCode::Char(c) if !c.is_control() => u32::from(c),
        KeyCode::Enter => 13,
        KeyCode::Tab | KeyCode::BackTab => 9,
        KeyCode::Backspace => 127,
        KeyCode::Esc => 27,
        KeyCode::Insert => 57348,
        KeyCode::Delete => 57349,
        KeyCode::Left => 57350,
        KeyCode::Right => 57351,
        KeyCode::Up => 57352,
        KeyCode::Down => 57353,
        KeyCode::PageUp => 57354,
        KeyCode::PageDown => 57355,
        KeyCode::Home => 57356,
        KeyCode::End => 57357,
        KeyCode::F(number) if (1..=12).contains(&number) => 57363 + u32::from(number),
        _ => return Err(Failure::Unavailable),
    };
    Ok(Gesture::Input(Input::Key(Key::Modified {
        code,
        modifiers: terminal::Modifiers {
            shift: event.modifiers.contains(KeyModifiers::SHIFT),
            alt: event.modifiers.contains(KeyModifiers::ALT),
            control: event.modifiers.contains(KeyModifiers::CONTROL),
        },
    })))
}
