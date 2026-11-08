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
    Entered { session: Id, change: Change },
}

#[derive(serde::Serialize, serde::Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct Entry {
    pub name: String,
    pub viewport: Geometry,
    pub create: bool,
    pub observe: bool,
    pub fixed: bool,
}

#[derive(serde::Serialize, serde::Deserialize, Clone, Debug, PartialEq, Eq)]
pub enum Operation {
    Create(Geometry),
    Named {
        geometry: Geometry,
        name: String,
    },
    Enter(Entry),
    Rename {
        target: Target,
        name: String,
    },
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
    Header {
        session: Id,
        enabled: bool,
    },
    Adjust {
        window: Id,
        divider: crate::Divider,
        ratio: crate::Ratio,
    },
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
