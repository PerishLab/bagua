mod binding;
mod command;
mod input;
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
    Index(u8),
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
    Reload,
    History(i32),
    Live,
    Ignore,
}

pub fn translate(event: KeyEvent, leader: bool) -> Result<Gesture, Failure> {
    Keys::default().translate(event, leader)
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Keys {
    prefix: char,
    bindings: Vec<binding::Binding>,
    direct: Vec<binding::Binding>,
}

impl Default for Keys {
    fn default() -> Self {
        Self {
            prefix: 'g',
            bindings: Vec::new(),
            direct: Vec::new(),
        }
    }
}

impl Keys {
    pub fn new(prefix: char) -> Result<Self, Failure> {
        if !prefix.is_ascii_lowercase() || matches!(prefix, 'h' | 'i' | 'j' | 'm') {
            return Err(Failure::Payload);
        }
        Ok(Self {
            prefix,
            bindings: Vec::new(),
            direct: Vec::new(),
        })
    }

    pub fn translate(&self, mut event: KeyEvent, leader: bool) -> Result<Gesture, Failure> {
        if event.kind == KeyEventKind::Release {
            return Ok(Gesture::Ignore);
        }
        if event.modifiers == KeyModifiers::CONTROL
            && let KeyCode::Char(c) = event.code
        {
            event.code = KeyCode::Char(c.to_ascii_lowercase());
        }
        let prefix =
            event.modifiers == KeyModifiers::CONTROL && event.code == KeyCode::Char(self.prefix);
        if prefix {
            return Ok(if leader {
                Gesture::Input(Input::Key(Key::Control(self.prefix)))
            } else {
                Gesture::Leader
            });
        }
        if let Some(binding) = self.mapping(event, leader) {
            return binding
                .action
                .clone()
                .map(Ok)
                .unwrap_or_else(|| input::input(event));
        }
        if leader {
            if !matches!(event.modifiers, KeyModifiers::NONE | KeyModifiers::SHIFT) {
                return Err(Failure::Unavailable);
            }
            return command::command(event.code);
        }
        if event.modifiers == KeyModifiers::ALT
            && let KeyCode::Char(key @ '1'..='9') = event.code
        {
            return Ok(Gesture::Index(key as u8 - b'0'));
        }
        input::input(event)
    }
}
