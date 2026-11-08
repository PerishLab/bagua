use crate::{Failure, Id, Target, World};

impl World {
    pub(crate) fn rename(
        &mut self,
        actor: Id,
        target: Target,
        name: String,
    ) -> Result<(), Failure> {
        if !crate::valid(&name) {
            return Err(Failure::Name);
        }
        let (session, window) = self.location(target)?;
        self.sessions[&session].require(actor)?;
        match target {
            Target::Session(_) => {
                if self.find(&name).is_some_and(|found| found.id() != session) {
                    return Err(Failure::Conflict);
                }
                self.sessions.get_mut(&session).unwrap().name = name;
            }
            Target::Window(_) => {
                let session = self.sessions.get_mut(&session).unwrap();
                if session
                    .windows
                    .values()
                    .any(|found| found.name() == name && Some(found.id()) != window)
                {
                    return Err(Failure::Conflict);
                }
                session.windows.get_mut(&window.unwrap()).unwrap().name = name;
            }
            Target::Pane(_) => return Err(Failure::State),
        }
        Ok(())
    }
}
