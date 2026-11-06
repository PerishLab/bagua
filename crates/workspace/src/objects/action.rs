use crate::{Geometry, Id, Sizing, Snapshot, Split, Target};

#[derive(serde::Serialize, serde::Deserialize, Clone, Copy, Debug, PartialEq, Eq)]
pub struct Creation {
    pub session: Id,
    pub window: Id,
    pub pane: Id,
}

#[derive(serde::Serialize, serde::Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct Change {
    pub revision: u64,
    pub created: Vec<Target>,
    pub retired: Vec<Id>,
}

#[derive(serde::Serialize, serde::Deserialize, Clone, Debug, PartialEq, Eq)]
pub enum Outcome {
    Snapshot(Snapshot),
    Changed(Change),
}

#[derive(serde::Serialize, serde::Deserialize, Clone, Copy, Debug, PartialEq, Eq)]
pub enum Operation {
    Create(Geometry),
    Read,
    Join {
        session: Id,
        viewport: Geometry,
    },
    Acquire {
        session: Id,
        fixed: Option<Geometry>,
    },
    Transfer {
        session: Id,
        next: Id,
    },
    Detach(Id),
    Window(Id),
    Split {
        pane: Id,
        split: Split,
    },
    Focus(Id),
    Close(Target),
    Resize {
        session: Id,
        geometry: Geometry,
    },
    Size {
        session: Id,
        sizing: Sizing,
    },
}
