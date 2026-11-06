use protocol::{
    Request,
    wire::{self, Message},
};
use std::{
    io::{self, Read, Write},
    net::Shutdown,
    os::unix::net::UnixStream,
    path::Path,
    time::Duration,
};

pub struct Connection {
    stream: UnixStream,
}

impl Connection {
    pub fn channel(self) -> io::Result<super::Channel> {
        super::Channel::new(self.stream)
    }

    pub fn connect(path: &Path) -> io::Result<Self> {
        super::socket::namespace(path)?;
        let stream = UnixStream::connect(path)?;
        super::socket::peer(&stream)?;
        stream.set_read_timeout(Some(Duration::from_secs(10)))?;
        stream.set_write_timeout(Some(Duration::from_secs(10)))?;
        Ok(Self { stream })
    }

    pub fn send(&mut self, request: &Request) -> io::Result<()> {
        self.stream
            .write_all(&wire::encode(request, wire::REQUEST).map_err(super::error)?)
    }

    pub fn receive(&mut self) -> io::Result<Message> {
        let mut header = [0; 4];
        self.stream.read_exact(&mut header)?;
        let length = u32::from_be_bytes(header) as usize;
        if length == 0 || length > wire::FRAME {
            return Err(super::error(protocol::Failure::Payload));
        }
        let mut bytes = vec![0; length];
        self.stream.read_exact(&mut bytes)?;
        serde_json::from_slice(&bytes).map_err(io::Error::other)
    }

    pub fn finish(&self) -> io::Result<()> {
        self.stream.shutdown(Shutdown::Write)
    }

    pub fn duplicate(&self) -> io::Result<Self> {
        Ok(Self {
            stream: self.stream.try_clone()?,
        })
    }

    pub fn streaming(&self) -> io::Result<()> {
        self.stream.set_read_timeout(None)
    }
}
