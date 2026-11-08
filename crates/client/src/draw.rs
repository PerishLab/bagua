use crate::App;
use protocol::Failure;
use std::fmt::Write;
use terminal::{Color, Point, Style};
use unicode_width::UnicodeWidthStr;

impl App {
    pub fn draw(&self) -> Result<Vec<u8>, Failure> {
        let mut output = String::from("\x1b[?25l\x1b[0m\x1b[2J");
        for placement in self.placements()? {
            self.border(&mut output, placement);
            let Some(screen) = self.screen(placement.pane) else {
                continue;
            };
            let extent = screen.extent;
            let mut previous = None;
            for y in 0..extent.height() {
                for x in 0..extent.width() {
                    self.cell(&mut output, &mut previous, (placement, Point { x, y }))?;
                }
            }
        }
        let role = if self.controller() {
            "control"
        } else {
            "observe"
        };
        let text = format!(
            "bagua {} {role} | {} | {}",
            self.caption(),
            self.notice,
            self.names()
        );
        write!(
            output,
            "\x1b[0m\x1b[{};1H\x1b[7m{}\x1b[0m",
            self.viewport.height() + 1,
            label(&text, self.viewport.width())
        )
        .unwrap();
        self.cursor(&mut output)?;
        if output.len() > 4_194_304 {
            return Err(Failure::Capacity);
        }
        Ok(output.into_bytes())
    }

    fn caption(&self) -> String {
        let Some(session) = self.session() else {
            return "closed".into();
        };
        let selected = session
            .windows()
            .enumerate()
            .find(|(_, window)| Some(window.id()) == session.selected());
        match selected {
            Some((index, _)) => format!("w{}", index + 1),
            None => "empty".into(),
        }
    }

    fn names(&self) -> String {
        let Some(session) = self.session() else {
            return String::new();
        };
        match session
            .windows()
            .find(|window| Some(window.id()) == session.selected())
        {
            Some(window) => format!("{}/{}", session.name(), window.name()),
            None => session.name().into(),
        }
    }

    fn border(&self, output: &mut String, place: workspace::Placement) {
        let Some(session) = self.session() else {
            return;
        };
        let right = place.rect.x + place.rect.geometry.columns();
        let bottom = place.rect.y + place.rect.geometry.rows();
        if right < session.geometry().columns() {
            for y in place.rect.y..bottom {
                self.mark(output, Point { x: right, y }, '|');
            }
        }
        if bottom < session.geometry().rows() {
            for x in place.rect.x..right {
                self.mark(output, Point { x, y: bottom }, '-');
            }
        }
    }

    fn mark(&self, output: &mut String, point: Point, value: char) {
        let Some(x) = point.x.checked_sub(self.offset.x) else {
            return;
        };
        let Some(y) = point.y.checked_sub(self.offset.y) else {
            return;
        };
        if x < self.viewport.width() && y < self.viewport.height() {
            write!(output, "\x1b[0m\x1b[{};{}H{value}", y + 1, x + 1).unwrap();
        }
    }

    fn cell(
        &self,
        output: &mut String,
        previous: &mut Option<Style>,
        data: (workspace::Placement, Point),
    ) -> Result<(), Failure> {
        let (place, point) = data;
        let Some(x) = (place.rect.x + point.x).checked_sub(self.offset.x) else {
            return Ok(());
        };
        let Some(y) = (place.rect.y + point.y).checked_sub(self.offset.y) else {
            return Ok(());
        };
        if x >= self.viewport.width() || y >= self.viewport.height() {
            return Ok(());
        }
        let screen = self.screen(place.pane).ok_or(Failure::Target)?;
        let cell = &screen.cells
            [usize::from(point.y) * usize::from(screen.extent.width()) + usize::from(point.x)];
        if cell.width == 0 && x > 0 {
            return Ok(());
        }
        let selected = self
            .selection
            .as_ref()
            .is_some_and(|s| s.contains(place.pane, point));
        let mut style = cell.style;
        style.inverse ^= selected;
        if *previous != Some(style) {
            sgr(output, style);
            *previous = Some(style);
        }
        write!(output, "\x1b[{};{}H", y + 1, x + 1).unwrap();
        let text = if cell.text.is_empty() {
            " "
        } else {
            &cell.text
        };
        let available = self.viewport.width() - x;
        match (UnicodeWidthStr::width(text), cell.width, available) {
            (width, declared, room)
                if width == usize::from(declared) && u16::from(declared) <= room =>
            {
                output.push_str(text)
            }
            _ => output.push(' '),
        }
        Ok(())
    }

    fn cursor(&self, output: &mut String) -> Result<(), Failure> {
        let Some(focus) = self.focus() else {
            return Ok(());
        };
        let Some(screen) = self.screen(focus) else {
            return Ok(());
        };
        if !screen.cursor.visible || self.selection.is_some() {
            return Ok(());
        }
        let Some(place) = self.placements()?.into_iter().find(|p| p.pane == focus) else {
            return Ok(());
        };
        let point = screen.cursor.position;
        let Some(x) = (place.rect.x + point.x).checked_sub(self.offset.x) else {
            return Ok(());
        };
        let Some(y) = (place.rect.y + point.y).checked_sub(self.offset.y) else {
            return Ok(());
        };
        if x < self.viewport.width() && y < self.viewport.height() {
            write!(output, "\x1b[{};{}H\x1b[?25h", y + 1, x + 1).unwrap();
        }
        Ok(())
    }
}

fn sgr(output: &mut String, style: Style) {
    output.push_str("\x1b[0m");
    for (enabled, number) in [
        (style.bold, 1),
        (style.dim, 2),
        (style.italic, 3),
        (style.underline, 4),
        (style.inverse, 7),
    ] {
        if enabled {
            write!(output, "\x1b[{number}m").unwrap();
        }
    }
    color(output, style.foreground, false);
    color(output, style.background, true);
}

fn color(output: &mut String, color: Color, background: bool) {
    let prefix = if background { 48 } else { 38 };
    match color {
        Color::Default => {}
        Color::Indexed(index) => write!(output, "\x1b[{prefix};5;{index}m").unwrap(),
        Color::Rgb(r, g, b) => write!(output, "\x1b[{prefix};2;{r};{g};{b}m").unwrap(),
    }
}

fn label(text: &str, width: u16) -> String {
    let mut result = String::new();
    let mut used = 0;
    for c in text.chars() {
        let c = if c.is_control() { '?' } else { c };
        let size = unicode_width::UnicodeWidthChar::width(c).unwrap_or(0);
        if used + size > usize::from(width) {
            break;
        }
        result.push(c);
        used += size;
    }
    result
}
