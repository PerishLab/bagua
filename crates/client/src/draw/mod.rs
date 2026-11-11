#[cfg(test)]
mod checks;
mod context;
mod format;
mod paint;
mod row;
mod status;
use crate::{App, app::preferences::Component};
pub use paint::Paint;
use protocol::Failure;
use row::{Row, tone};
use terminal::{Extent, Point};
use workspace::Placement;

impl App {
    pub fn draw(&self) -> Result<Vec<u8>, Failure> {
        Ok(self.paint()?.render(None))
    }

    pub fn paint(&self) -> Result<Paint, Failure> {
        let mut body = Vec::new();
        let places = self.placements()?;
        for y in 0..self.viewport.height() {
            let mut row = Row::new(self.viewport.width());
            let world = y + self.offset.y;
            for place in &places {
                self.border(&mut row, *place, world);
                self.content(&mut row, *place, world);
                self.header(&mut row, *place, world);
            }
            body.push(row);
        }
        let mut bars = self.bars();
        if self.decor.status.separator {
            let mut row = Row::new(self.viewport.width());
            row.label(
                0..self.viewport.width(),
                &"─".repeat(usize::from(self.viewport.width())),
                tone(false, 0, false),
            );
            if self.decor.status.inset() > 0 {
                bars.push(row);
            } else {
                bars.insert(0, row);
            }
        }
        let rows = if self.decor.status.inset() > 0 {
            bars.extend(body);
            bars
        } else {
            body.extend(bars);
            body
        };
        let extent =
            Extent::new(self.viewport.width(), rows.len() as u16).ok_or(Failure::Capacity)?;
        let mut cursor = self.cursor()?;
        if let Some(point) = &mut cursor {
            point.y += self.decor.status.inset();
        }
        let paint = Paint {
            tracking: self.tracking(),
            rows: rows
                .iter()
                .enumerate()
                .map(|(y, row)| row.encode(y as u16))
                .collect(),
            cursor,
            extent,
        };
        paint.check()?;
        Ok(paint)
    }

    fn span(&self, start: u16, end: u16) -> std::ops::Range<u16> {
        start
            .saturating_sub(self.offset.x)
            .min(self.viewport.width())
            ..end.saturating_sub(self.offset.x).min(self.viewport.width())
    }

    fn content(&self, row: &mut Row, place: Placement, world: u16) {
        let Some(y) = world.checked_sub(place.rect.y) else {
            return;
        };
        if y >= place.rect.geometry.rows() {
            return;
        }
        let Some(screen) = self.screen(place.pane) else {
            return;
        };
        for x in 0..screen.extent.width() {
            let Some(local) = (place.rect.x + x).checked_sub(self.offset.x) else {
                continue;
            };
            if local >= self.viewport.width() {
                continue;
            }
            let mut cell = screen.cells
                [usize::from(y) * usize::from(screen.extent.width()) + usize::from(x)]
            .clone();
            let selected = self.selection.as_ref().is_some_and(|selection| {
                selection.contains(place.pane, Point { x, y })
                    || (cell.width == 2 && selection.contains(place.pane, Point { x: x + 1, y }))
            });
            cell.style.inverse ^= selected;
            row.put(local, cell);
        }
    }

    fn border(&self, row: &mut Row, place: Placement, world: u16) {
        let Some(session) = self.session() else {
            return;
        };
        let rect = place.frame;
        let right = rect.x + rect.geometry.columns();
        let bottom = rect.y + rect.geometry.rows();
        let style = tone(false, 0, false);
        if right < session.geometry().columns() && (rect.y..bottom).contains(&world) {
            let span = self.span(right, right + 1);
            row.label(span, "|", style);
        }
        if bottom < session.geometry().rows() && world == bottom {
            let span = self.span(rect.x, right);
            row.label(
                span.clone(),
                &"-".repeat(usize::from(span.end - span.start)),
                style,
            );
        }
    }

    fn header(&self, row: &mut Row, place: Placement, world: u16) {
        if !self.session().is_some_and(|session| session.header()) || world != heading(place) {
            return;
        }
        if self.label(row, place) {
            return;
        }
        let active = self.focus() == Some(place.pane);
        let decor = &self.decor.header;
        let style = format::Theme::header(decor, active).base;
        let text: Vec<_> = decor
            .components
            .iter()
            .map(|component| match component {
                Component::Pane => format!("p{} {}", place.pane.value(), self.activity(place.pane)),
                Component::Command => self
                    .observed(place.pane)
                    .map_or_else(|| "cmd?".into(), |state| format!("cmd~{}", state.command)),
                Component::Cwd => self
                    .observed(place.pane)
                    .map_or_else(|| "cwd?".into(), |state| format!("cwd~{}", state.cwd)),
                _ => String::new(),
            })
            .collect();
        let span = self.span(
            place.frame.x,
            place.frame.x + place.frame.geometry.columns(),
        );
        if decor.border {
            row.label(
                span.clone(),
                &"─".repeat(usize::from(span.end - span.start)),
                tone(false, 0, false),
            );
        } else {
            row.fill(span.clone(), style);
        }
        row.label(span, &text.join(" | "), style);
    }

    pub(super) fn activity(&self, pane: workspace::Id) -> &'static str {
        let state = self
            .session()
            .into_iter()
            .flat_map(|s| s.windows())
            .flat_map(|w| w.panes())
            .find(|p| p.id() == pane)
            .map(|p| p.state());
        match state {
            Some(workspace::State::Running) => "run",
            Some(workspace::State::Pending) => "pending",
            Some(workspace::State::Failed) => "failed",
            Some(workspace::State::Exited(_)) => "exited",
            None => "?",
        }
    }

    fn cursor(&self) -> Result<Option<Point>, Failure> {
        let Some(focus) = self.focus() else {
            return Ok(None);
        };
        let Some(screen) = self.screen(focus) else {
            return Ok(None);
        };
        if !screen.cursor.visible || self.selection.is_some() || self.history(focus).is_some() {
            return Ok(None);
        }
        let Some(place) = self.placements()?.into_iter().find(|p| p.pane == focus) else {
            return Ok(None);
        };
        let point = screen.cursor.position;
        let Some(x) = (place.rect.x + point.x).checked_sub(self.offset.x) else {
            return Ok(None);
        };
        let Some(y) = (place.rect.y + point.y).checked_sub(self.offset.y) else {
            return Ok(None);
        };
        Ok((x < self.viewport.width() && y < self.viewport.height()).then_some(Point { x, y }))
    }
}

fn heading(place: Placement) -> u16 {
    if place.rect.y > place.frame.y {
        place.frame.y
    } else {
        place.frame.y + place.frame.geometry.rows() - 1
    }
}
