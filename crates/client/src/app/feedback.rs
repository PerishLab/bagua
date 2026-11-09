use super::App;
use std::time::Duration;

impl App {
    pub fn status(&mut self, value: impl Into<String>) {
        self.notice = value
            .into()
            .chars()
            .map(|c| if c.is_control() { '?' } else { c })
            .take(512)
            .collect();
        self.expires = self.now.checked_add(Duration::from_secs(2));
        self.dirty = true;
        self.urgent = false;
    }
    pub fn error(&mut self, value: impl Into<String>) {
        self.status(value);
        self.urgent = true;
    }

    pub fn recovering(&mut self, value: bool) {
        if self.recovering != value {
            self.recovering = value;
            self.dirty = true;
        }
    }

    pub(crate) fn mode(&self) -> String {
        if self.recovering {
            return "Recovering".into();
        }
        let copy = self.selection.is_some() || self.past.active();
        let role = match (self.controller(), copy) {
            (false, true) => "Observe Copy",
            (false, false) => "Observe",
            (true, true) => "Copy",
            (true, false) => "Live",
        };
        if let Some(view) = &self.past.view {
            return format!("{role} {} / {}", view.top, view.next);
        }
        if let Some(selection) = &self.selection {
            return format!("{role} {},{}", selection.end.x + 1, selection.end.y + 1);
        }
        role.into()
    }
}
