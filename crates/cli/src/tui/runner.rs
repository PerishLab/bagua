#[cfg(test)]
mod checks;
mod context;
mod decor;
mod dispatch;
mod flow;
mod input;
mod intent;
mod recovery;
mod reload;
mod routing;
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
    Header {
        refused: bool,
    },
    Metadata {
        pane: Id,
        at: Instant,
        received: bool,
    },
    Spawn(Program),
    Start(Id),
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
    intents: VecDeque<routing::Deferred>,
    routing: routing::Routing,
    staged: usize,
    fence: Option<intent::Fence>,
    recovery: Option<recovery::Recovery>,
    header: Option<(u64, bool)>,
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
            routing: routing::Routing::default(),
            staged: 0,
            fence: None,
            recovery: None,
            header: None,
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
            self.recover()?;
            self.app.tick(Instant::now(), std::time::SystemTime::now());
            self.copies(terminal);
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
                terminal.present(self.app.paint()?)?;
                self.app.clean();
                painted = Instant::now();
            }
            terminal.flush()?;
            std::thread::sleep(Duration::from_millis(2));
        }
        self.copies(terminal);
        if self.app.copying().is_some() {
            return Err(std::io::Error::other(
                "clipboard effects remain unadmitted; OS clipboard unknown",
            )
            .into());
        }
        Ok(())
    }

    fn copies(&mut self, terminal: &mut Terminal) {
        while let Some(copy) = self.app.copying() {
            if !terminal.copy(copy) {
                break;
            }
            self.app.copied();
        }
    }

    fn receive(&mut self, message: Message) -> Result<(), Error> {
        if self.metadata(&message)? {
            return Ok(());
        }
        if self.synchronizing(&message)? {
            return Ok(());
        }
        let rejected = self.rejected(&message);
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
            if let (Some(Job::Start(pane)), Err(fault)) = (self.jobs.get(request), result) {
                self.app.reported(*pane, fault.clone());
            }
            self.jobs.remove(request);
        }
        self.app.accept(message)?;
        if rejected {
            self.app
                .status("Header reservation refused; application layout unchanged");
        } else if let Some(reason) = refusal {
            self.resolve(reason);
        }
        Ok(())
    }

    fn settled(&self) -> bool {
        if self.app.copying().is_some() {
            return false;
        }
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
