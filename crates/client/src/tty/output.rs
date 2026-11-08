#[cfg(test)]
mod checks;
use crate::{Copy, Paint};
use std::{
    collections::VecDeque,
    io::{self, Write},
    time::{Duration, Instant},
};

#[derive(Default)]
pub(super) struct Output {
    active: Option<Writing>,
    latest: Option<View>,
    baseline: Option<Paint>,
    copies: VecDeque<Copy>,
    bytes: usize,
    count: usize,
}

struct Writing {
    bytes: Vec<u8>,
    offset: usize,
    reliable: bool,
    paint: Option<Paint>,
}

enum View {
    Raw(Vec<u8>),
    Paint(Paint),
}

impl Output {
    pub fn offer(&mut self, frame: Vec<u8>) -> io::Result<()> {
        if frame.len() > 4_194_304 {
            return Err(io::Error::other("presentation exceeds its byte budget"));
        }
        self.latest = Some(View::Raw(frame));
        Ok(())
    }

    pub fn present(&mut self, paint: Paint) -> io::Result<()> {
        paint
            .check()
            .map_err(|_| io::Error::other("presentation exceeds its byte budget"))?;
        self.latest = Some(View::Paint(paint));
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
                    paint: None,
                })
                .or_else(|| {
                    self.latest.take().map(|view| match view {
                        View::Raw(bytes) => Writing {
                            bytes,
                            offset: 0,
                            reliable: false,
                            paint: None,
                        },
                        View::Paint(paint) => Writing {
                            bytes: paint.render(self.baseline.as_ref()),
                            offset: 0,
                            reliable: false,
                            paint: Some(paint),
                        },
                    })
                });
        }
        let Some(active) = &mut self.active else {
            return Ok(());
        };
        if active.write(writer)? {
            if active.reliable {
                self.bytes -= active.bytes.len();
                self.count -= 1;
            } else {
                self.baseline = active.paint.take();
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

impl Writing {
    fn write(&mut self, writer: &mut impl Write) -> io::Result<bool> {
        let length = (self.bytes.len() - self.offset).min(65_536);
        if length == 0 {
            return Ok(true);
        }
        match writer.write(&self.bytes[self.offset..self.offset + length]) {
            Ok(0) => {
                return Err(io::Error::new(
                    io::ErrorKind::WriteZero,
                    "terminal output closed",
                ));
            }
            Ok(count) => self.offset += count,
            Err(error)
                if matches!(
                    error.kind(),
                    io::ErrorKind::WouldBlock | io::ErrorKind::Interrupted
                ) =>
            {
                return Ok(false);
            }
            Err(error) => return Err(error),
        }
        Ok(self.offset == self.bytes.len())
    }
}
