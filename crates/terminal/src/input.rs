use crate::{CHUNK, Extent, Failure, Modes, Mouse};

#[derive(serde::Serialize, serde::Deserialize, Clone, Copy, Debug, PartialEq, Eq)]
pub enum Key {
    Up,
    Down,
    Left,
    Right,
    Home,
    End,
    Insert,
    Delete,
    Previous,
    Next,
    Enter,
    Tab,
    Backspace,
    Backtab,
    Alt(char),
    Escape,
    Function(u8),
    Control(char),
    Modified {
        code: u32,
        modifiers: crate::Modifiers,
    },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Input<'a> {
    Text(&'a str),
    Paste(&'a str),
    Key(Key),
    Mouse(Mouse),
}

impl Input<'_> {
    pub fn encode(self, modes: Modes, extent: Extent) -> Result<Vec<u8>, Failure> {
        match self {
            Self::Text(text) => {
                check(text, false)?;
                Ok(text.as_bytes().into())
            }
            Self::Paste(text) => {
                check(text, true)?;
                if modes.paste {
                    Ok(format!("\x1b[200~{text}\x1b[201~").into_bytes())
                } else {
                    Ok(text.as_bytes().into())
                }
            }
            Self::Key(key) => key.encode(modes),
            Self::Mouse(mouse) => mouse.encode(modes, extent),
        }
    }
}

fn check(text: &str, paste: bool) -> Result<(), Failure> {
    if text.len() > CHUNK {
        return Err(Failure::Capacity);
    }
    if text
        .chars()
        .any(|c| c.is_control() && !(paste && matches!(c, '\r' | '\n' | '\t')))
    {
        return Err(Failure::Input);
    }
    Ok(())
}

impl Key {
    fn encode(self, modes: Modes) -> Result<Vec<u8>, Failure> {
        if let Some(result) = extended(self, modes.keyboard) {
            return result;
        }
        let application = modes.cursor;
        let prefix = if application { "\x1bO" } else { "\x1b[" };
        match self {
            Self::Up => Ok(format!("{prefix}A").into_bytes()),
            Self::Down => Ok(format!("{prefix}B").into_bytes()),
            Self::Right => Ok(format!("{prefix}C").into_bytes()),
            Self::Left => Ok(format!("{prefix}D").into_bytes()),
            Self::Home => Ok(format!("{prefix}H").into_bytes()),
            Self::End => Ok(format!("{prefix}F").into_bytes()),
            Self::Insert => Ok(b"\x1b[2~".into()),
            Self::Delete => Ok(b"\x1b[3~".into()),
            Self::Previous => Ok(b"\x1b[5~".into()),
            Self::Next => Ok(b"\x1b[6~".into()),
            Self::Enter => Ok(b"\r".into()),
            Self::Tab => Ok(b"\t".into()),
            Self::Backtab => Ok(b"\x1b[Z".into()),
            Self::Alt(c) if !c.is_control() => Ok(format!("\x1b{c}").into_bytes()),
            Self::Alt(_) => Err(Failure::Input),
            Self::Backspace => Ok(b"\x7f".into()),
            Self::Escape => Ok(b"\x1b".into()),
            Self::Function(number) => function(number),
            Self::Control(c) => control(c),
            Self::Modified { .. } => Err(Failure::Unsupported),
        }
    }
}

fn extended(key: Key, flags: u8) -> Option<Result<Vec<u8>, Failure>> {
    let (code, modifiers) = match key {
        Key::Modified { code, modifiers } => (code, modifiers),
        Key::Control(c) if flags == 1 && !c.is_control() => (
            u32::from(c.to_ascii_lowercase()),
            crate::Modifiers {
                shift: false,
                alt: false,
                control: true,
            },
        ),
        Key::Alt(c) if flags == 1 && !c.is_control() => (
            u32::from(c),
            crate::Modifiers {
                shift: false,
                alt: true,
                control: false,
            },
        ),
        Key::Escape if flags == 1 => (27, crate::Modifiers::default()),
        Key::Backtab if flags == 1 => (
            9,
            crate::Modifiers {
                shift: true,
                alt: false,
                control: false,
            },
        ),
        _ => return None,
    };
    Some(modified(code, modifiers, flags))
}

fn modified(code: u32, modifiers: crate::Modifiers, flags: u8) -> Result<Vec<u8>, Failure> {
    if !valid(code) || flags > 1 {
        return Err(Failure::Unsupported);
    }
    let value = 1
        + u8::from(modifiers.shift)
        + 2 * u8::from(modifiers.alt)
        + 4 * u8::from(modifiers.control);
    if value == 1 && matches!(code, 13 | 9 | 127) {
        return Ok(vec![if code == 13 { b'\r' } else { code as u8 }]);
    }
    let navigation = match code {
        57348 => Some("2~"),
        57349 => Some("3~"),
        57350 => Some("1D"),
        57351 => Some("1C"),
        57352 => Some("1A"),
        57353 => Some("1B"),
        57354 => Some("5~"),
        57355 => Some("6~"),
        57356 => Some("1H"),
        57357 => Some("1F"),
        57364..=57367 => Some(["1P", "1Q", "1R", "1S"][(code - 57364) as usize]),
        57368..=57375 => {
            Some(["15~", "17~", "18~", "19~", "20~", "21~", "23~", "24~"][(code - 57368) as usize])
        }
        _ => None,
    };
    if let Some(sequence) = navigation {
        let (number, suffix) = sequence.split_at(sequence.len() - 1);
        return Ok(format!("\x1b[{number};{value}{suffix}").into_bytes());
    }
    if flags == 0 {
        return Err(Failure::Unsupported);
    }
    Ok(format!("\x1b[{code};{value}u").into_bytes())
}

fn valid(code: u32) -> bool {
    matches!(code, 9 | 13 | 27 | 127 | 57348..=57357 | 57364..=57375)
        || char::from_u32(code)
            .is_some_and(|c| !c.is_control() && !(0xe000..=0xf8ff).contains(&code))
}

fn function(number: u8) -> Result<Vec<u8>, Failure> {
    let sequence = match number {
        1 => "\x1bOP",
        2 => "\x1bOQ",
        3 => "\x1bOR",
        4 => "\x1bOS",
        5 => "\x1b[15~",
        6 => "\x1b[17~",
        7 => "\x1b[18~",
        8 => "\x1b[19~",
        9 => "\x1b[20~",
        10 => "\x1b[21~",
        11 => "\x1b[23~",
        12 => "\x1b[24~",
        _ => return Err(Failure::Unsupported),
    };
    Ok(sequence.as_bytes().into())
}

fn control(c: char) -> Result<Vec<u8>, Failure> {
    let c = c.to_ascii_uppercase();
    if !('@'..='_').contains(&c) {
        return Err(Failure::Unsupported);
    }
    Ok(vec![c as u8 & 0x1f])
}
