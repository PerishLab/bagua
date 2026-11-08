#[cfg(test)]
mod checks;
mod context;
mod flow;
mod input;
mod intent;
mod reload;
use super::queue::{Queued, weight};
use crate::error::Error;
use bagua::ipc::Channel;
use client::{App, Gesture, Terminal};
use crossterm::event::Event;
use protocol::{
    Command, Failure, Request,
    wire::{Message, Program},
};
use std::{
    collections::{BTreeMap, VecDeque},
    sync::atomic::{AtomicBool, Ordering},
    time::{Duration, Instant},
};
use workspace::{Id, Operation};

enum Job {
    Ordinary,
    Spawn(Program),
    Start,
    Resolve {
        pane: Id,
        command: Command,
        program: Program,
    },
    Inspect(Id),
}

pub(super) struct Runner {
    channel: Channel,
    pub(super) app: App,
    next: u64,
    program: Option<Program>,
    jobs: BTreeMap<u64, Job>,
    launches: VecDeque<(Id, Program)>,
    leader: bool,
    intents: VecDeque<Gesture>,
    staged: usize,
    fence: Option<intent::Fence>,
    paused: bool,
    suspended: bool,
    keyboard: client::Keyboard,
    size: (u16, u16),
    pub(super) queue: VecDeque<Queued>,
    pub(super) bytes: usize,
    leaving: Option<Instant>,
    pub(super) config: crate::config::Loaded,
}

impl Runner {
    pub fn new(channel: Channel, app: App, next: u64, program: Option<Program>) -> Self {
        Self {
            channel,
            app,
            next,
            program,
            jobs: BTreeMap::new(),
            launches: VecDeque::new(),
            leader: false,
            intents: VecDeque::new(),
            staged: 0,
            fence: None,
            paused: false,
            suspended: false,
            keyboard: client::Keyboard::default(),
            size: (0, 0),
            queue: VecDeque::new(),
            bytes: 0,
            leaving: None,
            config: crate::config::Loaded::disabled(),
        }
    }

    pub fn run(&mut self, terminal: &mut Terminal, stop: &AtomicBool) -> Result<(), Error> {
        let mut painted = Instant::now() - Duration::from_secs(1);
        while !stop.load(Ordering::Relaxed) {
            if self.app.done() && self.leaving.is_none() {
                self.leaving = Some(Instant::now());
            }
            if self.leaving.is_some() && self.settled() {
                break;
            }
            if self
                .leaving
                .is_some_and(|started| started.elapsed() > Duration::from_secs(10))
            {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::TimedOut,
                    "detaching with unfinished operations; inspect host state",
                )
                .into());
            }
            self.channel.flush()?;
            for _ in 0..8 {
                let Some(message) = self.channel.receive()? else {
                    break;
                };
                self.receive(message)?;
            }
            self.pending()?;
            if self.leaving.is_none() {
                self.input(terminal)?;
            }
            self.release();
            self.drain()?;
            let size = terminal.size()?;
            if size != self.size {
                self.size = size;
                self.event(Event::Resize(size.0, size.1))?;
            }
            if self.app.dirty() && !self.suspended && painted.elapsed() >= Duration::from_millis(16)
            {
                terminal.offer(self.app.draw()?)?;
                self.app.clean();
                painted = Instant::now();
            }
            terminal.flush()?;
            std::thread::sleep(Duration::from_millis(2));
        }
        Ok(())
    }

    fn receive(&mut self, message: Message) -> Result<(), Error> {
        let refusal = self.settlement(&message);
        if let Message::Context {
            request,
            pane,
            state,
        } = &message
        {
            self.described(*request, *pane, state)?;
        }
        if let Message::Reply(reply) = &message {
            self.replied(reply)?;
        }
        if let Message::Finished { request, result } = &message {
            if matches!(self.jobs.get(request), Some(Job::Resolve { .. })) {
                if result.is_ok() {
                    return Err(Failure::Payload.into());
                }
                self.refused(*request);
            }
            self.jobs.remove(request);
        }
        self.app.accept(message)?;
        if let Some(reason) = refusal {
            self.pause(&reason);
        }
        Ok(())
    }

    fn pending(&mut self) -> Result<(), Error> {
        if self.jobs.len() >= 16 {
            return Ok(());
        }
        if let Some((pane, program)) = self.launches.front().cloned() {
            if self.submit(Command::Start { pane, program }, Job::Start)? {
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
        if self.leaving.is_some() {
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
        Ok(())
    }

    fn submit(&mut self, command: Command, job: Job) -> Result<bool, Error> {
        if intent::mutation(&command) && self.fence.is_some() {
            return Ok(false);
        }
        if self.jobs.len() >= 16 {
            self.app
                .status("Request budget full; operation was not admitted");
            return Ok(false);
        }
        let request = Request::identified(self.next, command)?;
        match self.channel.send(&request) {
            Ok(()) => {}
            Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                self.app
                    .status("Transport budget full; operation was not admitted");
                return Ok(false);
            }
            Err(error) => return Err(error.into()),
        }
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

    fn settled(&self) -> bool {
        match (
            self.queue.front(),
            self.jobs.first_key_value(),
            self.intents.front(),
        ) {
            (None, None, None) => self.gate() == intent::Gate::Open,
            _ => false,
        }
    }
}
