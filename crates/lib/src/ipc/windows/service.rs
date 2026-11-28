use super::{link::Link, security::random, socket::Socket};
use crate::Host;
use std::{
    collections::BTreeMap,
    io,
    path::Path,
    sync::atomic::{AtomicBool, Ordering},
    time::{Duration, Instant},
};
use windows_sys::Win32::System::Threading::WaitForMultipleObjects;
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
        history.check().map_err(crate::ipc::error)?;
        let mut scope = [0; 16];
        random(&mut scope)?;
        Ok(Self {
            socket: Socket::bind(path)?,
            host: Host::identified(scope, history, identity)
                .map_err(crate::ipc::error)?
                .bound(path.to_path_buf()),
            links: BTreeMap::new(),
            drain: None,
        })
    }

    pub fn events(&mut self, observer: impl FnMut(crate::Event) + Send + 'static) {
        self.host.events(observer);
    }

    pub fn run(&mut self, stop: &AtomicBool) -> io::Result<()> {
        while self.step(stop)? {
            self.wait();
        }
        Ok(())
    }

    pub fn step(&mut self, stop: &AtomicBool) -> io::Result<bool> {
        if stop.load(Ordering::Relaxed) {
            self.host.terminate().map_err(crate::ipc::error)?;
        }
        if !self.host.stopped() {
            self.accept()?;
        }
        self.pump(false)?;
        self.host.tick().map_err(crate::ipc::error)?;
        self.pump(true)?;
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

    fn pump(&mut self, flush: bool) -> io::Result<()> {
        let mut closed = Vec::new();
        for (id, link) in &mut self.links {
            let outcome = if flush {
                link.flush(&mut self.host, *id)
            } else {
                link.step(&mut self.host, *id)
            };
            match outcome {
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
            self.host.disconnect(id).map_err(crate::ipc::error)?;
        }
        Ok(())
    }

    fn wait(&self) {
        if self.socket.ready() {
            return;
        }
        let mut handles = Vec::with_capacity(64);
        if let Some(event) = self.socket.event() {
            handles.push(event);
        }
        for (id, link) in &self.links {
            if link.ready(&self.host, *id) {
                return;
            }
            link.events(&mut handles);
        }
        if self.host.wake(&mut handles) {
            return;
        }
        let count = handles.len().min(64);
        if count == 0 {
            std::thread::sleep(Duration::from_millis(2));
            return;
        }
        unsafe {
            WaitForMultipleObjects(count as u32, handles.as_ptr(), 0, 16);
        }
    }

    fn accept(&mut self) -> io::Result<()> {
        for _ in 0..4 {
            let Some(handle) = self.socket.accept()? else {
                return Ok(());
            };
            let actor = match self.host.connect() {
                Ok(actor) => actor,
                Err(_) => continue,
            };
            let link = match Link::new(handle) {
                Ok(link) => link,
                Err(error) => {
                    self.host.disconnect(actor).map_err(crate::ipc::error)?;
                    return Err(error);
                }
            };
            self.links.insert(actor, link);
        }
        Ok(())
    }
}
