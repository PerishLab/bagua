mod output;
use output::Output;
use rustix::fs::{Mode, OFlags};
use std::{
    fs::File,
    io::{self, IsTerminal, Read, Write},
    time::{Duration, Instant},
};

const ENTER: &[u8] =
    b"\x1b[?1049h\x1b[>1u\x1b[?25l\x1b[?7l\x1b[?2004h\x1b[?1003l\x1b[?1002h\x1b[?1006h";
const LEAVE: &[u8] =
    b"\x1b[0m\x1b[?1000l\x1b[?1002l\x1b[?1003l\x1b[?1006l\x1b[?2004l\x1b[?7h\x1b[?25h\x1b[<u\x1b[?1049l";

pub struct Terminal {
    file: File,
    output: Output,
    active: bool,
}

impl Terminal {
    pub fn read(&mut self, buffer: &mut [u8]) -> io::Result<usize> {
        if buffer.is_empty() {
            return Ok(0);
        }
        match self.file.read(buffer) {
            Ok(0) => Err(io::Error::new(
                io::ErrorKind::UnexpectedEof,
                "terminal input closed",
            )),
            Err(error)
                if matches!(
                    error.kind(),
                    io::ErrorKind::WouldBlock | io::ErrorKind::Interrupted
                ) =>
            {
                Ok(0)
            }
            other => other,
        }
    }
    pub fn enter() -> io::Result<Self> {
        if !io::stdin().is_terminal() || !io::stdout().is_terminal() {
            return Err(io::Error::other(
                "interactive mode requires terminal stdin and stdout",
            ));
        }
        let file = rustix::fs::open(
            "/dev/tty",
            OFlags::RDWR | OFlags::CLOEXEC | OFlags::NONBLOCK | OFlags::NOCTTY,
            Mode::empty(),
        )?;
        crossterm::terminal::enable_raw_mode()?;
        let mut terminal = Self {
            file: file.into(),
            output: Output::default(),
            active: true,
        };
        terminal.control(ENTER)?;
        Ok(terminal)
    }

    pub fn size(&self) -> io::Result<(u16, u16)> {
        crossterm::terminal::size()
    }

    pub fn offer(&mut self, frame: Vec<u8>) -> io::Result<()> {
        self.output.offer(frame)
    }
    pub fn present(&mut self, paint: crate::Paint) -> io::Result<()> {
        self.output.present(paint)
    }

    pub fn copy(&mut self, copy: &crate::Copy) -> bool {
        self.output.copy(copy)
    }

    pub fn flush(&mut self) -> io::Result<()> {
        self.output.flush(&mut self.file)
    }

    pub fn finish(&mut self) -> io::Result<()> {
        if !self.active {
            return Ok(());
        }
        self.active = false;
        let mut failures = Vec::new();
        if let Err(error) = self.output.drain(&mut self.file) {
            failures.push(format!(
                "output incomplete; copied effects unknown: {error}"
            ));
            if let Err(error) = self.control(b"\x18\x1b\\") {
                failures.push(format!("output cancellation: {error}"));
            }
        }
        if let Err(error) = crossterm::terminal::disable_raw_mode() {
            failures.push(format!("terminal mode: {error}"));
        }
        if let Err(error) = self.control(LEAVE) {
            failures.push(format!("display: {error}"));
        }
        if failures.is_empty() {
            Ok(())
        } else {
            Err(io::Error::other(failures.join("; ")))
        }
    }

    fn control(&mut self, bytes: &[u8]) -> io::Result<()> {
        let deadline = Instant::now() + Duration::from_millis(250);
        let mut offset = 0;
        while offset < bytes.len() {
            match self.file.write(&bytes[offset..]) {
                Ok(0) => {
                    return Err(io::Error::new(
                        io::ErrorKind::WriteZero,
                        "terminal output closed",
                    ));
                }
                Ok(count) => offset += count,
                Err(error) if error.kind() == io::ErrorKind::Interrupted => continue,
                Err(error)
                    if error.kind() == io::ErrorKind::WouldBlock && Instant::now() < deadline =>
                {
                    std::thread::sleep(Duration::from_millis(2))
                }
                Err(error) => return Err(error),
            }
        }
        Ok(())
    }
}

impl Drop for Terminal {
    fn drop(&mut self) {
        if let Err(error) = self.finish() {
            let _ = writeln!(io::stderr(), "bagua terminal restoration failed: {error}");
        }
    }
}
