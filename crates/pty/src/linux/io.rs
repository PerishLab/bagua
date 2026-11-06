use super::{Process, spawn::winsize};
use crate::{CHUNK, Failure, Interest, Ready, Size, Stage, Transfer};
use rustix::{
    event::{PollFd, PollFlags, Timespec, poll},
    io::Errno,
};
use std::time::Duration;

impl Process {
    pub fn size(&self) -> Result<Size, Failure> {
        let size = rustix::termios::tcgetwinsize(self.master()?)
            .map_err(|error| Failure::io(Stage::Resize, error))?;
        Size::new(size.ws_col, size.ws_row).ok_or(Failure::Input("kernel reported zero PTY size"))
    }

    pub fn resize(&self, size: Size) -> Result<(), Failure> {
        rustix::termios::tcsetwinsize(self.master()?, winsize(size))
            .map_err(|error| Failure::io(Stage::Resize, error))
    }

    pub fn read(&mut self, buffer: &mut [u8]) -> Result<Transfer, Failure> {
        if buffer.is_empty() {
            return Err(Failure::Input("read buffer must not be empty"));
        }
        let length = buffer.len().min(CHUNK);
        let result = rustix::io::read(self.master()?, &mut buffer[..length]);
        match result {
            Ok(0) => Ok(Transfer::Eof),
            other => transfer(other, Stage::Read),
        }
    }

    pub fn write(&mut self, buffer: &[u8]) -> Result<Transfer, Failure> {
        let length = buffer.len().min(CHUNK);
        transfer(
            rustix::io::write(self.master()?, &buffer[..length]),
            Stage::Write,
        )
    }

    pub fn poll(&self, interest: Interest, timeout: Duration) -> Result<Ready, Failure> {
        if timeout > Duration::from_secs(1) {
            return Err(Failure::Input("poll timeout exceeds one second"));
        }
        let events = match interest {
            Interest::Read => PollFlags::IN,
            Interest::Write => PollFlags::OUT,
            Interest::Both => PollFlags::IN | PollFlags::OUT,
        };
        let mut entries = [PollFd::new(self.master()?, events)];
        let timeout =
            Timespec::try_from(timeout).map_err(|_| Failure::Input("poll timeout is invalid"))?;
        match poll(&mut entries, Some(&timeout)) {
            Ok(_) => readiness(entries[0].revents()),
            Err(Errno::INTR) => Ok(Ready::default()),
            Err(error) => Err(Failure::io(Stage::Poll, error)),
        }
    }
}

fn transfer(result: Result<usize, Errno>, stage: Stage) -> Result<Transfer, Failure> {
    match result {
        Ok(bytes) => Ok(Transfer::Bytes(bytes)),
        Err(Errno::AGAIN | Errno::INTR) => Ok(Transfer::Pending),
        Err(Errno::IO) => Ok(Transfer::Eof),
        Err(error) => Err(Failure::io(stage, error)),
    }
}

fn readiness(events: PollFlags) -> Result<Ready, Failure> {
    if events.contains(PollFlags::NVAL) {
        return Err(Failure::Closed);
    }
    Ok(Ready {
        read: events.contains(PollFlags::IN),
        write: events.contains(PollFlags::OUT),
        hangup: events.intersects(PollFlags::HUP | PollFlags::ERR),
    })
}
