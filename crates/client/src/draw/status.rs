use super::{App, Component, Row, tone};
use terminal::Style;

impl App {
    pub(super) fn bar(&self) -> Row {
        let width = self.viewport.width();
        let status = &self.decor.status;
        let style = tone(status.color, status.inactive, false);
        let active = tone(status.color, status.active, true);
        let mut row = Row::new(width);
        row.fill(0..width, style);
        let mode = self.mode();
        let priority = self.urgent || self.leader.is_some() || mode != "Live";
        let clock = !matches!(
            (
                width,
                priority,
                status.components.contains(&Component::Clock)
            ),
            (_, _, false) | (0..=11, _, _) | (12..=63, true, _)
        );
        let end = if clock { width - 6 } else { width };
        let mut x = 0;
        if mode != "Live" {
            x = row.label(x..end, &format!("{mode} | "), active);
        }
        if let Some(prefix) = &self.leader
            && status.components.contains(&Component::Prefix)
        {
            x = row.label(x..end, &format!("{prefix}: prefix pending | "), active);
        }
        if self.urgent {
            x = row.label(x..end, &format!("{} | ", self.notice), active);
        }
        let role = if self.controller() {
            "control"
        } else {
            "observe"
        };
        x = row.label(x..end, &format!("bagua {} {role}", self.caption()), style);
        if !self.urgent && !self.notice.is_empty() && status.components.contains(&Component::Notice)
        {
            x = row.label(x..end, &format!(" | {}", self.notice), style);
        }
        for component in &status.components {
            match component {
                Component::Session => {
                    let text = self.session().map_or("closed", |session| session.name());
                    x = row.label(x..end, &format!(" | {text}"), style);
                }
                Component::Windows => x = self.windows(&mut row, (x, end), (style, active)),
                _ => {}
            }
        }
        if clock {
            row.label(width - 5..width, &self.clock, style);
        }
        row
    }

    fn windows(&self, row: &mut Row, span: (u16, u16), styles: (Style, Style)) -> u16 {
        let (mut x, end) = span;
        let Some(session) = self.session() else {
            return x;
        };
        x = row.label(x..end, " | ", styles.0);
        let labels: Vec<_> = session
            .windows()
            .enumerate()
            .map(|(index, window)| {
                let selected = Some(window.id()) == session.selected();
                let text = if selected {
                    format!("[{}:{}] ", index + 1, window.name())
                } else {
                    format!("{}:{} ", index + 1, window.name())
                };
                (text, selected)
            })
            .collect();
        let range = visible(&labels, end.saturating_sub(x));
        if range.start > 0 {
            x = row.label(x..end, "< ", styles.0);
        }
        for (text, selected) in &labels[range.clone()] {
            x = row.label(x..end, text, if *selected { styles.1 } else { styles.0 });
        }
        if range.end < labels.len() {
            x = row.label(x..end, ">", styles.0);
        }

        x
    }

    fn caption(&self) -> String {
        let Some(session) = self.session() else {
            return "closed".into();
        };
        match session
            .windows()
            .enumerate()
            .find(|(_, window)| Some(window.id()) == session.selected())
        {
            Some((index, _)) => format!("w{}", index + 1),
            None => "empty".into(),
        }
    }
}

fn visible(labels: &[(String, bool)], available: u16) -> std::ops::Range<usize> {
    let Some(selected) = labels.iter().position(|(_, selected)| *selected) else {
        return 0..0;
    };
    let budget = usize::from(available.saturating_sub(4));
    let width = |index: usize| unicode_width::UnicodeWidthStr::width(labels[index].0.as_str());
    let mut start = selected;
    let mut end = selected + 1;
    let mut used = width(selected).min(budget);
    while start > 0 {
        let next = width(start - 1);
        if used + next > budget {
            break;
        }
        start -= 1;
        used += next;
    }
    while end < labels.len() {
        let next = width(end);
        if used + next > budget {
            break;
        }
        end += 1;
        used += next;
    }
    start..end
}
