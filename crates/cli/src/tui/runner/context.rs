use super::{Job, Runner};
use crate::{
    error::Error,
    tui::queue::{Queued, weight},
};
use protocol::{Command, Failure, wire::Context};
use workspace::{Id, Operation, Target};

impl Runner {
    pub(super) fn dispatch(&mut self, queued: Queued) -> Result<bool, Error> {
        if !matches!(
            queued.command,
            Command::Workspace(Operation::Split { .. } | Operation::Window(_))
        ) {
            return self.submit(queued.command, Job::Ordinary);
        }
        let prepared = queued.program.is_some();
        let program = queued
            .program
            .or_else(|| self.program.clone())
            .ok_or(Failure::Payload)?;
        let resolve = self.config.workspace().cwd == "foreground";
        if resolve
            && !matches!(queued.command, Command::Workspace(Operation::Window(_)))
            && queued.source.is_none()
        {
            self.app
                .status("No current pane directory; operation refused");
            return Ok(true);
        }
        if resolve
            && !prepared
            && let Some(pane) = queued.source
        {
            if self
                .jobs
                .values()
                .any(|job| matches!(job, Job::Resolve { .. }))
            {
                return Ok(false);
            }
            self.submit(
                Command::Describe(pane),
                Job::Resolve {
                    pane,
                    command: queued.command,
                    program,
                },
            )?;
            return Ok(false);
        }
        if resolve && queued.source.is_none() {
            if self
                .app
                .session()
                .is_none_or(|session| session.windows().next().is_some())
            {
                self.app
                    .status("No current pane directory; operation refused");
                return Ok(true);
            }
            self.app
                .status("Empty workspace: using the explicit launch directory");
        }
        self.submit(queued.command, Job::Spawn(program))
    }

    pub(super) fn described(
        &mut self,
        request: u64,
        pane: Id,
        state: &Context,
    ) -> Result<(), Error> {
        let Some(Job::Resolve {
            pane: expected,
            command,
            program,
        }) = self.jobs.get(&request)
        else {
            return Err(Failure::Identity.into());
        };
        if *expected != pane {
            return Err(Failure::Target.into());
        }
        if state.pid == 0 || state.session == 0 || state.started == 0 {
            return Err(Failure::Payload.into());
        }
        if state.cwd.len() > 4096 || state.command.len() > 256 {
            return Err(Failure::Capacity.into());
        }
        let mut program = program.clone();
        program.cwd = state.cwd.clone();
        if let Err(error) = crate::config::verify(&program) {
            self.refused(request);
            self.jobs.insert(request, Job::Ordinary);
            self.app.status(error.message());
            return Ok(());
        }
        let queued = self.queue.front_mut().ok_or(Failure::Identity)?;
        if queued.command != *command || queued.source != Some(pane) {
            return Err(Failure::Identity.into());
        }
        queued.program = Some(program);
        self.jobs.insert(request, Job::Ordinary);
        Ok(())
    }

    pub(super) fn refused(&mut self, request: u64) {
        if let Some(Job::Resolve { command, .. }) = self.jobs.get(&request)
            && self
                .queue
                .front()
                .is_some_and(|queued| queued.command == *command)
        {
            self.bytes -= weight(&self.queue.pop_front().unwrap().command);
        }
    }
    pub(super) fn replied(&mut self, reply: &protocol::Reply) -> Result<(), Error> {
        if let Ok(protocol::Status::Applied(protocol::Outcome::Workspace(
            workspace::Outcome::Changed(change),
        ))) = &reply.result
            && let Some(Job::Spawn(program)) = self.jobs.get(&reply.request)
        {
            let panes = change
                .created
                .iter()
                .filter(|target| matches!(target, Target::Pane(_)))
                .count();
            if change.created.len() > 2 || panes != 1 || !change.retired.is_empty() {
                return Err(Failure::Payload.into());
            }
            if self.launches.len() >= 32 {
                return Err(Failure::Capacity.into());
            }
            self.launches
                .extend(change.created.iter().filter_map(|target| match target {
                    Target::Pane(id) => Some((*id, program.clone())),
                    _ => None,
                }));
        }
        if reply.result.is_err() {
            self.refused(reply.request);
            self.jobs.remove(&reply.request);
        }
        Ok(())
    }
}
