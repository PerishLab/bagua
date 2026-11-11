use super::App;
use protocol::{Failure, wire::Context};
use std::time::Instant;
use workspace::{Id, State};

pub(crate) struct Record {
    pub state: Option<Context>,
    pub at: Instant,
}

impl App {
    pub fn context(
        &mut self,
        pane: Id,
        state: Option<Context>,
        at: Instant,
    ) -> Result<(), Failure> {
        if let Some(state) = &state
            && (state.command.len() > 256 || state.cwd.len() > 4096)
        {
            return Err(Failure::Capacity);
        }
        if self.running(pane) {
            self.records.insert(pane, Record { state, at });
            self.dirty = true;
        }
        Ok(())
    }

    pub fn needing(&self) -> Vec<Id> {
        let header = self
            .decor
            .header
            .template
            .as_ref()
            .is_some_and(|t| t.metadata())
            || self.decor.header.components.iter().any(|c| {
                matches!(
                    c,
                    super::preferences::Component::Command | super::preferences::Component::Cwd
                )
            });
        let status = self
            .decor
            .status
            .layout
            .as_ref()
            .is_some_and(|layout| layout.lines.iter().any(|line| line.metadata()));
        if !(status || header && self.session().is_some_and(|s| s.header())) {
            return Vec::new();
        }
        self.placements()
            .unwrap_or_default()
            .into_iter()
            .map(|place| place.pane)
            .filter(|pane| self.running(*pane))
            .filter(|pane| {
                self.records.get(pane).is_none_or(|record| {
                    self.now.saturating_duration_since(record.at) >= self.decor.status.refresh
                })
            })
            .collect()
    }

    pub(crate) fn running(&self, pane: Id) -> bool {
        self.session()
            .into_iter()
            .flat_map(|s| s.windows())
            .flat_map(|w| w.panes())
            .any(|p| p.id() == pane && p.state() == State::Running)
    }

    pub(crate) fn retain(&mut self) {
        let ids: Vec<_> = self
            .records
            .keys()
            .copied()
            .filter(|id| !self.running(*id))
            .collect();
        for id in ids {
            self.records.remove(&id);
        }
    }

    pub(crate) fn observed(&self, pane: Id) -> Option<&Context> {
        if !self.running(pane) {
            return None;
        }
        let record = self.records.get(&pane)?;
        if self.now.saturating_duration_since(record.at)
            >= self.decor.status.refresh.saturating_mul(2)
        {
            return None;
        }
        record.state.as_ref()
    }
}
