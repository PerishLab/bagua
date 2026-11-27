use super::{Host, client::Client};
use protocol::{
    Failure,
    wire::{Message, Screen},
};
use workspace::{Id, Target};

impl Host {
    pub(super) fn announce(&mut self, pane: Id, message: Message) -> Result<(), Failure> {
        let actors: Vec<_> = self
            .clients
            .keys()
            .copied()
            .filter(|actor| self.watching(*actor, pane))
            .collect();
        for actor in actors {
            self.deliver(actor, message.clone())?;
        }
        Ok(())
    }

    fn watching(&self, actor: Id, pane: Id) -> bool {
        let Some(client) = self.clients.get(&actor) else {
            return false;
        };
        let Ok((session, window)) = self.world.location(Target::Pane(pane)) else {
            return false;
        };
        if self.world.watch(actor, Target::Pane(pane)).is_err() {
            return false;
        }
        client.watches.contains(Target::Pane(pane))
            || client.watches.contains(Target::Session(session))
            || window.is_some_and(|id| client.watches.contains(Target::Window(id)))
    }

    pub(super) fn publish(&mut self) -> Result<(), Failure> {
        for client in self.clients.values_mut() {
            client.seen.retain(|id, _| self.world.pane(*id).is_some());
        }
        let actors: Vec<_> = self.clients.keys().copied().collect();
        if actors.is_empty() {
            return Ok(());
        }
        let count = actors.len();
        let mut budget = 2;
        for offset in 0..count {
            let actor = actors[(self.turn + offset) % count];
            if !self.clients.get(&actor).is_some_and(Client::available) {
                continue;
            }
            let shown = self.presentation(actor)?;
            if shown {
                if let Some(client) = self.clients.get_mut(&actor) {
                    client.catalogue = !client.catalogue;
                }
                budget -= 1;
            } else if let Some(client) = self.clients.get_mut(&actor) {
                client.resume();
            }
            if budget == 0 {
                break;
            }
        }
        self.turn = (self.turn + 1) % count;
        Ok(())
    }

    fn presentation(&mut self, actor: Id) -> Result<bool, Failure> {
        if self.past(actor)? {
            return Ok(true);
        }
        let preferred = self.clients[&actor].catalogue;
        for catalog in [preferred, !preferred] {
            let shown = if catalog {
                self.catalog(actor)?
            } else {
                self.view(actor)?
            };
            if shown {
                return Ok(true);
            }
        }
        Ok(false)
    }

    fn catalog(&mut self, actor: Id) -> Result<bool, Failure> {
        let client = self.clients.get(&actor).unwrap();
        if client.watches.empty() || client.catalog == self.world.revision() {
            return Ok(false);
        }
        let message = Message::Catalog(self.world.snapshot());
        let client = self.clients.get_mut(&actor).unwrap();
        if client.present(message)? {
            client.catalog = self.world.revision();
            return Ok(true);
        }
        Ok(false)
    }

    fn view(&mut self, actor: Id) -> Result<bool, Failure> {
        let missing: Vec<_> = self.clients[&actor]
            .wanted
            .keys()
            .copied()
            .filter(|id| self.world.watch(actor, Target::Pane(*id)).is_err())
            .collect();
        for id in missing {
            let request = self
                .clients
                .get_mut(&actor)
                .unwrap()
                .wanted
                .remove(&id)
                .unwrap();
            self.complete(actor, request, Err(Failure::Target.into()))?;
        }
        let client = &self.clients[&actor];
        let mut candidates: Vec<_> = self
            .panes
            .iter()
            .filter(|(id, pane)| {
                client.wanted.contains_key(id)
                    || (self.watching(actor, **id) && pane.dirty(client.seen.get(id)))
            })
            .map(|(id, _)| *id)
            .collect();
        candidates.sort_by_key(|id| (client.cursor.is_some_and(|cursor| *id <= cursor), *id));
        let Some(id) = candidates.first().copied() else {
            return Ok(false);
        };
        let mut state = Screen::from(&self.panes[&id].engine.snapshot());
        state.bytes = self.panes[&id].bytes;
        state.unsupported = self.panes[&id].unsupported;
        state.bells = self.panes[&id].bells;
        let revision = state.revision;
        let client = self.clients.get_mut(&actor).unwrap();
        if !client.present(Message::Screen { pane: id, state })? {
            return Ok(false);
        }
        client.cursor = Some(id);
        client.seen.insert(id, revision);
        let request = client.wanted.remove(&id);
        self.deliver(actor, self.panes[&id].observation(id))?;
        if let Some(request) = request {
            self.complete(actor, request, Ok(()))?;
        }
        Ok(true)
    }
}

#[cfg(test)]
mod checks {
    use super::*;
    use protocol::{Command, Request};

    fn drain(host: &mut Host, actor: Id) -> Vec<Message> {
        let mut out = Vec::new();
        while let Some(bytes) = host.output(actor) {
            let len = bytes.len();
            out.push(serde_json::from_slice(&bytes[4..]).unwrap());
            host.consume(actor, len);
        }
        out
    }

    #[test]
    fn synchronized() {
        let mut host = Host::new();
        let actor = host.connect().unwrap();
        let pane = Id::new(3).unwrap();
        for (id, command) in [
            (1, Command::create(80, 24).unwrap()),
            (
                2,
                Command::Workspace(workspace::Operation::Join {
                    session: Id::new(1).unwrap(),
                    viewport: workspace::Geometry::new(80, 24).unwrap(),
                }),
            ),
            (
                3,
                Command::Subscribe {
                    target: Target::Pane(pane),
                    after: None,
                },
            ),
        ] {
            host.submit(actor, Request::identified(id, command).unwrap())
                .unwrap();
        }
        host.tick().unwrap();
        drain(&mut host, actor);
        let target = host.panes.get_mut(&pane).unwrap();
        target.engine.feed(b"\x1b[?2026hpartial").unwrap();
        target.held = 1;
        host.tick().unwrap();
        assert!(
            !drain(&mut host, actor)
                .iter()
                .any(|m| matches!(m, Message::Screen { .. }))
        );
        let target = host.panes.get_mut(&pane).unwrap();
        target.engine.feed(b" complete\x1b[?2026l").unwrap();
        target.held = 0;
        host.tick().unwrap();
        assert!(
            drain(&mut host, actor)
                .iter()
                .any(|m| matches!(m, Message::Screen { .. }))
        );
        let target = host.panes.get_mut(&pane).unwrap();
        target.engine.feed(b"\x1b[?2026hstuck").unwrap();
        target.held = 9;
        host.tick().unwrap();
        assert!(
            drain(&mut host, actor)
                .iter()
                .any(|m| matches!(m, Message::Screen { .. }))
        );
    }
}
