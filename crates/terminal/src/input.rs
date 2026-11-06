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
            Self::Key(key) => key.encode(modes.cursor),
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
    fn encode(self, application: bool) -> Result<Vec<u8>, Failure> {
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
        }
    }
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
