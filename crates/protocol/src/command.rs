use crate::Failure;
use workspace::{Geometry, Id, Operation};

#[derive(serde::Serialize, serde::Deserialize, Clone, Debug, PartialEq, Eq)]
pub enum Input {
    Text(String),
    Paste(String),
    Key(terminal::Key),
    Mouse(terminal::Mouse),
}

impl Input {
    pub fn borrow(&self) -> terminal::Input<'_> {
        match self {
            Self::Text(text) => terminal::Input::Text(text),
            Self::Paste(text) => terminal::Input::Paste(text),
            Self::Key(key) => terminal::Input::Key(*key),
            Self::Mouse(mouse) => terminal::Input::Mouse(*mouse),
        }
    }

    pub fn check(&self) -> Result<(), Failure> {
        if let Self::Text(text) | Self::Paste(text) = self
            && text.len() > terminal::CHUNK
        {
            return Err(Failure::Payload);
        }
        Ok(())
    }
}

#[derive(serde::Serialize, serde::Deserialize, Clone, Debug, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub enum Command {
    Capabilities,
    Start {
        pane: Id,
        program: crate::wire::Program,
    },
    Inspect(Id),
    Describe(Id),
    History {
        pane: Id,
        read: crate::wire::Read,
    },
    Shutdown,
    Subscribe {
        target: workspace::Target,
        after: Option<terminal::Revision>,
    },
    Unsubscribe(workspace::Target),
    Workspace(Operation),
    Send {
        pane: Id,
        input: Input,
    },
}

impl Command {
    pub fn create(columns: u16, rows: u16) -> Result<Self, Failure> {
        Ok(Self::Workspace(Operation::Create(Self::geometry(
            columns, rows,
        )?)))
    }

    pub fn attach(value: u64, columns: u16, rows: u16) -> Result<Self, Failure> {
        Ok(Self::Workspace(Operation::Join {
            session: Self::identity(value)?,
            viewport: Self::geometry(columns, rows)?,
        }))
    }

    pub fn identity(value: u64) -> Result<Id, Failure> {
        Id::new(value).ok_or(Failure::Identity)
    }

    pub fn geometry(columns: u16, rows: u16) -> Result<Geometry, Failure> {
        Geometry::new(columns, rows).ok_or(Failure::Workspace(workspace::Failure::Geometry))
    }
}
