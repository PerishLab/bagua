mod action;
mod client;
mod context;
mod effects;
mod history;
mod input;
mod pane;
mod publish;

use client::Client;
use pane::Pane;
use protocol::{Failure, Request};
use std::collections::BTreeMap;
use workspace::{Id, World};

pub const CLIENTS: usize = 32;
pub const PANES: usize = 32;
pub const CELLS: usize = 262_144;
pub const COLUMNS: u16 = 256;
pub const ROWS: u16 = 64;

#[derive(Default)]
pub struct Host {
    world: World,
    panes: BTreeMap<Id, Pane>,
    clients: BTreeMap<Id, Client>,
    next: u64,
    closing: Vec<effects::Closing>,
    stopping: Option<(Id, u64)>,
    finished: bool,
    turn: usize,
    scope: [u8; 16],
    history: crate::config::History,
    identity: Option<protocol::wire::Image>,
}

impl Host {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn configured(scope: [u8; 16], history: crate::config::History) -> Result<Self, Failure> {
        history.check()?;
        Ok(Self {
            scope,
            history,
            ..Self::default()
        })
    }

    pub fn identified(
        scope: [u8; 16],
        history: crate::config::History,
        identity: Option<protocol::wire::Image>,
    ) -> Result<Self, Failure> {
        if identity.is_some_and(|image| image.inode == 0) {
            return Err(Failure::Payload);
        }
        let mut host = Self::configured(scope, history)?;
        host.identity = identity;
        Ok(host)
    }

    pub fn world(&self) -> &World {
        &self.world
    }

    pub fn connect(&mut self) -> Result<Id, Failure> {
        if self.clients.len() >= CLIENTS || self.stopping.is_some() {
            return Err(Failure::Capacity);
        }
        self.next = self
            .next
            .checked_add(1)
            .filter(|next| *next < u64::MAX)
            .ok_or(Failure::Capacity)?;
        let id = Id::new(self.next).ok_or(Failure::Identity)?;
        self.clients.insert(id, Client::new(id, self.identity)?);
        Ok(id)
    }

    pub fn disconnect(&mut self, id: Id) -> Result<(), Failure> {
        self.clients.remove(&id);
        self.world.disconnect(id)?;
        Ok(())
    }

    pub fn submit(&mut self, actor: Id, request: Request) -> Result<(), Failure> {
        let client = self.clients.get_mut(&actor).ok_or(Failure::Identity)?;
        client.room()?;
        let id = request.id;
        let result = if id <= client.last {
            Err(Failure::Identity)
        } else {
            client.last = id;
            self.apply(actor, request)
        };
        if let Err(error) = result {
            self.reply(actor, id, Err(error))?;
        }
        Ok(())
    }

    pub fn tick(&mut self) -> Result<(), Failure> {
        self.budget();
        self.pump()?;
        self.settle()?;
        self.publish()?;
        self.reap()?;
        Ok(())
    }

    pub fn stopped(&self) -> bool {
        self.stopping.is_some() && self.panes.values().all(|pane| pane.process.is_none())
    }

    pub fn outcome(&self) -> Result<(), protocol::wire::Fault> {
        if !self.stopped() {
            return Err(Failure::Pending.into());
        }
        self.panes
            .values()
            .find_map(|pane| pane.cleanup.clone())
            .map_or(Ok(()), Err)
    }

    pub fn pending(&self, actor: Id) -> bool {
        self.closing.iter().any(|item| item.actor == actor)
            || self
                .clients
                .get(&actor)
                .is_some_and(|client| !client.wanted.is_empty() || client.history.is_some())
            || self.panes.values().any(|pane| pane.pending(actor))
    }

    pub fn output(&mut self, actor: Id) -> Option<&[u8]> {
        self.clients.get_mut(&actor)?.output()
    }

    pub fn consume(&mut self, actor: Id, count: usize) {
        if let Some(client) = self.clients.get_mut(&actor) {
            client.consume(count);
        }
    }

    pub fn idle(&self, actor: Id) -> bool {
        self.clients.get(&actor).is_none_or(Client::idle)
    }

    pub fn connected(&self, actor: Id) -> bool {
        self.clients.contains_key(&actor)
    }

    pub fn terminate(&mut self) -> Result<(), Failure> {
        if self.stopping.is_none() {
            self.shutdown(Id::new(u64::MAX).unwrap(), 0)?;
        }
        Ok(())
    }
}
