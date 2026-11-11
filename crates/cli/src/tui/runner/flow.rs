use super::*;

impl Runner {
    pub(super) fn stage(&mut self, gesture: Gesture) -> Result<(), Failure> {
        if self.recovery.is_some() {
            self.app.error("Recovering; input was not admitted");
            return Ok(());
        }
        let bytes = size(&gesture);
        if self.staged + bytes > 65_536 {
            return Err(Failure::Capacity);
        }
        if let Some(pending) = self.intents.back_mut()
            && self.routing.adjacent(pending)
            && let Gesture::Input(protocol::Input::Text(text)) = &mut pending.gesture
            && let Gesture::Input(protocol::Input::Text(extra)) = &gesture
            && text.len() + extra.len() <= terminal::CHUNK
        {
            text.push_str(extra);
            self.staged += bytes;
            return Ok(());
        }
        if self.intents.len() >= 64 {
            return Err(Failure::Capacity);
        }
        self.staged += bytes;
        self.intents.push_back(self.routing.bind(gesture)?);
        Ok(())
    }

    pub(super) fn drain(&mut self) -> Result<(), Error> {
        for _ in 0..16 {
            if self.recovery.is_some() || self.suspended || self.fenced() {
                break;
            }
            let Some(deferred) = self.intents.pop_front() else {
                break;
            };
            self.routing.begin(&deferred);
            let gesture = deferred.gesture;
            self.staged -= size(&gesture);
            if gesture == Gesture::Reload {
                self.reload();
                continue;
            }
            if self.app.done() {
                self.intents.clear();
                self.staged = 0;
                self.refuse("Workspace closed; pending input was not sent");
                break;
            }
            let changing = intent::changing(&gesture);
            let commands = self.app.handle(gesture);
            if changing && let Err(failure) = &commands {
                self.refuse(failure.message());
                break;
            }
            if commands.as_ref().is_ok_and(Vec::is_empty) {
                self.routing.settle(&mut self.intents);
            }
            self.commands(commands)?;
        }
        Ok(())
    }

    pub(super) fn event(&mut self, event: Event) -> Result<(), Error> {
        if matches!(event, Event::Paste(_)) {
            self.leader = false;
            self.app.prefix(None);
        }
        if self.suspended && matches!(event, Event::Paste(_) | Event::Mouse(_)) {
            self.app.cancel();
            self.app
                .error("Terminal too small; resize or use the configured detach binding");
            return Ok(());
        }
        let commands = match event {
            Event::Key(key) => self.key(key),
            Event::Paste(text) => self
                .stage(Gesture::Input(protocol::Input::Paste(text)))
                .map(|()| Vec::new()),
            Event::Mouse(mouse) => {
                self.drain()?;
                if self.fenced() || self.recovery.is_some() || !self.intents.is_empty() {
                    self.app.cancel();
                    self.app
                        .error("Layout pending or recovering; mouse gesture was not admitted");
                    return Ok(());
                }
                self.app.mouse(mouse)
            }
            Event::Resize(columns, rows) => {
                let Some(extent) =
                    crate::tui::extent(columns, rows, self.config.decor().status.rows())
                else {
                    self.suspended = true;
                    return Ok(());
                };
                self.suspended = false;
                self.app.resize(extent);
                Ok(vec![Command::Workspace(Operation::Resize {
                    session: self.app.target(),
                    geometry: self.app.geometry(),
                })])
            }
            _ => return Ok(()),
        };
        self.commands(commands)
    }

    pub(super) fn commands(
        &mut self,
        commands: Result<Vec<Command>, Failure>,
    ) -> Result<(), Error> {
        let commands = match commands {
            Ok(commands) => commands,
            Err(error) => {
                self.app.error(error.message());
                return Ok(());
            }
        };
        for command in commands {
            let changing = intent::mutation(&command);
            if changing {
                self.routing.command()?;
            }
            if matches!(
                command,
                Command::Workspace(Operation::Split { .. } | Operation::Window(_))
            ) && self.program.is_none()
            {
                self.refuse("provide --executable and --cwd before creating panes");
                continue;
            }
            if !self.enqueue(command) && changing {
                self.refuse("the layout intent exceeded its admission budget");
            }
        }
        Ok(())
    }
}

pub(super) fn size(gesture: &Gesture) -> usize {
    match gesture {
        Gesture::Input(protocol::Input::Text(text) | protocol::Input::Paste(text)) => text.len(),
        _ => 64,
    }
}
