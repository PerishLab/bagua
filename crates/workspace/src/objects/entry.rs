use crate::{Change, Entry, Failure, Id, Outcome, Target, World};

impl World {
    pub(crate) fn enter(&mut self, actor: Id, entry: Entry) -> Result<Outcome, Failure> {
        if !crate::valid(&entry.name) {
            return Err(Failure::Name);
        }
        if !entry.viewport.valid() {
            return Err(Failure::Geometry);
        }
        if entry.observe && entry.fixed {
            return Err(Failure::Observer);
        }
        let revision = self.advance()?;
        let mut next = self.clone();
        let mut created = Vec::new();
        let id = match next.find(&entry.name) {
            Some(session) => session.id(),
            None if entry.create && !entry.observe => {
                let creation = next.named(entry.viewport, entry.name)?;
                created = vec![
                    Target::Session(creation.session),
                    Target::Window(creation.window),
                    Target::Pane(creation.pane),
                ];
                creation.session
            }
            None => return Err(Failure::Missing),
        };
        let session = next.sessions.get_mut(&id).unwrap();
        session.join(actor, entry.viewport)?;
        if !entry.observe {
            if entry.fixed {
                session.claim(actor, session.geometry())?;
            } else {
                session.acquire(actor)?;
            }
        }
        next.revision = revision;
        *self = next;
        Ok(Outcome::Entered {
            session: id,
            change: Change {
                revision,
                created,
                retired: Vec::new(),
            },
        })
    }
}
