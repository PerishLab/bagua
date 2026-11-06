use crate::Failure;
use terminal::Revision;
use workspace::Target;

#[derive(serde::Serialize, serde::Deserialize, Clone, Copy, Debug, PartialEq, Eq)]
pub struct Update {
    pub target: Target,
    pub base: Option<Revision>,
    pub revision: Revision,
}

#[derive(serde::Serialize, serde::Deserialize, Clone, Debug)]
pub struct Cursor {
    target: Target,
    revision: Option<Revision>,
}

impl Cursor {
    pub fn target(&self) -> Target {
        self.target
    }
    pub fn new(target: Target) -> Self {
        Self {
            target,
            revision: None,
        }
    }

    pub fn revision(&self) -> Option<Revision> {
        self.revision
    }

    pub fn accept(&mut self, update: Update) -> Result<(), Failure> {
        if update.target != self.target {
            return Err(Failure::Target);
        }
        if self
            .revision
            .is_some_and(|current| update.revision <= current)
        {
            return Err(Failure::Stale);
        }
        if update.base.is_some() && update.base != self.revision {
            return Err(Failure::Baseline);
        }
        self.revision = Some(update.revision);
        Ok(())
    }
}
