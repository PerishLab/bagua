#[derive(Debug)]
pub struct Host {
    session: workspace::Session,
    process: pty::Lifecycle,
    revision: terminal::Revision,
}

impl Host {
    pub fn new(session: workspace::Session) -> Self {
        Self {
            session,
            process: pty::Lifecycle::default(),
            revision: terminal::Revision::default(),
        }
    }

    pub fn session(&self) -> &workspace::Session {
        &self.session
    }

    pub fn process(&self) -> &pty::Lifecycle {
        &self.process
    }

    pub fn revision(&self) -> terminal::Revision {
        self.revision
    }

    pub fn detach(&mut self, client: workspace::Id) {
        self.session.detach(client);
    }
}
