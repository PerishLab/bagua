use crate::Id;

#[derive(serde::Serialize, serde::Deserialize, Clone, Copy, Debug, PartialEq, Eq)]
pub enum Failure {
    Busy,
    Observer,
    Missing,
    Capacity,
    Geometry,
    State,
    Exhausted,
    Name,
    Conflict,
    Stale,
}

impl Failure {
    pub fn code(self) -> &'static str {
        match self {
            Self::Busy => "busy",
            Self::Observer => "observer",
            Self::Missing => "missing",
            Self::Capacity => "capacity",
            Self::Geometry => "geometry",
            Self::State => "state",
            Self::Exhausted => "exhausted",
            Self::Name => "name",
            Self::Conflict => "conflict",
            Self::Stale => "stale",
        }
    }

    pub fn message(self) -> &'static str {
        match self {
            Self::Busy => "client is already joined or control is held by another client",
            Self::Observer => "operation requires the session controller",
            Self::Missing => "exact target or connected client does not exist",
            Self::Capacity => "workspace resource limit exceeded",
            Self::Geometry => "geometry cannot contain the layout and minimum pane sizes",
            Self::State => "operation is invalid for the current pane state",
            Self::Exhausted => "identity or revision space exhausted",
            Self::Name => {
                "name must contain 1..=64 UTF-8 bytes without controls or surrounding whitespace"
            }
            Self::Conflict => "the exact name already exists in this scope",
            Self::Stale => "controller changed; review the current owner before taking control",
        }
    }
}

#[derive(serde::Serialize, serde::Deserialize, Clone, Debug, Default, PartialEq, Eq)]
pub struct Control {
    owner: Option<Id>,
}

impl Control {
    pub fn owner(&self) -> Option<Id> {
        self.owner
    }

    pub fn acquire(&mut self, client: Id) -> Result<(), Failure> {
        if self.owner.is_some_and(|owner| owner != client) {
            return Err(Failure::Busy);
        }
        self.owner = Some(client);
        Ok(())
    }

    pub fn require(&self, client: Id) -> Result<(), Failure> {
        if self.owner == Some(client) {
            Ok(())
        } else {
            Err(Failure::Observer)
        }
    }

    pub fn transfer(&mut self, actor: Id, next: Id) -> Result<(), Failure> {
        self.require(actor)?;
        self.owner = Some(next);
        Ok(())
    }

    pub fn take(&mut self, client: Id, expected: Option<Id>) -> Result<(), Failure> {
        if self.owner != expected {
            return Err(Failure::Stale);
        }
        self.owner = Some(client);
        Ok(())
    }

    pub fn release(&mut self, client: Id) {
        if self.owner == Some(client) {
            self.owner = None;
        }
    }
}
