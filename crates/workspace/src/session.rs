use crate::{Control, Failure, Geometry, Id, Sizing, Window};
use std::collections::BTreeMap;

#[derive(serde::Serialize, serde::Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct Session {
    id: Id,
    control: Control,
    sizing: Sizing,
    clients: BTreeMap<Id, Geometry>,
    pub(crate) windows: BTreeMap<Id, Window>,
    pub(crate) selected: Option<Id>,
}

impl Session {
    pub fn new(id: Id, geometry: Geometry) -> Self {
        Self {
            id,
            control: Control::default(),
            sizing: Sizing::Automatic(geometry),
            clients: BTreeMap::new(),
            windows: BTreeMap::new(),
            selected: None,
        }
    }

    pub fn id(&self) -> Id {
        self.id
    }
    pub fn controller(&self) -> Option<Id> {
        self.control.owner()
    }
    pub fn sizing(&self) -> Sizing {
        self.sizing
    }
    pub fn geometry(&self) -> Geometry {
        self.sizing.geometry()
    }
    pub fn selected(&self) -> Option<Id> {
        self.selected
    }
    pub fn windows(&self) -> impl Iterator<Item = &Window> {
        self.windows.values()
    }
    pub fn viewport(&self, client: Id) -> Option<Geometry> {
        self.clients.get(&client).copied()
    }

    pub fn clients(&self) -> impl Iterator<Item = (Id, Geometry)> + '_ {
        self.clients.iter().map(|(id, geometry)| (*id, *geometry))
    }

    pub fn join(&mut self, client: Id, viewport: Geometry) -> Result<(), Failure> {
        if self.clients.contains_key(&client) {
            return Err(Failure::Busy);
        }
        if self.clients.len() >= 64 {
            return Err(Failure::Capacity);
        }
        self.clients.insert(client, viewport);
        Ok(())
    }

    pub fn acquire(&mut self, client: Id) -> Result<(), Failure> {
        let viewport = self.viewport(client).ok_or(Failure::Missing)?;
        self.preview(viewport)?;
        self.control.acquire(client)?;
        self.follow(viewport);
        Ok(())
    }

    pub fn claim(&mut self, client: Id, geometry: Geometry) -> Result<(), Failure> {
        self.viewport(client).ok_or(Failure::Missing)?;
        self.fit(geometry)?;
        self.control.acquire(client)?;
        self.sizing = Sizing::Fixed(geometry);
        Ok(())
    }

    pub fn transfer(&mut self, actor: Id, next: Id) -> Result<(), Failure> {
        self.control.require(actor)?;
        let viewport = self.viewport(next).ok_or(Failure::Missing)?;
        self.preview(viewport)?;
        self.control.transfer(actor, next)?;
        self.follow(viewport);
        Ok(())
    }

    pub fn require(&self, actor: Id) -> Result<(), Failure> {
        self.control.require(actor)
    }

    pub fn detach(&mut self, client: Id) {
        self.clients.remove(&client);
        self.control.release(client);
    }

    pub fn resize(&mut self, actor: Id, geometry: Geometry) -> Result<Geometry, Failure> {
        self.control.require(actor)?;
        self.preview(geometry)?;
        self.clients.insert(actor, geometry);
        self.follow(geometry);
        Ok(self.geometry())
    }

    pub fn report(&mut self, actor: Id, geometry: Geometry) -> Result<Geometry, Failure> {
        self.viewport(actor).ok_or(Failure::Missing)?;
        if self.controller() == Some(actor) {
            return self.resize(actor, geometry);
        }
        self.clients.insert(actor, geometry);
        Ok(self.geometry())
    }

    pub fn fixed(&mut self, actor: Id, geometry: Geometry) -> Result<(), Failure> {
        self.control.require(actor)?;
        self.fit(geometry)?;
        self.sizing = Sizing::Fixed(geometry);
        Ok(())
    }

    pub fn automatic(&mut self, actor: Id, geometry: Geometry) -> Result<(), Failure> {
        self.control.require(actor)?;
        self.fit(geometry)?;
        self.clients.insert(actor, geometry);
        self.sizing = Sizing::Automatic(geometry);
        Ok(())
    }

    fn follow(&mut self, geometry: Geometry) {
        if matches!(self.sizing, Sizing::Automatic(_)) {
            self.sizing = Sizing::Automatic(geometry);
        }
    }

    fn preview(&self, geometry: Geometry) -> Result<(), Failure> {
        if matches!(self.sizing, Sizing::Automatic(_)) {
            self.fit(geometry)?;
        }
        Ok(())
    }

    pub(crate) fn fit(&self, geometry: Geometry) -> Result<(), Failure> {
        if !geometry.valid() {
            return Err(Failure::Geometry);
        }
        for window in self.windows.values() {
            window.placements(geometry)?;
        }
        Ok(())
    }
}
