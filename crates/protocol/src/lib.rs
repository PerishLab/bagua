mod catalog;
mod sync;

pub use catalog::{Capability, Component, Profile, State};
pub use sync::{Cursor, Update};

pub const VERSION: u16 = 1;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Command {
    Capabilities,
    Create,
    Sessions,
    Attach { session: workspace::Id },
}

impl Command {
    pub fn attach(value: u64) -> Result<Self, Failure> {
        Ok(Self::Attach {
            session: workspace::Id::new(value).ok_or(Failure::Identity)?,
        })
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Request {
    pub version: u16,
    pub command: Command,
}

impl Request {
    pub fn new(command: Command) -> Self {
        Self {
            version: VERSION,
            command,
        }
    }

    pub fn check(self) -> Result<Command, Failure> {
        if self.version != VERSION {
            return Err(Failure::Version);
        }
        Ok(self.command)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Failure {
    Version,
    Identity,
    Unavailable,
    Target,
    Baseline,
    Stale,
}

impl Failure {
    pub fn code(self) -> &'static str {
        match self {
            Self::Version => "version",
            Self::Identity => "identity",
            Self::Unavailable => "unavailable",
            Self::Target => "target",
            Self::Baseline => "baseline",
            Self::Stale => "stale",
        }
    }

    pub fn message(self) -> &'static str {
        match self {
            Self::Version => "unsupported protocol version",
            Self::Identity => "identity must be nonzero",
            Self::Unavailable => "terminal hosting is not implemented in this source baseline",
            Self::Target => "update belongs to a different target",
            Self::Baseline => "incremental baseline is missing; request a complete snapshot",
            Self::Stale => "update does not advance the installed revision",
        }
    }
}
