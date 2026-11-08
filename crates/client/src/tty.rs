use rustix::fs::{Mode, OFlags};
use std::{
    fs::File,
    io::{self, IsTerminal, Read, Write},
    time::{Duration, Instant},
};

const ENTER: &[u8] =
    b"\x1b[?1049h\x1b[>1u\x1b[?25l\x1b[?7l\x1b[?2004h\x1b[?1000h\x1b[?1003h\x1b[?1006h";
const LEAVE: &[u8] =
    b"\x1b[0m\x1b[?1000l\x1b[?1002l\x1b[?1003l\x1b[?1006l\x1b[?2004l\x1b[?7h\x1b[?25h\x1b[<u\x1b[?1049l";

pub struct Terminal {
    file: File,
    writing: Option<(Vec<u8>, usize)>,
    latest: Option<Vec<u8>>,
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
            writing: None,
            latest: None,
            active: true,
        };
        terminal.control(ENTER)?;
        Ok(terminal)
    }

    pub fn size(&self) -> io::Result<(u16, u16)> {
        crossterm::terminal::size()
    }

    pub fn offer(&mut self, frame: Vec<u8>) -> io::Result<()> {
        if frame.len() > 4_194_304 {
            return Err(io::Error::other("presentation exceeds its byte budget"));
        }
        self.latest = Some(frame);
        Ok(())
    }

    pub fn flush(&mut self) -> io::Result<()> {
        if self.writing.is_none() {
            self.writing = self.latest.take().map(|bytes| (bytes, 0));
        }
        let Some((bytes, offset)) = &mut self.writing else {
            return Ok(());
        };
        let length = (bytes.len() - *offset).min(65_536);
        match self.file.write(&bytes[*offset..*offset + length]) {
            Ok(0) => {
                return Err(io::Error::new(
                    io::ErrorKind::WriteZero,
                    "terminal output closed",
                ));
            }
            Ok(count) => *offset += count,
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
        if *offset == bytes.len() {
            self.writing = None;
        }
        Ok(())
    }

    pub fn finish(&mut self) -> io::Result<()> {
        if !self.active {
            return Ok(());
        }
        self.active = false;
        let mode = crossterm::terminal::disable_raw_mode();
        let display = self.control(LEAVE);
        match (mode, display) {
            (Err(mode), Err(display)) => Err(io::Error::other(format!(
                "terminal mode: {mode}; display: {display}"
            ))),
            (Err(error), _) | (_, Err(error)) => Err(error),
            _ => Ok(()),
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
