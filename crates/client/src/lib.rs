use protocol::{Cursor, Failure, Update};
use terminal::{Extent, Revision};

#[derive(Debug)]
pub struct View {
    extent: Extent,
    cursor: Cursor,
}

impl View {
    pub fn new(target: workspace::Target, extent: Extent) -> Self {
        Self {
            extent,
            cursor: Cursor::new(target),
        }
    }

    pub fn resize(&mut self, extent: Extent) {
        self.extent = extent;
    }

    pub fn crop(&self, content: Extent, x: u16, y: u16) -> Option<Extent> {
        Some(content.remaining(x, y)?.fit(self.extent))
    }

    pub fn accept(&mut self, update: Update) -> Result<(), Failure> {
        self.cursor.accept(update)
    }

    pub fn revision(&self) -> Option<Revision> {
        self.cursor.revision()
    }
}
