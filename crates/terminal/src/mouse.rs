use crate::{Encoding, Extent, Failure, Modes, Tracking};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Point {
    pub x: u16,
    pub y: u16,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Modifiers {
    pub shift: bool,
    pub alt: bool,
    pub control: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Button {
    Left,
    Middle,
    Right,
}

impl Button {
    fn code(self) -> u8 {
        match self {
            Self::Left => 0,
            Self::Middle => 1,
            Self::Right => 2,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Direction {
    Up,
    Down,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Action {
    Press(Button),
    Release(Button),
    Move(Option<Button>),
    Wheel(Direction),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Mouse {
    pub position: Point,
    pub action: Action,
    pub modifiers: Modifiers,
}

impl Mouse {
    pub(crate) fn encode(self, modes: Modes, extent: Extent) -> Result<Vec<u8>, Failure> {
        if self.position.x >= extent.width() || self.position.y >= extent.height() {
            return Err(Failure::Position);
        }
        let mut code = self.action.code(modes.tracking)?;
        code |= u8::from(self.modifiers.shift) * 4;
        code |= u8::from(self.modifiers.alt) * 8;
        code |= u8::from(self.modifiers.control) * 16;
        let x = self.position.x + 1;
        let y = self.position.y + 1;
        match modes.encoding {
            Encoding::Sgr => {
                let suffix = if matches!(self.action, Action::Release(_)) {
                    'm'
                } else {
                    'M'
                };
                Ok(format!("\x1b[<{code};{x};{y}{suffix}").into_bytes())
            }
            Encoding::Legacy => self.legacy(code),
            Encoding::Utf8 => Err(Failure::Unsupported),
        }
    }

    fn legacy(self, mut code: u8) -> Result<Vec<u8>, Failure> {
        if self.position.x >= 223 || self.position.y >= 223 {
            return Err(Failure::Position);
        }
        if matches!(self.action, Action::Release(_)) {
            code = (code & !3) | 3;
        }
        Ok(vec![
            0x1b,
            b'[',
            b'M',
            code + 32,
            self.position.x as u8 + 33,
            self.position.y as u8 + 33,
        ])
    }
}

impl Action {
    fn code(self, tracking: Tracking) -> Result<u8, Failure> {
        if tracking == Tracking::Off {
            return Err(Failure::Inactive);
        }
        match self {
            Self::Press(button) => Ok(button.code()),
            Self::Wheel(Direction::Up) => Ok(64),
            Self::Wheel(Direction::Down) => Ok(65),
            Self::Release(_) if tracking == Tracking::Press => Err(Failure::Inactive),
            Self::Release(button) => Ok(button.code()),
            Self::Move(button) => motion(button, tracking),
        }
    }
}

fn motion(button: Option<Button>, tracking: Tracking) -> Result<u8, Failure> {
    let enabled = tracking == Tracking::Motion || (tracking == Tracking::Drag && button.is_some());
    if !enabled {
        return Err(Failure::Inactive);
    }
    Ok(32 | button.map(Button::code).unwrap_or(3))
}
