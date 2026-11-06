use super::queue::weight;
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
use workspace::{Id, Operation, Target};

enum Job {
    Ordinary,
    Spawn,
    Inspect(Id),
}

pub(super) struct Runner {
    channel: Channel,
    pub(super) app: App,
    next: u64,
    program: Option<Program>,
    jobs: BTreeMap<u64, Job>,
    launches: VecDeque<Id>,
    leader: bool,
    suspended: bool,
    keyboard: client::Keyboard,
    size: (u16, u16),
    pub(super) queue: VecDeque<Command>,
    pub(super) bytes: usize,
    leaving: Option<Instant>,
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
            suspended: false,
            keyboard: client::Keyboard::default(),
            size: (0, 0),
            queue: VecDeque::new(),
            bytes: 0,
            leaving: None,
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
        if let Message::Reply(reply) = &message {
            self.replied(reply);
        }
        if let Message::Finished { request, .. } = &message {
            self.jobs.remove(request);
        }
        self.app.accept(message)?;
        Ok(())
    }

    fn pending(&mut self) -> Result<(), Error> {
        if self.jobs.len() >= 16 {
            return Ok(());
        }
        if let Some(command) = self.queue.front().cloned() {
            let job = match command {
                Command::Workspace(Operation::Split { .. } | Operation::Window(_)) => Job::Spawn,
                _ => Job::Ordinary,
            };
            if self.submit(command, job)? {
                let command = self.queue.pop_front().unwrap();
                self.bytes -= weight(&command);
            }
            return Ok(());
        }
        if let Some(pane) = self.launches.front().copied() {
            if let Some(program) = self.program.clone() {
                if self.submit(Command::Start { pane, program }, Job::Ordinary)? {
                    self.launches.pop_front();
                }
            } else {
                self.launches.pop_front();
                self.app
                    .status("Pane is pending; no explicit program configured");
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
        self.jobs.insert(self.next, job);
        self.next = self.next.checked_add(1).ok_or(Failure::Identity)?;
        Ok(true)
    }

    fn event(&mut self, event: Event) -> Result<(), Error> {
        if self.suspended && matches!(event, Event::Paste(_) | Event::Mouse(_)) {
            self.app
                .status("Terminal too small; resize or Ctrl-G d to detach");
            return Ok(());
        }
        let commands = match event {
            Event::Key(key) => self.key(key),
            Event::Paste(text) => self
                .app
                .handle(Gesture::Input(protocol::Input::Paste(text))),
            Event::Mouse(mouse) => self.app.mouse(mouse),
            Event::Resize(columns, rows) => {
                let Some(extent) = super::extent(columns, rows) else {
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

    fn commands(&mut self, commands: Result<Vec<Command>, Failure>) -> Result<(), Error> {
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
                self.app
                    .status("Provide --executable and --cwd to create interactive panes");
                continue;
            }
            self.enqueue(command);
        }
        Ok(())
    }

    fn key(&mut self, key: crossterm::event::KeyEvent) -> Result<Vec<Command>, Failure> {
        let gesture = client::translate(key, self.leader);
        self.leader = false;
        let gesture = gesture?;
        if gesture == Gesture::Detach {
            self.leaving = Some(Instant::now());
            return Ok(Vec::new());
        }
        if gesture == Gesture::Leader {
            self.leader = true;
            self.app.status("Ctrl-G: ? help, d detach");
            return Ok(Vec::new());
        }
        if self.suspended && gesture != Gesture::Detach {
            return Err(Failure::Payload);
        }
        self.app.handle(gesture)
    }

    fn settled(&self) -> bool {
        self.queue.is_empty() && self.jobs.is_empty() && self.launches.is_empty()
    }

    fn replied(&mut self, reply: &protocol::Reply) {
        if let Ok(protocol::Status::Applied(protocol::Outcome::Workspace(
            workspace::Outcome::Changed(change),
        ))) = &reply.result
            && matches!(self.jobs.get(&reply.request), Some(Job::Spawn))
        {
            self.launches
                .extend(change.created.iter().filter_map(|target| match target {
                    Target::Pane(id) => Some(*id),
                    _ => None,
                }));
        }
        if reply.result.is_err() {
            self.jobs.remove(&reply.request);
        }
    }

    fn input(&mut self, terminal: &mut Terminal) -> Result<(), Error> {
        let mut bytes = [0; 4096];
        let length = self.keyboard.room().min(bytes.len());
        let count = terminal.read(&mut bytes[..length])?;
        self.keyboard.push(&bytes[..count])?;
        for _ in 0..16 {
            match self.keyboard.next(Instant::now()) {
                Some(Ok(event)) => self.event(event)?,
                Some(Err(error)) => self.app.status(error.message()),
                None => break,
            }
            if self.leaving.is_some() {
                break;
            }
        }
        Ok(())
    }
}
