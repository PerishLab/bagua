use super::*;

impl Runner {
    pub(super) fn decorate(&mut self) -> Result<bool, Error> {
        if self.size == (0, 0) {
            return Ok(false);
        }
        if !matches!(
            (self.app.controller(), self.gate(), self.intents.front()),
            (true, intent::Gate::Open, None)
        ) {
            return Ok(false);
        }
        let Some(session) = self.app.session() else {
            return Ok(false);
        };
        let enabled = self.config.decor().header.enabled;
        if session.header() == enabled {
            return Ok(false);
        }
        let Some(revision) = self.app.revision() else {
            return Ok(false);
        };
        if self.header == Some((revision, enabled)) {
            return Ok(false);
        }
        let canvas = workspace::Canvas {
            header: enabled,
            ..session.canvas()
        };
        if session
            .windows()
            .any(|window| window.placements(canvas).is_err())
        {
            self.header = Some((revision, enabled));
            self.app.error(
                "Header reservation unavailable at this geometry; application layout unchanged",
            );
            return Ok(false);
        }
        if self.submit(
            Command::Workspace(Operation::Header {
                session: self.app.target(),
                enabled,
            }),
            Job::Header { refused: false },
        )? {
            self.header = Some((revision, enabled));
            return Ok(true);
        }
        Ok(false)
    }

    pub(super) fn rejected(&mut self, message: &Message) -> bool {
        let request = match message {
            Message::Reply(reply) => reply.request,
            Message::Finished { request, .. } => *request,
            _ => return false,
        };
        let Some(Job::Header { refused }) = self.jobs.get_mut(&request) else {
            return false;
        };
        if let Message::Reply(reply) = message
            && reply.result.is_err()
        {
            *refused = true;
        }
        *refused
    }

    pub(super) fn inspecting(&mut self) -> Result<(), Error> {
        if self.jobs.len() >= 16 {
            return Ok(());
        }
        if self
            .jobs
            .values()
            .filter(|job| matches!(job, Job::Metadata { .. }))
            .count()
            >= 2
        {
            return Ok(());
        }
        let pane = self.app.needing().into_iter().find(|pane| {
            !self
                .jobs
                .values()
                .any(|job| matches!(job, Job::Metadata { pane: id, .. } if id==pane))
        });
        if let Some(pane) = pane {
            self.submit(
                Command::Describe(pane),
                Job::Metadata {
                    pane,
                    at: Instant::now(),
                    received: false,
                },
            )?;
        }
        Ok(())
    }

    pub(super) fn metadata(&mut self, message: &Message) -> Result<bool, Error> {
        let request = match message {
            Message::Reply(reply) => reply.request,
            Message::Context { request, .. } | Message::Finished { request, .. } => *request,
            _ => return Ok(false),
        };
        let Some(Job::Metadata { pane, at, received }) = self.jobs.get_mut(&request) else {
            return Ok(false);
        };
        match message {
            Message::Context {
                pane: target,
                state,
                ..
            } => {
                if *target != *pane || *received {
                    return Err(Failure::Payload.into());
                }
                self.app.context(*pane, Some(state.clone()), *at)?;
                *received = true;
            }
            Message::Finished { result, .. } => {
                if result.is_ok() && !*received {
                    return Err(Failure::Payload.into());
                }
                if result.is_err() {
                    self.app.context(*pane, None, *at)?;
                }
                self.jobs.remove(&request);
            }
            _ => {}
        }
        Ok(true)
    }
}
