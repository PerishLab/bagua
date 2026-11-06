use crate::{Failure, Input, Profile};
use workspace::Id;

#[derive(serde::Serialize, serde::Deserialize, Clone, Debug, PartialEq, Eq)]
pub enum Intent {
    Subscribe {
        target: workspace::Target,
        after: Option<terminal::Revision>,
    },
    Unsubscribe(workspace::Target),
    Input {
        pane: Id,
        input: Input,
    },
    Retire(workspace::Change),
}

#[derive(serde::Serialize, serde::Deserialize, Clone, Debug, PartialEq, Eq)]
pub enum Outcome {
    Profile(Profile),
    Workspace(workspace::Outcome),
}

#[derive(serde::Serialize, serde::Deserialize, Clone, Debug, PartialEq, Eq)]
pub enum Status {
    Admitted,
    Applied(Outcome),
    Accepted(Intent),
}

#[derive(serde::Serialize, serde::Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct Reply {
    pub request: u64,
    pub result: Result<Status, Failure>,
}

#[derive(serde::Serialize, serde::Deserialize, Clone, Debug, PartialEq, Eq)]
pub enum Event {
    Subscribed {
        request: u64,
        target: workspace::Target,
        revision: terminal::Revision,
    },
    Failed {
        request: u64,
        error: Failure,
    },
    Completed {
        request: u64,
    },
    Process {
        pane: Id,
        state: workspace::State,
    },
    Presented {
        pane: Id,
        revision: terminal::Revision,
    },
}
