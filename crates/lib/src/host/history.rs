use super::Host;
use protocol::{
    Failure, Status,
    wire::{History, Message, Read, Scope, Screen},
};
use workspace::{Id, Target};

#[cfg(test)]
mod tests;

impl Host {
    pub(super) fn budget(&mut self) {
        let share = self.history.total / self.panes.len().max(1);
        for pane in self.panes.values_mut() {
            pane.engine.budget(share.min(self.history.bytes));
        }
    }

    pub(super) fn history(
        &mut self,
        actor: Id,
        request: u64,
        data: (Id, Read),
    ) -> Result<(), Failure> {
        let (pane, read) = data;
        self.world.watch(actor, Target::Pane(pane))?;
        if self.clients[&actor].history.is_some() {
            return Err(Failure::Pending);
        }
        self.check(pane, &read)?;
        self.clients.get_mut(&actor).unwrap().history = Some((request, pane, read));
        self.reply(actor, request, Ok(Status::Admitted))
    }

    fn scope(&self, pane: Id) -> Result<Scope, Failure> {
        let engine = &self.panes.get(&pane).ok_or(Failure::Target)?.engine;
        Ok(Scope {
            host: self.scope,
            pane,
            epoch: engine.epoch(),
        })
    }

    fn check(&self, pane: Id, read: &Read) -> Result<u64, Failure> {
        let scope = self.scope(pane)?;
        if read.scope.is_some_and(|given| given != scope)
            || read.top.is_some() != read.scope.is_some()
        {
            return Err(Failure::Baseline);
        }
        let engine = &self.panes[&pane].engine;
        if engine.modes().alternate {
            return Err(Failure::Unavailable);
        }
        let range = engine.history().range();
        if read.back > terminal::ROWS || (read.top.is_some() && read.back != 0) {
            return Err(Failure::Payload);
        }
        let back = if read.back == 0 {
            engine.extent().height()
        } else {
            read.back
        };
        let top = read
            .top
            .unwrap_or_else(|| range.next.saturating_sub(u64::from(back)).max(range.first));
        if top < range.first || top >= range.next {
            return Err(Failure::Baseline);
        }
        if u32::from(read.column) + u32::from(engine.extent().width())
            > u32::from(terminal::COLUMNS)
        {
            return Err(Failure::Payload);
        }
        Ok(top)
    }

    pub(super) fn past(&mut self, actor: Id) -> Result<bool, Failure> {
        let Some((request, pane, read)) = self.clients[&actor].history.clone() else {
            return Ok(false);
        };
        let result = self
            .world
            .watch(actor, Target::Pane(pane))
            .map_err(Failure::from)
            .and_then(|_| self.page(pane, &read));
        self.clients.get_mut(&actor).unwrap().history = None;
        match result {
            Ok(state) => {
                self.clients
                    .get_mut(&actor)
                    .unwrap()
                    .present(Message::History { request, state })?;
                self.complete(actor, request, Ok(()))?;
            }
            Err(error) => self.complete(actor, request, Err(error.into()))?,
        }
        Ok(true)
    }

    fn page(&self, pane: Id, read: &Read) -> Result<History, Failure> {
        let top = self.check(pane, read)?;
        let engine = &self.panes[&pane].engine;
        let range = engine.history().range();
        let view = engine
            .snapshot()
            .window((range.next - top) as usize, read.column)
            .map_err(|_| Failure::Payload)?;
        Ok(History {
            ticket: read.ticket,
            scope: self.scope(pane)?,
            first: range.first,
            next: range.next,
            top,
            column: read.column,
            bytes: range.bytes,
            evicted: range.evicted,
            screen: Screen::from(&view),
        })
    }
}
