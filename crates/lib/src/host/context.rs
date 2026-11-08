use super::Host;
use protocol::{
    Failure, Status,
    wire::{Context, Fault, Message},
};
use workspace::{Id, Target};

impl Host {
    pub(super) fn describe(&mut self, actor: Id, request: u64, pane: Id) -> Result<(), Failure> {
        self.world.watch(actor, Target::Pane(pane))?;
        match self.context(pane) {
            Ok(state) => {
                let message = Message::Context {
                    request,
                    pane,
                    state,
                };
                self.clients[&actor].reserve(&message)?;
                self.reply(actor, request, Ok(Status::Admitted))?;
                self.deliver(actor, message)?;
                self.complete(actor, request, Ok(()))
            }
            Err(fault) => {
                self.reply(actor, request, Ok(Status::Admitted))?;
                self.complete(actor, request, Err(fault))
            }
        }
    }

    fn context(&self, pane: Id) -> Result<Context, Fault> {
        let process = self
            .panes
            .get(&pane)
            .and_then(|pane| pane.process.as_ref())
            .ok_or_else(|| Fault::from(Failure::Unavailable))?;
        let state = process.context().map_err(super::effects::fault)?;
        let cwd = state
            .cwd
            .into_os_string()
            .into_string()
            .map_err(|_| Fault::from(Failure::Payload))?;
        Ok(Context {
            pid: state.pid,
            session: state.session,
            started: state.started,
            command: state.command,
            cwd,
        })
    }
}
