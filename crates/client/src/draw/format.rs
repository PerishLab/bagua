use super::{App, Row, context::Frame, row::tone};
use crate::{
    Header, Line, Status,
    template::{Span, Tone},
};
use std::ops::Range;
use terminal::{Color, Style};
use unicode_width::UnicodeWidthStr;
use workspace::Placement;

pub(super) struct Theme {
    pub base: Style,
    pub(super) active: Style,
    inactive: Style,
}

impl Theme {
    pub fn header(header: &Header, active: bool) -> Self {
        let on = style(header.color, header.active, header.highlight, true);
        let off = style(header.color, header.inactive, header.foreground, false);
        Self {
            base: if active { on } else { off },
            active: on,
            inactive: off,
        }
    }
    pub fn status(status: &Status) -> Self {
        let active = style(status.color, status.active, status.highlight, true);
        let inactive = style(status.color, status.inactive, status.foreground, false);
        Self {
            base: inactive,
            active,
            inactive,
        }
    }
    fn tone(&self, tone: Tone) -> Style {
        match tone {
            Tone::Base => self.base,
            Tone::Active => self.active,
            Tone::Inactive => self.inactive,
        }
    }
}

fn style(color: bool, background: u8, foreground: Option<u8>, active: bool) -> Style {
    let mut value = tone(color, background, active);
    if color && let Some(foreground) = foreground {
        value.foreground = Color::Indexed(foreground);
    }
    value
}

fn width(spans: &[Span]) -> u16 {
    spans
        .iter()
        .map(|s| s.text.width())
        .sum::<usize>()
        .min(u16::MAX as usize) as u16
}

fn put(row: &mut Row, span: Range<u16>, spans: &[Span], theme: &Theme) {
    let mut x = span.start;
    for value in spans {
        x = row.label(x..span.end, &value.text, theme.tone(value.tone));
    }
}

impl App {
    pub(super) fn label(&self, row: &mut Row, place: Placement) -> bool {
        let header = &self.decor.header;
        let Some(template) = &header.template else {
            return false;
        };
        let span = self.span(
            place.frame.x,
            place.frame.x + place.frame.geometry.columns(),
        );
        let theme = Theme::header(header, self.focus() == Some(place.pane));
        if header.border {
            row.label(
                span.clone(),
                &"─".repeat(usize::from(span.end - span.start)),
                tone(false, 0, false),
            );
        } else {
            row.fill(span.clone(), theme.base);
        }
        let mut frame = Frame::new(self, Some(place.pane));
        frame.width = usize::from(span.end - span.start);
        put(row, span, &template.render(&frame), &theme);
        true
    }

    pub(super) fn bars(&self) -> Vec<Row> {
        let Some(layout) = &self.decor.status.layout else {
            return vec![self.bar()];
        };
        let mut rows: Vec<_> = layout.lines.iter().map(|line| self.line(line)).collect();
        let system = self.system();
        if !system.is_empty() {
            let mut row = Row::new(self.viewport.width());
            row.label(
                0..self.viewport.width(),
                &system,
                Theme::status(&self.decor.status).active,
            );
            rows[0] = row;
        }
        rows
    }

    fn line(&self, line: &Line) -> Row {
        let width = self.viewport.width();
        let theme = Theme::status(&self.decor.status);
        let mut frame = Frame::new(self, None);
        let right = line.right.render(&frame);
        let end = width.saturating_sub(self::width(&right));
        frame.width = usize::from(end);
        let center = line.center.render(&frame);
        let start = (width.saturating_sub(self::width(&center)) / 2)
            .min(end.saturating_sub(self::width(&center)));
        let limit = if center.is_empty() { end } else { start };
        frame.width = usize::from(limit);
        let left = line.left.render(&frame);
        let mut row = Row::new(width);
        if line.fill {
            row.fill(0..width, theme.base);
        }
        put(&mut row, 0..limit, &left, &theme);
        put(&mut row, start..end, &center, &theme);
        put(&mut row, end..width, &right, &theme);
        row
    }

    fn system(&self) -> String {
        let mut values = Vec::new();
        let mode = self.mode();
        if !self.controller() && !mode.starts_with("Observe") {
            values.push("Observe".into());
        }
        if mode != "Live" {
            values.push(mode);
        }
        if let Some(prefix) = &self.leader {
            values.push(format!("{prefix}: prefix pending"));
        }
        if !self.notice.is_empty() {
            values.push(self.notice.clone());
        }
        values.join(" | ")
    }
}
