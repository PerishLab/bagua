use super::*;

impl Runner {
    pub(super) fn pending(&mut self) -> Result<(), Error> {
        if self.jobs.len() >= 16 {
            return Ok(());
        }
        if let Some((pane, program)) = self.launches.front().cloned() {
            if self.submit(Command::Start { pane, program }, Job::Start(pane))? {
                self.launches.pop_front();
            }
            return Ok(());
        }
        if let Some(queued) = self.queue.front().cloned() {
            if let Command::History { pane, read } = &queued.command
                && !self.app.current(*pane, read)
            {
                self.bytes -= weight(&self.queue.pop_front().unwrap().command);
                return Ok(());
            }
            if self.dispatch(queued)? {
                let command = self.queue.pop_front().unwrap();
                self.bytes -= weight(&command.command);
            }
            return Ok(());
        }
        if self.leaving.is_some() || self.recovery.is_some() {
            return Ok(());
        }
        if self.decorate()? {
            return Ok(());
        }
        if let Some(pane) = self.app.missing().into_iter().find(|pane| {
            !self
                .jobs
                .values()
                .any(|job| matches!(job,Job::Inspect(id) if id == pane))
        }) {
            self.submit(Command::Inspect(pane), Job::Inspect(pane))?;
        }
        self.inspecting()?;
        Ok(())
    }

    pub(super) fn submit(&mut self, command: Command, job: Job) -> Result<bool, Error> {
        if intent::mutation(&command) && self.fence.is_some() {
            return Ok(false);
        }
        if self.jobs.len() >= 16 {
            self.app
                .error("Request budget full; operation was not admitted");
            return Ok(false);
        }
        let request = Request::identified(self.next, command)?;
        match self.channel.send(&request) {
            Ok(()) => {}
            Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                self.app
                    .error("Transport budget full; operation was not admitted");
                return Ok(false);
            }
            Err(error) => return Err(error.into()),
        }
        crate::observation::event::Scope(self.scope.as_ref()).receipt(
            crate::observation::event::kind(&request.command),
            (request.id, crate::observation::event::Stage::Sent),
        );
        if let Command::History { pane, read } = &request.command {
            self.app.bind(self.next, *pane, read);
        }
        if intent::mutation(&request.command) {
            self.fence = Some(intent::Fence::new(&request));
        }
        self.jobs.insert(self.next, job);
        self.next = self.next.checked_add(1).ok_or(Failure::Identity)?;
        Ok(true)
    }
}
