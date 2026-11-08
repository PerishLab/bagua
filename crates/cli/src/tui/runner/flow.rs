use super::*;

impl Runner {
    pub(super) fn stage(&mut self, gesture: Gesture) -> Result<(), Failure> {
        if self.paused {
            self.app
                .status("Input paused after a failed intent; Escape resumes without replay");
            return Ok(());
        }
        let bytes = size(&gesture);
        if self.staged + bytes > 65_536 {
            return Err(Failure::Capacity);
        }
        if let Some(Gesture::Input(protocol::Input::Text(text))) = self.intents.back_mut()
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
        self.intents.push_back(gesture);
        Ok(())
    }

    pub(super) fn drain(&mut self) -> Result<(), Error> {
        for _ in 0..16 {
            if self.paused || self.suspended || self.fenced() {
                break;
            }
            let Some(gesture) = self.intents.pop_front() else {
                break;
            };
            self.staged -= size(&gesture);
            if gesture == Gesture::Reload {
                self.reload();
                continue;
            }
            if self.app.done() {
                self.pause("workspace closed; pending input was not sent");
                break;
            }
            let changing = intent::changing(&gesture);
            let commands = self.app.handle(gesture);
            if changing && let Err(failure) = &commands {
                self.pause(failure.message());
                break;
            }
            self.commands(commands)?;
        }
        Ok(())
    }

    pub(super) fn event(&mut self, event: Event) -> Result<(), Error> {
        if matches!(event, Event::Paste(_)) {
            self.leader = false;
        }
        if self.suspended && matches!(event, Event::Paste(_) | Event::Mouse(_)) {
            self.app
                .status("Terminal too small; resize or use the configured detach binding");
            return Ok(());
        }
        let commands = match event {
            Event::Key(key) => self.key(key),
            Event::Paste(text) => self
                .stage(Gesture::Input(protocol::Input::Paste(text)))
                .map(|()| Vec::new()),
            Event::Mouse(mouse) => {
                self.drain()?;
                if self.fenced() || self.paused || !self.intents.is_empty() {
                    self.app
                        .status("Layout pending or input paused; mouse gesture was not admitted");
                    return Ok(());
                }
                self.app.mouse(mouse)
            }
            Event::Resize(columns, rows) => {
                let Some(extent) = crate::tui::extent(columns, rows) else {
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
                self.app.status(error.message());
                return Ok(());
            }
        };
        for command in commands {
            if matches!(
                command,
                Command::Workspace(Operation::Split { .. } | Operation::Window(_))
            ) && self.program.is_none()
            {
                self.pause("provide --executable and --cwd before creating panes");
                continue;
            }
            let changing = intent::mutation(&command);
            if !self.enqueue(command) && changing {
                self.pause("the layout intent exceeded its admission budget");
            }
        }
        Ok(())
    }
}

fn size(gesture: &Gesture) -> usize {
    match gesture {
        Gesture::Input(protocol::Input::Text(text) | protocol::Input::Paste(text)) => text.len(),
        _ => 64,
    }
}
