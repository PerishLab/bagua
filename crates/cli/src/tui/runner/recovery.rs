use super::*;

pub(super) enum Resolution {
    Refused(String),
    Unknown(String),
}

pub(super) struct Recovery {
    started: Instant,
    request: Option<u64>,
    catalog: bool,
}

impl Runner {
    pub(super) fn refuse(&mut self, reason: &str) {
        self.routing.reject(&mut self.intents);
        self.staged = self
            .intents
            .iter()
            .map(|item| flow::size(&item.gesture))
            .sum();
        self.keyboard.cancel(Instant::now());
        self.leader = false;
        self.app.prefix(None);
        self.app.cancel();
        self.app
            .error(format!("{reason}; dependent input discarded"));
    }

    pub(super) fn resolve(&mut self, resolution: Resolution) {
        match resolution {
            Resolution::Refused(reason) => self.refuse(&reason),
            Resolution::Unknown(reason) => {
                self.refuse(&reason);
                self.intents.clear();
                self.staged = 0;
                self.routing.abandon();
                if self.recovery.is_none() {
                    self.recovery = Some(Recovery {
                        started: Instant::now(),
                        request: None,
                        catalog: false,
                    });
                }
                self.app
                    .error("Recovering; uncertain input will not be replayed");
            }
        }
    }

    pub(super) fn recover(&mut self) -> Result<(), Error> {
        if self.recovery.is_none() && self.fence.as_ref().is_some_and(intent::Fence::overdue) {
            self.resolve(Resolution::Unknown("Layout outcome not confirmed".into()));
        }
        let Some(recovery) = &self.recovery else {
            return Ok(());
        };
        if recovery.started.elapsed() >= Duration::from_secs(5) {
            return Err(std::io::Error::new(
                std::io::ErrorKind::TimedOut,
                "Recovery incomplete; reconnect with bagua; host and applications preserved",
            )
            .into());
        }
        if recovery.request.is_some() || !self.jobs.is_empty() || !self.launches.is_empty() {
            return Ok(());
        }
        let request = self.next;
        if self.submit(Command::Workspace(Operation::Read), Job::Ordinary)? {
            self.recovery.as_mut().unwrap().request = Some(request);
        }
        Ok(())
    }

    pub(super) fn synchronizing(&mut self, message: &Message) -> Result<bool, Error> {
        let Some(recovery) = &mut self.recovery else {
            return Ok(false);
        };
        match message {
            Message::Reply(reply) if recovery.request == Some(reply.request) => {
                if let Ok(protocol::Status::Applied(protocol::Outcome::Workspace(
                    workspace::Outcome::Snapshot(snapshot),
                ))) = &reply.result
                {
                    self.app.catalog(snapshot.clone())?;
                    recovery.catalog = true;
                }
                Ok(true)
            }
            Message::Finished { request, result } if recovery.request == Some(*request) => {
                if result.is_ok() && recovery.catalog {
                    self.fence = None;
                    self.routing.settle(&mut self.intents);
                    self.recovery = None;
                    self.app
                        .status("Live; uncertain input discarded, no replay");
                }
                Ok(false)
            }
            _ => Ok(false),
        }
    }
}

#[cfg(test)]
impl Recovery {
    pub(super) fn expire(&mut self) {
        self.started = Instant::now() - Duration::from_secs(6);
    }
}
