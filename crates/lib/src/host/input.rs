use super::{
    Host,
    pane::{Pane, Write},
};
use protocol::{
    Failure, Input, Status,
    wire::{Fault, Message},
};
use pty::{CHUNK, Transfer};
use workspace::{Id, State};

type Completion = (Id, u64, Result<(), Fault>);

impl Host {
    pub(super) fn send(
        &mut self,
        actor: Id,
        request: u64,
        data: (Id, Input),
    ) -> Result<(), Failure> {
        let (id, input) = data;
        self.world.input(actor, id)?;
        input.check()?;
        let pane = self.panes.get_mut(&id).ok_or(Failure::Target)?;
        if pane.eof || pane.retiring || pane.process.is_none() {
            return Err(Failure::Native);
        }
        let bytes = pane
            .engine
            .encode(input.borrow())
            .map_err(|_| Failure::Payload)?;
        if pane.input.len() >= 16 || pane.queued() + bytes.len() > 65_536 {
            return Err(Failure::Capacity);
        }
        pane.input.push_back(Write {
            actor,
            request,
            bytes,
            offset: 0,
        });
        self.reply(actor, request, Ok(Status::Admitted))
    }

    pub(super) fn pump(&mut self) -> Result<(), Failure> {
        let ids: Vec<_> = self.panes.keys().copied().collect();
        for id in ids {
            let pane = self.panes.get_mut(&id).unwrap();
            let completions = pane.pump();
            if let Some(exit) = pane.exit
                && self
                    .world
                    .pane(id)
                    .is_some_and(|p| p.state() == State::Running)
            {
                self.world.observe(id, State::Exited(exit.code))?;
            }
            let message = pane.notice(id);
            for (actor, request, result) in completions {
                self.complete(actor, request, result)?;
            }
            if let Some(message) = message {
                self.announce(id, message)?;
            }
        }
        Ok(())
    }
}

impl Pane {
    fn notice(&mut self, id: Id) -> Option<Message> {
        if self.reported {
            return None;
        }
        if matches!((self.exit, self.eof, &self.fault), (None, false, None)) {
            return None;
        }
        self.reported = true;
        Some(self.observation(id))
    }

    pub(super) fn observation(&self, id: Id) -> Message {
        Message::Process {
            pane: id,
            io: self
                .process
                .as_ref()
                .is_some_and(|process| process.status().io),
            closing: self.retiring && self.process.is_some(),
            exited: self.exit.is_some(),
            code: self.exit.and_then(|e| e.code),
            signal: self.exit.and_then(|e| e.signal),
            eof: self.eof,
            fault: self.fault.clone(),
        }
    }

    fn pump(&mut self) -> Vec<Completion> {
        let mut completed = Vec::new();
        if self.process.is_none() {
            return completed;
        }
        if self.retiring {
            self.cancel(&mut completed);
        } else {
            self.write(&mut completed);
            if let Err(error) = self.drain() {
                self.fault = Some(error);
                self.reported = false;
                self.retiring = true;
                self.cancel(&mut completed);
                if let Err(error) = self.process.as_mut().unwrap().retire() {
                    let error = super::effects::fault(error);
                    self.cleanup = Some(error.clone());
                    self.fault = Some(error);
                }
            }
        }
        match self.process.as_mut().unwrap().observe() {
            Ok(Some(exit)) => {
                if self.exit.is_none() {
                    self.reported = false;
                }
                self.exit = Some(exit);
            }
            Ok(None) => {}
            Err(error) => {
                let error = super::effects::fault(error);
                if self.retiring {
                    self.cleanup = Some(error.clone());
                }
                if self.fault.as_ref() != Some(&error) {
                    self.fault = Some(error);
                    self.reported = false;
                }
            }
        }
        if self.exit.is_some() && (self.eof || self.retiring) {
            self.cancel(&mut completed);
            self.process = None;
        }
        completed
    }

    fn write(&mut self, completed: &mut Vec<Completion>) {
        let process = self.process.as_mut().unwrap();
        let Some(write) = self.input.front_mut() else {
            return;
        };
        let result = process.write(&write.bytes[write.offset..]);
        match result {
            Ok(Transfer::Bytes(count)) => {
                write.offset += count;
                if write.offset == write.bytes.len() {
                    let write = self.input.pop_front().unwrap();
                    completed.push((write.actor, write.request, Ok(())));
                }
            }
            Ok(Transfer::Pending) => {}
            other => {
                let error = match other {
                    Err(error) => super::effects::fault(error),
                    _ => Failure::Native.into(),
                };
                let write = self.input.pop_front().unwrap();
                completed.push((write.actor, write.request, Err(error)));
            }
        }
    }

    fn respond(&mut self) {
        let process = self.process.as_mut().unwrap();
        let bytes = self.replies.make_contiguous();
        if bytes.is_empty() {
            return;
        }
        match process.write(bytes) {
            Ok(Transfer::Bytes(count)) => {
                self.replies.drain(..count);
            }
            Ok(Transfer::Pending) => {}
            other => {
                self.fault = Some(match other {
                    Err(error) => super::effects::fault(error),
                    _ => Failure::Native.into(),
                });
                self.reported = false;
                self.replies.clear();
            }
        }
    }

    fn drain(&mut self) -> Result<(), Fault> {
        self.respond();
        if self.eof || self.replies.len() > 65_536 {
            return Ok(());
        }
        let mut bytes = [0; CHUNK];
        match self
            .process
            .as_mut()
            .unwrap()
            .read(&mut bytes)
            .map_err(super::effects::fault)?
        {
            Transfer::Bytes(count) => {
                let report = self
                    .engine
                    .feed(&bytes[..count])
                    .map_err(|_| Fault::from(Failure::Payload))?;
                self.replies.extend(report.replies);
                self.bytes = self.bytes.saturating_add(count as u64);
                self.unsupported = self.unsupported.saturating_add(report.unsupported);
                self.bells = self.bells.saturating_add(report.bells);
            }
            Transfer::Eof => {
                self.eof = true;
                self.reported = false;
            }
            Transfer::Pending => {}
        }
        Ok(())
    }

    fn cancel(&mut self, completed: &mut Vec<Completion>) {
        for write in self.input.drain(..) {
            completed.push((write.actor, write.request, Err(Failure::Native.into())));
        }
        self.replies.clear();
    }
}
