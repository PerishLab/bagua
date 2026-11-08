use super::{link::Link, socket::Socket};
use crate::Host;
use std::{
    collections::BTreeMap,
    io,
    path::Path,
    sync::atomic::{AtomicBool, Ordering},
    time::{Duration, Instant},
};
use workspace::Id;

pub struct Service {
    socket: Socket,
    host: Host,
    links: BTreeMap<Id, Link>,
    drain: Option<Instant>,
}

impl Service {
    pub fn bind(path: &Path) -> io::Result<Self> {
        Self::configured(path, crate::config::History::default())
    }

    pub fn configured(path: &Path, history: crate::config::History) -> io::Result<Self> {
        Self::identified(path, history, None)
    }

    pub fn identified(
        path: &Path,
        history: crate::config::History,
        identity: Option<protocol::wire::Image>,
    ) -> io::Result<Self> {
        history.check().map_err(super::error)?;
        let mut scope = [0; 16];
        let mut filled = 0;
        while filled < scope.len() {
            let count = rustix::rand::getrandom(
                &mut scope[filled..],
                rustix::rand::GetRandomFlags::empty(),
            )?;
            if count == 0 {
                return Err(io::Error::other("host context entropy unavailable"));
            }
            filled += count;
        }
        Ok(Self {
            socket: Socket::bind(path)?,
            host: Host::identified(scope, history, identity).map_err(super::error)?,
            links: BTreeMap::new(),
            drain: None,
        })
    }

    pub fn run(&mut self, stop: &AtomicBool) -> io::Result<()> {
        while self.step(stop)? {
            std::thread::sleep(Duration::from_millis(2));
        }
        Ok(())
    }

    pub fn step(&mut self, stop: &AtomicBool) -> io::Result<bool> {
        if stop.load(Ordering::Relaxed) {
            self.host.terminate().map_err(super::error)?;
        }
        if !self.host.stopped() {
            self.accept()?;
        }
        let mut closed = Vec::new();
        for (id, link) in &mut self.links {
            match link.step(&mut self.host, *id) {
                Ok(true) => {}
                Ok(false) => closed.push(*id),
                Err(error) => {
                    eprintln!("bagua connection {} closed: {error}", id.value());
                    closed.push(*id);
                }
            }
        }
        for id in closed {
            self.links.remove(&id);
            self.host.disconnect(id).map_err(super::error)?;
        }
        self.host.tick().map_err(super::error)?;
        if !self.host.stopped() {
            return Ok(true);
        }
        let deadline = self
            .drain
            .get_or_insert_with(|| Instant::now() + Duration::from_secs(2));
        let running =
            Instant::now() < *deadline && self.links.keys().any(|id| !self.host.idle(*id));
        if !running {
            self.host
                .outcome()
                .map_err(|fault| io::Error::other(format!("{}: {}", fault.code, fault.message)))?;
        }
        Ok(running)
    }

    fn accept(&mut self) -> io::Result<()> {
        for _ in 0..4 {
            let stream = match self.socket.listener.accept() {
                Ok((stream, _)) => stream,
                Err(error) if error.kind() == io::ErrorKind::WouldBlock => return Ok(()),
                Err(error) if error.kind() == io::ErrorKind::Interrupted => continue,
                Err(error) => return Err(error),
            };
            if super::socket::peer(&stream).is_err() {
                continue;
            }
            let actor = match self.host.connect() {
                Ok(actor) => actor,
                Err(_) => continue,
            };
            let link = match Link::new(stream) {
                Ok(link) => link,
                Err(error) => {
                    self.host.disconnect(actor).map_err(super::error)?;
                    return Err(error);
                }
            };
            self.links.insert(actor, link);
        }
        Ok(())
    }
}
