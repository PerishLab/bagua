mod catalog;
mod command;
mod replica;
mod reply;
mod stamp;
mod sync;
mod watch;
pub mod wire;

pub use catalog::{Capability, Component, Profile, State};
pub use command::{Command, Input};
pub use replica::{Frame, Latest, Packet, Replica};
pub use reply::{Event, Intent, Outcome, Reply, Status};
pub use stamp::Stamped;
pub use sync::{Cursor, Update};
pub use watch::{SUBSCRIPTIONS, Subscriptions};

pub const VERSION: u16 = 5;

#[derive(serde::Serialize, serde::Deserialize, Clone, Debug, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Request {
    pub version: u16,
    pub id: u64,
    pub command: Command,
}

impl Request {
    pub fn new(command: Command) -> Self {
        Self {
            version: VERSION,
            id: 1,
            command,
        }
    }

    pub fn identified(id: u64, command: Command) -> Result<Self, Failure> {
        if id == 0 {
            return Err(Failure::Identity);
        }
        Ok(Self {
            version: VERSION,
            id,
            command,
        })
    }

    pub fn check(self) -> Result<Command, Failure> {
        if self.version != VERSION {
            return Err(Failure::Version);
        }
        if self.id == 0 {
            return Err(Failure::Identity);
        }
        Ok(self.command)
    }
}

#[derive(serde::Serialize, serde::Deserialize, Clone, Copy, Debug, PartialEq, Eq)]
pub enum Failure {
    Version,
    Identity,
    Unavailable,
    Target,
    Baseline,
    Stale,
    Workspace(workspace::Failure),
    Payload,
    Capacity,
    Native,
    Pending,
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
            Self::Workspace(error) => error.code(),
            Self::Payload => "payload",
            Self::Capacity => "capacity",
            Self::Native => "native",
            Self::Pending => "pending",
        }
    }

    pub fn message(self) -> &'static str {
        match self {
            Self::Version => "unsupported protocol version",
            Self::Identity => "identity must be nonzero",
            Self::Unavailable => {
                "operation requires a supported host connection or implemented endpoint"
            }
            Self::Target => "update belongs to a different target",
            Self::Baseline => "incremental baseline is missing; request a complete snapshot",
            Self::Stale => "update does not advance the installed revision",
            Self::Workspace(error) => error.message(),
            Self::Payload => "payload exceeds its declared bound or cannot be applied",
            Self::Capacity => "protocol resource limit exceeded",
            Self::Native => "native effect failed; inspect the associated result",
            Self::Pending => "an earlier operation still owns this resource",
        }
    }
}

impl From<workspace::Failure> for Failure {
    fn from(error: workspace::Failure) -> Self {
        Self::Workspace(error)
    }
}
