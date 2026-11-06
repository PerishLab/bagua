use crate::{Failure, Id};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum State {
    Pending,
    Failed,
    Running,
    Exited(Option<i32>),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Pane {
    id: Id,
    state: State,
}

impl Pane {
    pub(crate) fn new(id: Id) -> Self {
        Self {
            id,
            state: State::Pending,
        }
    }

    pub fn id(&self) -> Id {
        self.id
    }

    pub fn state(&self) -> State {
        self.state
    }

    pub(crate) fn observe(&mut self, state: State) -> Result<(), Failure> {
        match (self.state, state) {
            (State::Pending, State::Running | State::Failed)
            | (State::Running, State::Exited(_)) => self.state = state,
            _ => return Err(Failure::State),
        }
        Ok(())
    }
}
