use crate::Host;
use protocol::{
    Request,
    wire::{Decoder, REQUEST},
};
use std::{
    io::{self, Read, Write},
    os::unix::net::UnixStream,
    time::{Duration, Instant},
};
use workspace::Id;

pub(super) struct Link {
    stream: UnixStream,
    decoder: Decoder,
    ended: bool,
    progress: Instant,
}

impl Link {
    pub fn new(stream: UnixStream) -> io::Result<Self> {
        stream.set_nonblocking(true)?;
        Ok(Self {
            stream,
            decoder: Decoder::new(REQUEST),
            ended: false,
            progress: Instant::now(),
        })
    }

    pub fn step(&mut self, host: &mut Host, actor: Id) -> io::Result<bool> {
        if !host.connected(actor) {
            return Ok(false);
        }
        if let Some(bytes) = host.output(actor) {
            let length = bytes.len().min(65_536);
            match self.stream.write(&bytes[..length]) {
                Ok(0) => return Ok(false),
                Ok(count) => {
                    host.consume(actor, count);
                    self.progress = Instant::now();
                }
                Err(error) if transient(&error) => {}
                Err(error) => return Err(error),
            }
        }
        if let Some(request) = self.read()? {
            host.submit(actor, request).map_err(super::error)?;
        }
        let queued = !matches!(
            (self.decoder.empty(), host.pending(actor), host.idle(actor)),
            (true, false, true)
        );
        if self.ended && !queued {
            return Ok(false);
        }
        if self.progress.elapsed() > Duration::from_secs(10) && queued {
            return Err(io::Error::new(
                io::ErrorKind::TimedOut,
                "connection made no IO progress",
            ));
        }
        Ok(true)
    }

    fn read(&mut self) -> io::Result<Option<Request>> {
        if let Some(request) = self.decoder.take().map_err(super::error)? {
            return Ok(Some(request));
        }
        if !self.ended {
            let mut bytes = [0; 4096];
            let length = self.decoder.room().min(bytes.len());
            match self.stream.read(&mut bytes[..length]) {
                Ok(0) => self.ended = true,
                Ok(count) => {
                    self.decoder.push(&bytes[..count]).map_err(super::error)?;
                    self.progress = Instant::now();
                }
                Err(error) if transient(&error) => {}
                Err(error) => return Err(error),
            }
        }
        let request = self.decoder.take().map_err(super::error)?;
        if self.ended && request.is_none() && !self.decoder.empty() {
            return Err(io::Error::new(
                io::ErrorKind::UnexpectedEof,
                "incomplete request frame",
            ));
        }
        Ok(request)
    }
}

fn transient(error: &io::Error) -> bool {
    matches!(
        error.kind(),
        io::ErrorKind::WouldBlock | io::ErrorKind::Interrupted
    )
}
