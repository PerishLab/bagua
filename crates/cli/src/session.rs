use crate::{args::Args, error::Error};
use bagua::ipc::Connection;
use protocol::{Command, Failure, Outcome, Request, Status, wire::Message};
use std::{
    io::{self, BufRead, Read},
    path::Path,
};
use workspace::{Operation, Snapshot};

pub(crate) struct Session {
    connection: Connection,
    pub actor: workspace::Id,
    pub canvas: workspace::Geometry,
    pub created: Vec<workspace::Target>,
    pub next: u64,
    json: bool,
}

impl Session {
    pub fn channel(self) -> Result<bagua::ipc::Channel, Error> {
        Ok(self.connection.channel()?)
    }

    pub fn connect(path: &Path, args: &Args) -> Result<Self, Error> {
        if args.request == 0 {
            return Err(Failure::Identity.into());
        }
        let mut connection = Connection::connect(path)?;
        let (actor, canvas) = match connection.receive()? {
            Message::Hello {
                version: protocol::VERSION,
                client,
                canvas,
            } => (client, canvas),
            _ => return Err(Failure::Version.into()),
        };
        Ok(Self {
            connection,
            actor,
            canvas,
            created: Vec::new(),
            next: args.request,
            json: args.json,
        })
    }

    pub fn join(&mut self, id: u64, acquire: bool) -> Result<(), Error> {
        let snapshot = self
            .call(Command::Workspace(Operation::Read), false)?
            .ok_or(Failure::Payload)?;
        let id = Command::identity(id)?;
        let geometry = snapshot
            .sessions()
            .find(|s| s.id() == id)
            .ok_or(Failure::Target)?
            .geometry();
        self.call(
            Command::Workspace(Operation::Join {
                session: id,
                viewport: geometry,
            }),
            false,
        )?;
        if acquire {
            self.call(
                Command::Workspace(Operation::Acquire {
                    session: id,
                    fixed: None,
                }),
                false,
            )?;
        }
        Ok(())
    }

    pub fn call(&mut self, command: Command, visible: bool) -> Result<Option<Snapshot>, Error> {
        let request = Request::identified(self.next, command)?;
        self.next = self.next.checked_add(1).ok_or(Failure::Identity)?;
        self.exchange(request, visible)
    }

    fn exchange(&mut self, request: Request, visible: bool) -> Result<Option<Snapshot>, Error> {
        self.connection
            .send(&request)
            .map_err(|error| Error::from(error).at(request.id))?;
        let mut snapshot = None;
        loop {
            let message = self
                .connection
                .receive()
                .map_err(|error| Error::from(error).at(request.id))?;
            if visible {
                self.print(&message)?;
            }
            match message {
                Message::Reply(reply) if reply.request == request.id => {
                    let status = reply
                        .result
                        .map_err(|error| Error::from(error).at(request.id))?;
                    self.remember(&status);
                    snapshot = catalog(status).or(snapshot);
                }
                Message::Finished {
                    request: id,
                    result,
                } if id == request.id => {
                    result.map_err(|error| Error::from(error).at(request.id))?;
                    return Ok(snapshot);
                }
                _ => {}
            }
        }
    }

    pub fn control(&mut self) -> Result<(), Error> {
        self.json = true;
        let input = io::stdin();
        let mut reader = input.lock();
        loop {
            let mut bytes = Vec::new();
            let count = std::io::Read::take(reader.by_ref(), protocol::wire::REQUEST as u64 + 1)
                .read_until(b'\n', &mut bytes)?;
            if count == 0 {
                return Ok(());
            }
            if count > protocol::wire::REQUEST {
                return Err(Failure::Payload.into());
            }
            let request = serde_json::from_slice(&bytes).map_err(|_| Failure::Payload)?;
            self.exchange(request, true)?;
        }
    }

    fn remember(&mut self, status: &Status) {
        if let Status::Applied(Outcome::Workspace(workspace::Outcome::Changed(change))) = status {
            self.created = change.created.clone();
        }
    }

    pub fn watch(&mut self) -> Result<(), Error> {
        self.connection.streaming()?;
        loop {
            let message = self.connection.receive()?;
            self.print(&message)?;
        }
    }

    fn print(&self, message: &Message) -> Result<(), Error> {
        let output = if self.json {
            serde_json::to_string(message)
        } else {
            serde_json::to_string_pretty(message)
        };
        println!("{}", output.map_err(|_| Failure::Payload)?);
        Ok(())
    }
}

fn catalog(status: Status) -> Option<Snapshot> {
    match status {
        Status::Applied(Outcome::Workspace(workspace::Outcome::Snapshot(value))) => Some(value),
        _ => None,
    }
}
