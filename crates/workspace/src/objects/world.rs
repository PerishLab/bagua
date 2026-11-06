use crate::{Creation, Failure, Geometry, Id, Pane, Session, Snapshot, State, Target, Window};
use std::collections::BTreeMap;

#[derive(Clone, Debug)]
pub struct World {
    pub(crate) sessions: BTreeMap<Id, Session>,
    pub(crate) revision: u64,
    next: u64,
}

impl Default for World {
    fn default() -> Self {
        Self {
            sessions: BTreeMap::new(),
            revision: 0,
            next: 1,
        }
    }
}

impl World {
    pub fn revision(&self) -> u64 {
        self.revision
    }
    pub fn session(&self, id: Id) -> Option<&Session> {
        self.sessions.get(&id)
    }
    pub fn snapshot(&self) -> Snapshot {
        Snapshot::new(self.revision, self.sessions.values().cloned().collect())
    }

    pub fn create(&mut self, geometry: Geometry) -> Result<Creation, Failure> {
        if !geometry.valid() {
            return Err(Failure::Geometry);
        }
        if self.sessions.len() >= 32 {
            return Err(Failure::Capacity);
        }
        self.capacity()?;
        let revision = self.advance()?;
        let ids = self.ids(3)?;
        let creation = Creation {
            session: ids[0],
            window: ids[1],
            pane: ids[2],
        };
        let mut session = Session::new(creation.session, geometry);
        session
            .windows
            .insert(creation.window, Window::new(creation.window, creation.pane));
        session.selected = Some(creation.window);
        self.sessions.insert(creation.session, session);
        self.commit(revision, 3);
        Ok(creation)
    }

    pub fn pane(&self, pane: Id) -> Option<&Pane> {
        self.sessions
            .values()
            .flat_map(Session::windows)
            .find_map(|w| w.panes.get(&pane))
    }

    pub fn observe(&mut self, pane: Id, state: State) -> Result<u64, Failure> {
        let revision = self.advance()?;
        let (session, window) = self.location(Target::Pane(pane))?;
        self.sessions
            .get_mut(&session)
            .unwrap()
            .windows
            .get_mut(&window.unwrap())
            .unwrap()
            .panes
            .get_mut(&pane)
            .unwrap()
            .observe(state)?;
        self.revision = revision;
        Ok(revision)
    }

    pub fn input(&self, actor: Id, pane: Id) -> Result<(), Failure> {
        let (session, _) = self.location(Target::Pane(pane))?;
        self.sessions[&session].require(actor)?;
        if self.pane(pane).unwrap().state() != State::Running {
            return Err(Failure::State);
        }
        Ok(())
    }

    pub fn watch(&self, actor: Id, target: Target) -> Result<(), Failure> {
        let (session, _) = self.location(target)?;
        self.sessions[&session]
            .viewport(actor)
            .ok_or(Failure::Missing)?;
        Ok(())
    }

    pub fn disconnect(&mut self, actor: Id) -> Result<Vec<Id>, Failure> {
        let affected = self
            .sessions
            .values()
            .filter(|s| s.viewport(actor).is_some())
            .map(Session::id)
            .collect::<Vec<_>>();
        if affected.is_empty() {
            return Ok(affected);
        }
        let revision = self.advance()?;
        for id in &affected {
            self.sessions.get_mut(id).unwrap().detach(actor);
        }
        self.revision = revision;
        Ok(affected)
    }

    pub fn location(&self, target: Target) -> Result<(Id, Option<Id>), Failure> {
        if let Target::Session(id) = target {
            return self
                .sessions
                .contains_key(&id)
                .then_some((id, None))
                .ok_or(Failure::Missing);
        }
        self.sessions
            .values()
            .find_map(|s| locate(s, target))
            .ok_or(Failure::Missing)
    }

    pub(crate) fn advance(&self) -> Result<u64, Failure> {
        self.revision.checked_add(1).ok_or(Failure::Exhausted)
    }

    pub(crate) fn capacity(&self) -> Result<(), Failure> {
        let panes: usize = self
            .sessions
            .values()
            .flat_map(Session::windows)
            .map(|w| w.panes.len())
            .sum();
        if panes >= 4096 {
            return Err(Failure::Capacity);
        }
        Ok(())
    }

    pub(crate) fn ids(&self, count: u64) -> Result<Vec<Id>, Failure> {
        self.next.checked_add(count).ok_or(Failure::Exhausted)?;
        Ok((0..count)
            .map(|offset| Id::new(self.next + offset).unwrap())
            .collect())
    }

    pub(crate) fn commit(&mut self, revision: u64, allocated: u64) {
        self.revision = revision;
        self.next += allocated;
    }
}

fn locate(session: &Session, target: Target) -> Option<(Id, Option<Id>)> {
    let window = session.windows().find(|window| match target {
        Target::Window(id) => window.id() == id,
        Target::Pane(id) => window.panes.contains_key(&id),
        Target::Session(_) => false,
    })?;
    Some((session.id(), Some(window.id())))
}
