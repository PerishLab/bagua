use terminal::Revision;

pub trait Stamped {
    fn revision(&self) -> Revision;
}

impl Stamped for terminal::Snapshot {
    fn revision(&self) -> Revision {
        terminal::Snapshot::revision(self)
    }
}

impl Stamped for workspace::Snapshot {
    fn revision(&self) -> Revision {
        Revision::new(workspace::Snapshot::revision(self))
    }
}
