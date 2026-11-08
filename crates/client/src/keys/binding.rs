use super::{Gesture, Keys};
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use protocol::Failure;
use std::collections::BTreeMap;

#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct Binding {
    code: KeyCode,
    modifiers: KeyModifiers,
    pub action: Option<Gesture>,
}

impl Keys {
    pub fn bind(
        mut self,
        prefix: &BTreeMap<String, String>,
        direct: &BTreeMap<String, String>,
    ) -> Result<Self, Failure> {
        if prefix.len() + direct.len() > 128 {
            return Err(Failure::Capacity);
        }
        self.bindings = compile(prefix)?;
        self.direct = compile(direct)?;
        if self.bindings.iter().chain(&self.direct).any(|binding| {
            binding.code == KeyCode::Char(self.prefix) && binding.modifiers == KeyModifiers::CONTROL
        }) {
            return Err(Failure::Payload);
        }
        Ok(self)
    }

    pub(super) fn mapping(&self, event: KeyEvent, leader: bool) -> Option<&Binding> {
        let bindings = if leader { &self.bindings } else { &self.direct };
        bindings
            .iter()
            .find(|binding| binding.code == event.code && binding.modifiers == event.modifiers)
    }
}

fn compile(values: &BTreeMap<String, String>) -> Result<Vec<Binding>, Failure> {
    let mut bindings = Vec::new();
    for (key, value) in values {
        let (code, modifiers) = parse(key)?;
        if bindings
            .iter()
            .any(|binding: &Binding| binding.code == code && binding.modifiers == modifiers)
        {
            return Err(Failure::Payload);
        }
        bindings.push(Binding {
            code,
            modifiers,
            action: action(value)?,
        });
    }
    Ok(bindings)
}

fn parse(text: &str) -> Result<(KeyCode, KeyModifiers), Failure> {
    if text.len() > 64 {
        return Err(Failure::Capacity);
    }
    let mut text = text;
    let mut modifiers = KeyModifiers::NONE;
    loop {
        let found = [
            ("Ctrl-", KeyModifiers::CONTROL),
            ("Alt-", KeyModifiers::ALT),
            ("Shift-", KeyModifiers::SHIFT),
        ]
        .into_iter()
        .find(|(prefix, _)| text.starts_with(prefix));
        let Some((prefix, flag)) = found else { break };
        if modifiers.contains(flag) {
            return Err(Failure::Payload);
        }
        modifiers |= flag;
        text = &text[prefix.len()..];
    }
    let mut code = code(text)?;
    if modifiers == KeyModifiers::CONTROL
        && let KeyCode::Char(c) = code
    {
        code = KeyCode::Char(c.to_ascii_lowercase());
    }
    if code == KeyCode::Tab && modifiers.contains(KeyModifiers::SHIFT) {
        code = KeyCode::BackTab;
    }
    Ok((code, modifiers))
}

fn code(text: &str) -> Result<KeyCode, Failure> {
    if let Some(number) = text.strip_prefix('F').filter(|number| !number.is_empty()) {
        let number = number.parse::<u8>().map_err(|_| Failure::Payload)?;
        return (1..=12)
            .contains(&number)
            .then_some(KeyCode::F(number))
            .ok_or(Failure::Payload);
    }
    let value = match text {
        "Space" => KeyCode::Char(' '),
        "Enter" => KeyCode::Enter,
        "Tab" => KeyCode::Tab,
        "Backspace" => KeyCode::Backspace,
        "Escape" => KeyCode::Esc,
        "Up" => KeyCode::Up,
        "Down" => KeyCode::Down,
        "Left" => KeyCode::Left,
        "Right" => KeyCode::Right,
        "Home" => KeyCode::Home,
        "End" => KeyCode::End,
        "PageUp" => KeyCode::PageUp,
        "PageDown" => KeyCode::PageDown,
        "Insert" => KeyCode::Insert,
        "Delete" => KeyCode::Delete,
        _ => {
            let mut chars = text.chars();
            let value = chars.next().ok_or(Failure::Payload)?;
            if chars.next().is_some() || value.is_control() {
                return Err(Failure::Payload);
            }
            KeyCode::Char(value)
        }
    };
    Ok(value)
}

fn action(text: &str) -> Result<Option<Gesture>, Failure> {
    let gesture = match text {
        "pass" => return Ok(None),
        "ignore" => Gesture::Ignore,
        "detach" => Gesture::Detach,
        "split-columns" => Gesture::Split(workspace::Axis::Columns),
        "split-rows" => Gesture::Split(workspace::Axis::Rows),
        "window" => Gesture::Window,
        "pane" => Gesture::Pane,
        "next-window" => Gesture::Next,
        "acquire" => Gesture::Acquire,
        "transfer" => Gesture::Transfer,
        "fixed" => Gesture::Fixed,
        "automatic" => Gesture::Automatic,
        "close-pane" => Gesture::Close,
        "close-session" => Gesture::Destroy,
        "select" => Gesture::Select,
        "copy" => Gesture::Copy,
        "paste" => Gesture::Paste,
        "help" => Gesture::Help,
        "reload" => Gesture::Reload,
        "page-up" => Gesture::History(-1),
        "page-down" => Gesture::History(1),
        "live" => Gesture::Live,
        "left" => Gesture::Pan(terminal::Key::Left),
        "right" => Gesture::Pan(terminal::Key::Right),
        "up" => Gesture::Pan(terminal::Key::Up),
        "down" => Gesture::Pan(terminal::Key::Down),
        _ if text.starts_with("window-") => {
            let index = text[7..].parse::<u8>().map_err(|_| Failure::Payload)?;
            if !(1..=32).contains(&index) {
                return Err(Failure::Payload);
            }
            Gesture::Index(index)
        }
        _ => return Err(Failure::Payload),
    };
    Ok(Some(gesture))
}
