use super::{duplex::Duplex, handle::Handle};
use protocol::{
    Request,
    wire::{self, Decoder, Message},
};
use std::{collections::VecDeque, io};

pub struct Channel {
    stream: Duplex,
    decoder: Decoder,
    queue: VecDeque<(Vec<u8>, usize)>,
    bytes: usize,
}

impl Channel {
    pub(super) fn new(handle: Handle) -> io::Result<Self> {
        Ok(Self {
            stream: Duplex::new(handle)?,
            decoder: Decoder::new(wire::FRAME),
            queue: VecDeque::new(),
            bytes: 0,
        })
    }

    pub fn send(&mut self, request: &Request) -> io::Result<()> {
        let bytes = wire::encode(request, wire::REQUEST).map_err(crate::ipc::error)?;
        if self.queue.len() >= 64 || self.bytes + bytes.len() > 262_144 {
            return Err(io::Error::new(
                io::ErrorKind::WouldBlock,
                "client transport budget full",
            ));
        }
        self.bytes += bytes.len();
        self.queue.push_back((bytes, 0));
        Ok(())
    }

    pub fn flush(&mut self) -> io::Result<()> {
        let Some((bytes, offset)) = self.queue.front_mut() else {
            return Ok(());
        };
        let length = (bytes.len() - *offset).min(65_536);
        match self.stream.write(&bytes[*offset..*offset + length]) {
            Ok(0) => {
                return Err(io::Error::new(
                    io::ErrorKind::WriteZero,
                    "host connection closed",
                ));
            }
            Ok(count) => {
                *offset += count;
            }
            Err(error) if transient(&error) => return Ok(()),
            Err(error) => return Err(error),
        }
        if *offset == bytes.len() {
            self.bytes -= bytes.len();
            self.queue.pop_front();
        }
        Ok(())
    }

    pub fn receive(&mut self) -> io::Result<Option<Message>> {
        if let Some(message) = self.decoder.take().map_err(crate::ipc::error)? {
            return Ok(Some(message));
        }
        let mut bytes = [0; 65_536];
        let length = self.decoder.room().min(bytes.len());
        match self.stream.read(&mut bytes[..length]) {
            Ok(0) => {
                return Err(io::Error::new(
                    io::ErrorKind::UnexpectedEof,
                    "host connection closed",
                ));
            }
            Ok(count) => self
                .decoder
                .push(&bytes[..count])
                .map_err(crate::ipc::error)?,
            Err(error) if transient(&error) => return Ok(None),
            Err(error) => return Err(error),
        }
        self.decoder.take().map_err(crate::ipc::error)
    }
}

fn transient(error: &io::Error) -> bool {
    matches!(
        error.kind(),
        io::ErrorKind::WouldBlock | io::ErrorKind::Interrupted
    )
}
