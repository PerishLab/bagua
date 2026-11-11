use protocol::Failure;
use terminal::{Extent, Point, Tracking};

#[derive(Clone)]
pub struct Paint {
    pub(crate) tracking: Tracking,
    pub(crate) rows: Vec<String>,
    pub(crate) cursor: Option<Point>,
    pub(crate) extent: Extent,
}

impl Paint {
    pub(crate) fn check(&self) -> Result<(), Failure> {
        if self.rows.iter().map(String::len).sum::<usize>() + 128 > 4_194_304 {
            return Err(Failure::Capacity);
        }
        Ok(())
    }

    pub(crate) fn render(&self, previous: Option<&Self>) -> Vec<u8> {
        let previous = previous.filter(|old| old.extent == self.extent);
        let changed: Vec<_> = self
            .rows
            .iter()
            .enumerate()
            .filter(|(index, row)| previous.is_none_or(|old| old.rows.get(*index) != Some(*row)))
            .map(|(_, row)| row.as_str())
            .collect();
        if changed.is_empty()
            && previous
                .is_some_and(|old| old.cursor == self.cursor && old.tracking == self.tracking)
        {
            return Vec::new();
        }
        let mut output = String::from("\x1b[?25l");
        if previous.is_none_or(|old| old.tracking != self.tracking) {
            output.push_str(match self.tracking {
                Tracking::Off => "\x1b[?1000l\x1b[?1002l\x1b[?1003l\x1b[?1006l",
                Tracking::Motion => "\x1b[?1002l\x1b[?1003h\x1b[?1006h",
                _ => "\x1b[?1003l\x1b[?1002h\x1b[?1006h",
            });
        }
        if previous.is_none() {
            output.push_str("\x1b[0m\x1b[2J");
        }
        for row in changed {
            output.push_str(row);
        }
        output.push_str("\x1b[0m");
        if let Some(point) = self.cursor {
            output.push_str(&format!("\x1b[{};{}H\x1b[?25h", point.y + 1, point.x + 1));
        }
        output.into_bytes()
    }
}
