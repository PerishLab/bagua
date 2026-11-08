use protocol::Failure;
use terminal::{Extent, Point};

#[derive(Clone)]
pub struct Paint {
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
        if changed.is_empty() && previous.is_some_and(|old| old.cursor == self.cursor) {
            return Vec::new();
        }
        let mut output = String::from("\x1b[?25l");
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
