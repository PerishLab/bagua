use crate::Session;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Snapshot {
    revision: u64,
    sessions: Vec<Session>,
}

impl Snapshot {
    pub(crate) fn new(revision: u64, sessions: Vec<Session>) -> Self {
        Self { revision, sessions }
    }

    pub fn revision(&self) -> u64 {
        self.revision
    }

    pub fn sessions(&self) -> impl Iterator<Item = &Session> {
        self.sessions.iter()
    }
}
