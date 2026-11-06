use serde::{Deserialize, Serialize};
use terminal::{Cell, Cursor, Extent, Modes, Point, Revision, Snapshot};

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Screen {
    pub bytes: u64,
    pub unsupported: u64,
    pub bells: u64,
    pub revision: Revision,
    pub extent: Extent,
    pub cursor: Cursor,
    pub modes: Modes,
    pub cells: Vec<Cell>,
    pub wrapped: Vec<bool>,
}

impl crate::Stamped for Screen {
    fn revision(&self) -> Revision {
        self.revision
    }
}

impl From<&Snapshot> for Screen {
    fn from(snapshot: &Snapshot) -> Self {
        let extent = snapshot.extent();
        let cells = (0..extent.height())
            .flat_map(|y| (0..extent.width()).map(move |x| snapshot.cell(Point { x, y }).unwrap()))
            .collect();
        Self {
            bytes: 0,
            unsupported: 0,
            bells: 0,
            revision: snapshot.revision(),
            extent,
            cursor: snapshot.cursor(),
            modes: snapshot.modes(),
            cells,
            wrapped: (0..extent.height())
                .map(|row| snapshot.wrapped(row))
                .collect(),
        }
    }
}
