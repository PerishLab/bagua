use crate::{Change, Failure, Id, Operation, Outcome, Sizing, Target, Window, World};

impl World {
    pub fn apply(&mut self, actor: Id, operation: Operation) -> Result<Outcome, Failure> {
        match operation {
            Operation::Enter(entry) => return self.enter(actor, entry),
            Operation::Named { geometry, name } => {
                let created = self.named(geometry, name)?;
                return Ok(self.creation(created));
            }
            Operation::Read => return Ok(Outcome::Snapshot(self.snapshot())),
            Operation::Create(geometry) => {
                let created = self.create(geometry)?;
                return Ok(self.creation(created));
            }
            _ => {}
        }
        let revision = self.advance()?;
        let mut change = Change {
            revision,
            created: Vec::new(),
            retired: Vec::new(),
        };
        self.mutate(actor, operation, &mut change)?;
        self.commit(revision, change.created.len() as u64);
        Ok(Outcome::Changed(change))
    }

    fn mutate(
        &mut self,
        actor: Id,
        operation: Operation,
        change: &mut Change,
    ) -> Result<(), Failure> {
        match operation {
            Operation::Rename { target, name } => self.rename(actor, target, name)?,
            Operation::Join { session, viewport } => self.edit(session)?.join(actor, viewport)?,
            Operation::Acquire { session, fixed } => match fixed {
                Some(geometry) => self.edit(session)?.claim(actor, geometry)?,
                None => self.edit(session)?.acquire(actor)?,
            },
            Operation::Transfer { session, next } => self.edit(session)?.transfer(actor, next)?,
            Operation::Take {
                session,
                expected,
                fixed,
            } => self.edit(session)?.take(actor, expected, fixed)?,
            Operation::Detach(session) => {
                let session = self.edit(session)?;
                session.viewport(actor).ok_or(Failure::Missing)?;
                session.detach(actor);
            }
            Operation::Resize { session, geometry } => {
                self.edit(session)?.report(actor, geometry)?;
            }
            Operation::Size { session, sizing } => match sizing {
                Sizing::Automatic(geometry) => self.edit(session)?.automatic(actor, geometry)?,
                Sizing::Fixed(geometry) => self.edit(session)?.fixed(actor, geometry)?,
            },
            Operation::Window(session) => change.created = self.window(actor, session)?,
            Operation::Split { pane, split } => change.created = self.split(actor, pane, split)?,
            Operation::Focus(pane) => self.focus(actor, pane)?,
            Operation::Header { session, enabled } => {
                self.edit(session)?.reserve(actor, enabled)?
            }
            Operation::Adjust {
                window,
                divider,
                ratio,
            } => {
                let (session, _) = self.location(Target::Window(window))?;
                let session = self.edit(session)?;
                session.require(actor)?;
                let geometry = session.canvas();
                session
                    .windows
                    .get_mut(&window)
                    .unwrap()
                    .adjust(divider, ratio, geometry)?;
            }
            Operation::Close(target) => change.retired = self.close(actor, target)?,
            Operation::Read
            | Operation::Create(_)
            | Operation::Enter(_)
            | Operation::Named { .. } => return Err(Failure::State),
        }
        Ok(())
    }

    fn creation(&self, created: crate::Creation) -> Outcome {
        Outcome::Changed(Change {
            revision: self.revision,
            created: vec![
                Target::Session(created.session),
                Target::Window(created.window),
                Target::Pane(created.pane),
            ],
            retired: Vec::new(),
        })
    }

    fn edit(&mut self, id: Id) -> Result<&mut crate::Session, Failure> {
        self.sessions.get_mut(&id).ok_or(Failure::Missing)
    }

    fn window(&mut self, actor: Id, session: Id) -> Result<Vec<Target>, Failure> {
        self.capacity()?;
        let ids = self.ids(2)?;
        let session = self.edit(session)?;
        session.require(actor)?;
        if session.windows.len() >= 32 {
            return Err(Failure::Capacity);
        }
        let name = crate::names::available(
            "window",
            session.windows.values().map(|window| window.name().into()),
        )?;
        let window = Window::new(ids[0], ids[1], name);
        window.placements(session.canvas())?;
        session.windows.insert(ids[0], window);
        session.selected = Some(ids[0]);
        Ok(vec![Target::Window(ids[0]), Target::Pane(ids[1])])
    }

    fn split(&mut self, actor: Id, pane: Id, split: crate::Split) -> Result<Vec<Target>, Failure> {
        self.capacity()?;
        let id = self.ids(1)?[0];
        let (session, window) = self.location(Target::Pane(pane))?;
        let session = self.edit(session)?;
        session.require(actor)?;
        let geometry = session.canvas();
        session
            .windows
            .get_mut(&window.unwrap())
            .unwrap()
            .split(pane, id, split, geometry)?;
        session.selected = window;
        Ok(vec![Target::Pane(id)])
    }

    fn focus(&mut self, actor: Id, pane: Id) -> Result<(), Failure> {
        let (session, window) = self.location(Target::Pane(pane))?;
        let session = self.edit(session)?;
        session.require(actor)?;
        session
            .windows
            .get_mut(&window.unwrap())
            .unwrap()
            .select(pane)?;
        session.selected = window;
        Ok(())
    }

    fn close(&mut self, actor: Id, target: Target) -> Result<Vec<Id>, Failure> {
        let (session, _) = self.location(target)?;
        self.sessions[&session].require(actor)?;
        self.remove(target)
    }
}
