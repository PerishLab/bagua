#[cfg(test)]
mod checks;
mod copy;
use crate::App;
pub(crate) use copy::Copies;
pub use copy::Copy;
use protocol::{Failure, Input};
use terminal::{Key, Point};
use workspace::Id;

pub(super) struct Selection {
    pub screen: protocol::wire::Screen,
    pub pane: Id,
    pub start: Point,
    pub end: Point,
}

impl Selection {
    pub fn range(&self) -> (Point, Point) {
        if (self.start.y, self.start.x) <= (self.end.y, self.end.x) {
            (self.start, self.end)
        } else {
            (self.end, self.start)
        }
    }
    pub fn contains(&self, pane: Id, point: Point) -> bool {
        let (start, end) = self.range();
        pane == self.pane
            && (point.y, point.x) >= (start.y, start.x)
            && (point.y, point.x) <= (end.y, end.x)
    }
}

impl App {
    pub(super) fn begin(&mut self) -> Result<(), Failure> {
        let pane = self.focus().ok_or(Failure::Target)?;
        let point = self.screen(pane).ok_or(Failure::Baseline)?.cursor.position;
        self.select(pane, point, false)
    }

    pub fn select(&mut self, pane: Id, point: Point, extend: bool) -> Result<(), Failure> {
        let screen = self.screen(pane).ok_or(Failure::Target)?;
        if point.x >= screen.extent.width() || point.y >= screen.extent.height() {
            return Err(Failure::Payload);
        }
        match &mut self.selection {
            Some(selection) if extend && selection.pane == pane => selection.end = point,
            _ => {
                let screen = self.screen(pane).unwrap().clone();
                self.selection = Some(Selection {
                    pane,
                    start: point,
                    end: point,
                    screen,
                });
            }
        }
        self.status(
            "Selection: default arrows extend/Escape cancels; use the configured copy binding",
        );
        Ok(())
    }

    pub(super) fn selecting(&mut self, input: Input) -> Result<(), Failure> {
        if input == Input::Key(Key::Escape) {
            self.live();
            return Ok(());
        }
        let selection = self.selection.as_ref().unwrap();
        let pane = selection.pane;
        let screen = self.screen(pane).ok_or(Failure::Target)?;
        let mut point = selection.end;
        match input {
            Input::Key(Key::Left) => point.x = point.x.saturating_sub(1),
            Input::Key(Key::Right) => {
                point.x = point.x.saturating_add(1).min(screen.extent.width() - 1)
            }
            Input::Key(Key::Up) => point.y = point.y.saturating_sub(1),
            Input::Key(Key::Down) => {
                point.y = point.y.saturating_add(1).min(screen.extent.height() - 1)
            }
            _ => return Err(Failure::Unavailable),
        }
        self.select(pane, point, true)
    }

    pub(super) fn copy(&mut self) -> Result<(), Failure> {
        let selection = self.selection.as_ref().ok_or(Failure::Target)?;
        let screen = self.screen(selection.pane).ok_or(Failure::Target)?;
        let (start, end) = selection.range();
        let width = usize::from(screen.extent.width());
        let mut copied = String::new();
        for y in start.y..=end.y {
            let mut first = if y == start.y { start.x } else { 0 };
            if first > 0 && screen.cells[usize::from(y) * width + usize::from(first)].width == 0 {
                first -= 1;
            }
            let last = if y == end.y {
                end.x
            } else {
                screen.extent.width() - 1
            };
            let mut line = String::new();
            for x in first..=last {
                let cell = &screen.cells[usize::from(y) * width + usize::from(x)];
                if cell.width == 0 {
                    continue;
                }
                line.push_str(if cell.text.is_empty() {
                    " "
                } else {
                    &cell.text
                });
            }
            copied.push_str(line.trim_end_matches(' '));
            if y < end.y && !screen.wrapped[usize::from(y)] {
                copied.push('\n');
            }
            if copied.len() > self.copying.limit {
                return Err(Failure::Capacity);
            }
        }
        self.copies.push(&copied, self.copying.outer)?;
        self.clipboard = copied;
        self.live();
        self.status(format!(
            "Copied {} bytes locally{}",
            self.clipboard.len(),
            if self.copying.outer {
                "; OSC52 pending, OS clipboard unverified"
            } else {
                "; use the configured paste binding"
            }
        ));
        Ok(())
    }
}
