use crate::{Control, Failure, Geometry, Id, Sizing};

#[derive(Debug)]
pub struct Session {
    id: Id,
    control: Control,
    sizing: Sizing,
}

impl Session {
    pub fn new(id: Id, geometry: Geometry) -> Self {
        Self {
            id,
            control: Control::default(),
            sizing: Sizing::Automatic(geometry),
        }
    }

    pub fn id(&self) -> Id {
        self.id
    }

    pub fn controller(&self) -> Option<Id> {
        self.control.owner()
    }

    pub fn acquire(&mut self, client: Id) -> Result<(), Failure> {
        self.control.acquire(client)
    }

    pub fn transfer(&mut self, actor: Id, next: Id) -> Result<(), Failure> {
        self.control.transfer(actor, next)
    }

    pub fn detach(&mut self, client: Id) {
        self.control.release(client);
    }

    pub fn sizing(&self) -> Sizing {
        self.sizing
    }

    pub fn geometry(&self) -> Geometry {
        self.sizing.geometry()
    }

    pub fn resize(&mut self, actor: Id, geometry: Geometry) -> Result<Geometry, Failure> {
        self.control.require(actor)?;
        if matches!(self.sizing, Sizing::Automatic(_)) {
            self.sizing = Sizing::Automatic(geometry);
        }
        Ok(self.geometry())
    }

    pub fn fixed(&mut self, actor: Id, geometry: Geometry) -> Result<(), Failure> {
        self.control.require(actor)?;
        self.sizing = Sizing::Fixed(geometry);
        Ok(())
    }

    pub fn automatic(&mut self, actor: Id, geometry: Geometry) -> Result<(), Failure> {
        self.control.require(actor)?;
        self.sizing = Sizing::Automatic(geometry);
        Ok(())
    }
}
