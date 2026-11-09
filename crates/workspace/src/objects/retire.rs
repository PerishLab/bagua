use crate::{Change, Failure, Id, State, Target, Window, World};

impl World {
    pub fn finish(&mut self, pane: Id) -> Result<Change, Failure> {
        if !matches!(
            self.pane(pane).ok_or(Failure::Missing)?.state(),
            State::Exited(_)
        ) {
            return Err(Failure::State);
        }
        let revision = self.advance()?;
        let retired = self.remove(Target::Pane(pane))?;
        self.revision = revision;
        Ok(Change {
            revision,
            created: Vec::new(),
            retired,
        })
    }

    pub(super) fn remove(&mut self, target: Target) -> Result<Vec<Id>, Failure> {
        let (session, window) = self.location(target)?;
        if let Target::Session(_) = target {
            let session = self.sessions.remove(&session).unwrap();
            return Ok(session
                .windows()
                .flat_map(Window::panes)
                .map(|pane| pane.id())
                .collect());
        }
        let id = session;
        let session = self.sessions.get_mut(&id).ok_or(Failure::Missing)?;
        let window = window.unwrap();
        let retired = match target {
            Target::Pane(pane) => {
                session.windows.get_mut(&window).unwrap().remove(pane);
                vec![pane]
            }
            Target::Window(_) => session.windows[&window].panes.keys().copied().collect(),
            Target::Session(_) => return Err(Failure::State),
        };
        if matches!(target, Target::Window(_)) || session.windows[&window].panes.is_empty() {
            session.windows.remove(&window);
        }
        if session
            .selected
            .is_some_and(|id| !session.windows.contains_key(&id))
        {
            session.selected = session.windows.keys().next().copied();
        }
        if session.windows.is_empty() {
            self.sessions.remove(&id);
        }
        Ok(retired)
    }
}
