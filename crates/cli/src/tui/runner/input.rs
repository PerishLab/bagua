use super::Runner;
use crate::error::Error;
use client::{Gesture, Terminal};
use protocol::{Command, Failure};
use std::time::Instant;

impl Runner {
    pub(super) fn key(&mut self, key: crossterm::event::KeyEvent) -> Result<Vec<Command>, Failure> {
        if key.kind == crossterm::event::KeyEventKind::Release {
            return Ok(Vec::new());
        }
        let gesture = self.config.keys().translate(key, self.leader);
        self.leader = false;
        let gesture = gesture?;
        if gesture == Gesture::Help {
            self.app.status(format!("Prefix: {} | defaults for keys without overrides: d/q detach, \\/- split, = window, Tab pane, w next, r reload, PgUp/PgDn history, End live; bagua config lists binding overrides", self.config.prefix()));
            return Ok(Vec::new());
        }
        if gesture == Gesture::Reload {
            self.reload();
            return Ok(Vec::new());
        }
        if gesture == Gesture::Detach {
            self.leaving = Some(Instant::now());
            return Ok(Vec::new());
        }
        if gesture == Gesture::Leader {
            self.leader = true;
            self.app
                .status(format!("{}: prefix pending", self.config.prefix()));
            return Ok(Vec::new());
        }
        if self.suspended && gesture != Gesture::Detach {
            return Err(Failure::Payload);
        }
        self.app.handle(gesture)
    }

    pub(super) fn input(&mut self, terminal: &mut Terminal) -> Result<(), Error> {
        let mut bytes = [0; 4096];
        let length = self.keyboard.room().min(bytes.len());
        let count = terminal.read(&mut bytes[..length])?;
        self.keyboard.push(&bytes[..count])?;
        for _ in 0..16 {
            match self.keyboard.next(Instant::now()) {
                Some(Ok(event)) => self.event(event)?,
                Some(Err(error)) => {
                    self.leader = false;
                    self.app.status(error.message());
                }
                None => break,
            }
            if self.leaving.is_some() {
                break;
            }
        }
        Ok(())
    }
}
