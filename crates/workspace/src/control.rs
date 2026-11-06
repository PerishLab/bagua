use crate::Id;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Failure {
    Busy,
    Observer,
}

#[derive(Debug, Default)]
pub struct Control {
    owner: Option<Id>,
}

impl Control {
    pub fn owner(&self) -> Option<Id> {
        self.owner
    }

    pub fn acquire(&mut self, client: Id) -> Result<(), Failure> {
        if self.owner.is_some_and(|owner| owner != client) {
            return Err(Failure::Busy);
        }
        self.owner = Some(client);
        Ok(())
    }

    pub fn require(&self, client: Id) -> Result<(), Failure> {
        if self.owner == Some(client) {
            Ok(())
        } else {
            Err(Failure::Observer)
        }
    }

    pub fn transfer(&mut self, actor: Id, next: Id) -> Result<(), Failure> {
        self.require(actor)?;
        self.owner = Some(next);
        Ok(())
    }

    pub fn release(&mut self, client: Id) {
        if self.owner == Some(client) {
            self.owner = None;
        }
    }
}
