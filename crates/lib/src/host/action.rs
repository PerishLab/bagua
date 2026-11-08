use super::{Host, pane::Pane};
use protocol::{
    Command, Failure, Outcome, Reply, Request, Status,
    wire::{Fault, Message, Program},
};
use workspace::{Id, State, Target};

impl Host {
    pub(super) fn apply(&mut self, actor: Id, request: Request) -> Result<(), Failure> {
        let id = request.id;
        let command = request.check()?;
        if self.stopping.is_some() {
            return Err(Failure::Pending);
        }
        match command {
            Command::Capabilities => {
                self.reply(
                    actor,
                    id,
                    Ok(Status::Applied(Outcome::Profile(crate::profile()))),
                )?;
                self.complete(actor, id, Ok(()))
            }
            Command::Workspace(operation) => self.operation(actor, id, operation),
            Command::Start { pane, program } => self.start(actor, id, (pane, program)),
            Command::Send { pane, input } => self.send(actor, id, (pane, input)),
            Command::Inspect(pane) => self.inspect(actor, id, pane),
            Command::Describe(pane) => self.describe(actor, id, pane),
            Command::History { pane, read } => self.history(actor, id, (pane, read)),
            Command::Subscribe { target, after: _ } => self.subscribe(actor, id, target),
            Command::Unsubscribe(target) => {
                self.clients
                    .get_mut(&actor)
                    .unwrap()
                    .watches
                    .unsubscribe(target);
                self.reply(actor, id, Ok(Status::Admitted))?;
                self.complete(actor, id, Ok(()))
            }
            Command::Shutdown => self.shutdown(actor, id),
        }
    }

    fn start(&mut self, actor: Id, request: u64, data: (Id, Program)) -> Result<(), Failure> {
        let (id, program) = data;
        let (session, _) = self.world.location(Target::Pane(id))?;
        self.world.session(session).unwrap().require(actor)?;
        if self.world.pane(id).unwrap().state() != State::Pending {
            return Err(Failure::Pending);
        }
        let extent = self.extent(id)?;
        if let std::collections::btree_map::Entry::Vacant(entry) = self.panes.entry(id) {
            entry.insert(Pane::new(extent, self.history)?);
        }
        self.reply(actor, request, Ok(Status::Admitted))?;
        let size = pty::Size::new(extent.width(), extent.height()).unwrap();
        let mut launch = pty::Launch::new(
            program.executable.into(),
            program.cwd.into(),
            radicula_process::Environment::new(
                program
                    .environment
                    .into_iter()
                    .map(|(k, v)| (k.into(), v.into())),
            ),
            size,
        );
        launch.args = program.args.into_iter().map(Into::into).collect();
        match pty::Process::spawn(launch) {
            Ok(process) => {
                self.panes.get_mut(&id).unwrap().process = Some(process);
                self.world.observe(id, State::Running)?;
                self.complete(actor, request, Ok(()))
            }
            Err(error) => {
                let fault = super::effects::fault(error);
                self.panes.get_mut(&id).unwrap().fault = Some(fault.clone());
                self.world.observe(id, State::Failed)?;
                self.complete(actor, request, Err(fault))
            }
        }
    }

    fn inspect(&mut self, actor: Id, request: u64, pane: Id) -> Result<(), Failure> {
        self.world.watch(actor, Target::Pane(pane))?;
        let client = self.clients.get_mut(&actor).unwrap();
        if client.wanted.contains_key(&pane) {
            return Err(Failure::Pending);
        }
        if client.wanted.len() >= super::PANES {
            return Err(Failure::Capacity);
        }
        client.wanted.insert(pane, request);
        self.reply(actor, request, Ok(Status::Admitted))
    }

    fn subscribe(&mut self, actor: Id, request: u64, target: Target) -> Result<(), Failure> {
        self.world.watch(actor, target)?;
        let client = self.clients.get_mut(&actor).unwrap();
        client.watches.subscribe(target)?;
        client.seen.clear();
        client.catalog = u64::MAX;
        self.reply(actor, request, Ok(Status::Admitted))?;
        self.deliver(
            actor,
            Message::Event(protocol::Event::Subscribed {
                request,
                target,
                revision: match target {
                    Target::Pane(id) => self.panes[&id].engine.revision(),
                    _ => terminal::Revision::new(self.world.revision()),
                },
            }),
        )?;
        self.complete(actor, request, Ok(()))
    }

    pub(super) fn reply(
        &mut self,
        actor: Id,
        request: u64,
        result: Result<Status, Failure>,
    ) -> Result<(), Failure> {
        self.deliver(actor, Message::Reply(Reply { request, result }))
    }

    pub(super) fn complete(
        &mut self,
        actor: Id,
        request: u64,
        result: Result<(), Fault>,
    ) -> Result<(), Failure> {
        self.deliver(actor, Message::Finished { request, result })
    }

    pub(super) fn deliver(&mut self, actor: Id, message: Message) -> Result<(), Failure> {
        let Some(client) = self.clients.get_mut(&actor) else {
            return Ok(());
        };
        if client.push(message).is_err() {
            self.disconnect(actor)?;
        }
        Ok(())
    }
}
