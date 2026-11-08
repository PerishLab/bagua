mod extended;
mod mouse;
mod paste;
mod sequence;
use crossterm::event::{Event, KeyCode, KeyEvent, KeyModifiers};
use protocol::Failure;
use std::{
    collections::VecDeque,
    time::{Duration, Instant},
};

pub struct Keyboard {
    bytes: VecDeque<u8>,
    escape: Option<Instant>,
    paste: Option<paste::Paste>,
    discard: bool,
    delay: Duration,
}

impl Default for Keyboard {
    fn default() -> Self {
        Self::new(Duration::from_millis(50))
    }
}

impl Keyboard {
    pub fn new(delay: Duration) -> Self {
        Self {
            bytes: VecDeque::new(),
            escape: None,
            paste: None,
            discard: false,
            delay,
        }
    }

    pub fn delay(&mut self, delay: Duration) {
        self.delay = delay;
    }

    pub fn room(&self) -> usize {
        8192 - self.bytes.len()
    }
    pub fn push(&mut self, bytes: &[u8]) -> Result<(), Failure> {
        if bytes.len() > self.room() {
            return Err(Failure::Capacity);
        }
        self.bytes.extend(bytes);
        Ok(())
    }
    pub fn next(&mut self, now: Instant) -> Option<Result<Event, Failure>> {
        if self.discard {
            while let Some(byte) = self.bytes.pop_front() {
                if (0x40..=0x7e).contains(&byte) {
                    self.discard = false;
                    break;
                }
            }
            if self.discard {
                return None;
            }
        }
        if let Some(paste) = &mut self.paste {
            while let Some(byte) = self.bytes.pop_front() {
                if let Some(result) = paste.feed(byte) {
                    self.paste = None;
                    return Some(result.map(Event::Paste));
                }
            }
            return None;
        }
        let byte = *self.bytes.front()?;
        if byte == 27 {
            return self.escaped(now);
        }
        let length = match byte {
            0..=127 => 1,
            0xc2..=0xdf => 2,
            0xe0..=0xef => 3,
            0xf0..=0xf4 => 4,
            _ => {
                self.bytes.pop_front();
                return Some(Err(Failure::Payload));
            }
        };
        if self.bytes.len() < length {
            return None;
        }
        let bytes: Vec<_> = self.bytes.drain(..length).collect();
        Some(character(&bytes).map(|(code, mods)| Event::Key(KeyEvent::new(code, mods))))
    }
    fn escaped(&mut self, now: Instant) -> Option<Result<Event, Failure>> {
        let started = *self.escape.get_or_insert(now);
        let timed = now.saturating_duration_since(started) >= self.delay;
        if self.bytes.len() == 1 {
            if !timed {
                return None;
            }
            self.bytes.pop_front();
            self.escape = None;
            return Some(Ok(Event::Key(KeyEvent::new(
                KeyCode::Esc,
                KeyModifiers::NONE,
            ))));
        }
        let data: Vec<_> = self.bytes.iter().take(64).copied().collect();
        if data.starts_with(b"\x1b[200~") {
            self.bytes.drain(..6);
            self.escape = None;
            self.paste = Some(paste::Paste::default());
            return self.next(now);
        }
        if let Some((length, event)) = sequence::parse(&data) {
            self.bytes.drain(..length);
            self.escape = None;
            return Some(event);
        }
        if timed && data.len() == 2 {
            self.bytes.drain(..2);
            self.escape = None;
            return Some(Ok(Event::Key(KeyEvent::new(
                KeyCode::Char(char::from(data[1])),
                KeyModifiers::ALT,
            ))));
        }
        if data.len() >= 64 || timed {
            self.discard = data.starts_with(b"\x1b[");
            self.bytes.drain(..data.len());
            self.escape = None;
            return Some(Err(Failure::Unavailable));
        }
        None
    }
}

pub(super) fn character(bytes: &[u8]) -> Result<(KeyCode, KeyModifiers), Failure> {
    let value = std::str::from_utf8(bytes)
        .map_err(|_| Failure::Payload)?
        .chars()
        .next()
        .ok_or(Failure::Payload)?;
    let (code, mods) = match value {
        '\t' => (KeyCode::Tab, KeyModifiers::NONE),
        '\r' | '\n' => (KeyCode::Enter, KeyModifiers::NONE),
        '\x7f' | '\x08' => (KeyCode::Backspace, KeyModifiers::NONE),
        '\0' => (KeyCode::Char('@'), KeyModifiers::CONTROL),
        c @ '\x01'..='\x1a' => (
            KeyCode::Char(char::from(c as u8 + b'a' - 1)),
            KeyModifiers::CONTROL,
        ),
        c @ '\x1c'..='\x1f' => (
            KeyCode::Char(char::from(c as u8 + 64)),
            KeyModifiers::CONTROL,
        ),
        c if !c.is_control() => (KeyCode::Char(c), KeyModifiers::NONE),
        _ => return Err(Failure::Unavailable),
    };
    Ok((code, mods))
}
