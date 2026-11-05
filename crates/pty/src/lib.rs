#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum State {
    #[default]
    Absent,
    Running,
    Exited(Option<i32>),
    Closed,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Event {
    Started,
    Exited(Option<i32>),
    Closed,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Transition;

#[derive(Debug, Default)]
pub struct Lifecycle {
    state: State,
}

impl Lifecycle {
    pub fn state(&self) -> State {
        self.state
    }

    pub fn observe(&mut self, event: Event) -> Result<State, Transition> {
        let next = match (self.state, event) {
            (State::Absent, Event::Started) => State::Running,
            (State::Running, Event::Exited(code)) => State::Exited(code),
            (State::Exited(_), Event::Closed) => State::Closed,
            _ => return Err(Transition),
        };
        self.state = next;
        Ok(next)
    }
}
