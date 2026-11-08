#[cfg(test)]
mod checks;
use crate::Copy;
use std::{
    collections::VecDeque,
    io::{self, Write},
    time::{Duration, Instant},
};

#[derive(Default)]
pub(super) struct Output {
    active: Option<Writing>,
    latest: Option<Vec<u8>>,
    copies: VecDeque<Copy>,
    bytes: usize,
    count: usize,
}

struct Writing {
    bytes: Vec<u8>,
    offset: usize,
    reliable: bool,
}

impl Output {
    pub fn offer(&mut self, frame: Vec<u8>) -> io::Result<()> {
        if frame.len() > 4_194_304 {
            return Err(io::Error::other("presentation exceeds its byte budget"));
        }
        self.latest = Some(frame);
        Ok(())
    }

    pub fn copy(&mut self, copy: &Copy) -> bool {
        if self.count >= 4 || self.bytes + copy.0.len() > 2_097_152 {
            return false;
        }
        self.bytes += copy.0.len();
        self.count += 1;
        self.copies.push_back(copy.clone());
        true
    }

    pub fn flush(&mut self, writer: &mut impl Write) -> io::Result<()> {
        if self.active.is_none() {
            self.active = self
                .copies
                .pop_front()
                .map(|copy| Writing {
                    bytes: copy.0,
                    offset: 0,
                    reliable: true,
                })
                .or_else(|| {
                    self.latest.take().map(|bytes| Writing {
                        bytes,
                        offset: 0,
                        reliable: false,
                    })
                });
        }
        let Some(active) = &mut self.active else {
            return Ok(());
        };
        let length = (active.bytes.len() - active.offset).min(65_536);
        match writer.write(&active.bytes[active.offset..active.offset + length]) {
            Ok(0) if length == 0 => {}
            Ok(0) => {
                return Err(io::Error::new(
                    io::ErrorKind::WriteZero,
                    "terminal output closed",
                ));
            }
            Ok(count) => active.offset += count,
            Err(error)
                if matches!(
                    error.kind(),
                    io::ErrorKind::WouldBlock | io::ErrorKind::Interrupted
                ) =>
            {
                return Ok(());
            }
            Err(error) => return Err(error),
        }
        if active.offset == active.bytes.len() {
            if active.reliable {
                self.bytes -= active.bytes.len();
                self.count -= 1;
            }
            self.active = None;
        }
        Ok(())
    }

    pub fn drain(&mut self, writer: &mut impl Write) -> io::Result<()> {
        self.latest = None;
        let deadline = Instant::now() + Duration::from_millis(250);
        while self.active.is_some() || !self.copies.is_empty() {
            self.flush(writer)?;
            if Instant::now() >= deadline {
                return Err(io::Error::new(
                    io::ErrorKind::TimedOut,
                    "terminal output drain",
                ));
            }
            std::thread::sleep(Duration::from_millis(2));
        }
        Ok(())
    }
}
