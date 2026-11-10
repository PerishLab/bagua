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
    pub entered: Option<workspace::Id>,
    pub identity: Option<protocol::wire::Image>,
    pub next: u64,
    json: bool,
    pub scope: Option<locus::Context>,
}

impl Session {
    pub fn channel(self) -> Result<bagua::ipc::Channel, Error> {
        Ok(self.connection.channel()?)
    }

    pub fn connect(path: &Path, args: &Args) -> Result<Self, Error> {
        if args.request == 0 {
            return Err(Failure::Identity.into());
        }
        let connection = Connection::connect(path)?;
        Self::adopt(connection, args)
    }

    #[locus::trace(with = crate::observation::view(), frame = context)]
    pub fn adopt(mut connection: Connection, args: &Args) -> Result<Self, Error> {
        if args.request == 0 {
            return Err(Failure::Identity.into());
        }
        let (actor, canvas, identity) = match connection.receive()? {
            Message::Hello {
                version: protocol::VERSION,
                client,
                canvas,
                identity,
            } => (client, canvas, identity),
            _ => return Err(Failure::Version.into()),
        };
        crate::observation::event::Scope(context.as_ref()).connection(actor);
        Ok(Self {
            connection,
            scope: context,
            actor,
            canvas,
            created: Vec::new(),
            entered: None,
            identity,
            next: args.request,
            json: args.json,
        })
    }

    pub fn bounded(&self, timeout: std::time::Duration) -> Result<(), Error> {
        Ok(self.connection.bounded(timeout)?)
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
        let kind = crate::observation::event::kind(&request.command);
        self.connection.send(&request).map_err(|error| {
            crate::observation::event::Scope(self.scope.as_ref()).receipt(
                kind,
                (request.id, crate::observation::event::Stage::Unknown),
            );
            Error::from(error).at(request.id)
        })?;
        crate::observation::event::Scope(self.scope.as_ref())
            .receipt(kind, (request.id, crate::observation::event::Stage::Sent));
        let mut snapshot = None;
        loop {
            let message = self.connection.receive().map_err(|error| {
                crate::observation::event::Scope(self.scope.as_ref()).receipt(
                    kind,
                    (request.id, crate::observation::event::Stage::Unknown),
                );
                Error::from(error).at(request.id)
            })?;
            if visible {
                self.print(&message)?;
            }
            match message {
                Message::Reply(reply) if reply.request == request.id => {
                    crate::observation::event::Scope(self.scope.as_ref()).receipt(
                        kind,
                        (request.id, crate::observation::event::reply(&reply.result)),
                    );
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
                    crate::observation::event::Scope(self.scope.as_ref()).receipt(
                        kind,
                        (
                            request.id,
                            crate::observation::event::finish(result.is_ok()),
                        ),
                    );
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
        if let Status::Applied(Outcome::Workspace(workspace::Outcome::Entered {
            session,
            change,
        })) = status
        {
            self.entered = Some(*session);
            self.created = change.created.clone();
        }
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
