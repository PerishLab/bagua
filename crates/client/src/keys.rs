use crossterm::event::{KeyCode, KeyEvent, KeyEventKind, KeyModifiers};
use protocol::{Failure, Input};
use terminal::Key;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Gesture {
    Input(Input),
    Leader,
    Detach,
    Split(workspace::Axis),
    Window,
    Pane,
    Next,
    Acquire,
    Transfer,
    Fixed,
    Automatic,
    Close,
    Destroy,
    Select,
    Copy,
    Paste,
    Pan(Key),
    Help,
    Ignore,
}

pub fn translate(event: KeyEvent, leader: bool) -> Result<Gesture, Failure> {
    if event.kind == KeyEventKind::Release {
        return Ok(Gesture::Ignore);
    }
    if leader {
        return command(event.code);
    }
    if event.modifiers == KeyModifiers::CONTROL && event.code == KeyCode::Char('g') {
        return Ok(Gesture::Leader);
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
        return Err(Failure::Unavailable);
    }
    Ok(Gesture::Input(Input::Key(key)))
}

fn character(c: char, modifiers: KeyModifiers) -> Result<Gesture, Failure> {
    let input = match modifiers {
        KeyModifiers::CONTROL => Input::Key(Key::Control(c)),
        KeyModifiers::ALT => Input::Key(Key::Alt(c)),
        KeyModifiers::NONE | KeyModifiers::SHIFT if !c.is_control() => Input::Text(c.into()),
        _ => return Err(Failure::Unavailable),
    };
    Ok(Gesture::Input(input))
}

fn command(code: KeyCode) -> Result<Gesture, Failure> {
    Ok(match code {
        KeyCode::Char('d' | 'q') => Gesture::Detach,
        KeyCode::Char('v') => Gesture::Split(workspace::Axis::Columns),
        KeyCode::Char('h') => Gesture::Split(workspace::Axis::Rows),
        KeyCode::Char('n') => Gesture::Window,
        KeyCode::Tab => Gesture::Pane,
        KeyCode::Char('w') => Gesture::Next,
        KeyCode::Char('a') => Gesture::Acquire,
        KeyCode::Char('t') => Gesture::Transfer,
        KeyCode::Char('f') => Gesture::Fixed,
        KeyCode::Char('u') => Gesture::Automatic,
        KeyCode::Char('x') => Gesture::Close,
        KeyCode::Char('!') => Gesture::Destroy,
        KeyCode::Char('[') => Gesture::Select,
        KeyCode::Char('c') => Gesture::Copy,
        KeyCode::Char('p') => Gesture::Paste,
        KeyCode::Left => Gesture::Pan(Key::Left),
        KeyCode::Right => Gesture::Pan(Key::Right),
        KeyCode::Up => Gesture::Pan(Key::Up),
        KeyCode::Down => Gesture::Pan(Key::Down),
        KeyCode::Char('?') => Gesture::Help,
        KeyCode::Esc => Gesture::Ignore,
        _ => return Err(Failure::Unavailable),
    })
}
