use crate::args::{Action, Axis, Dimensions, Kind};
use protocol::{Command, Failure, Input};
use workspace::{Geometry, Operation, Ratio, Sizing, Split, Target};

pub(crate) fn parse(action: &Action) -> Result<Command, Failure> {
    let operation = match action {
        Action::Open { .. } | Action::Connect { .. } | Action::Serve | Action::Control => {
            return Err(Failure::Unavailable);
        }
        Action::Shutdown => return Ok(Command::Shutdown),
        Action::Inspect { pane } => return Ok(Command::Inspect(Command::identity(*pane)?)),
        Action::Start {
            pane,
            executable,
            cwd,
            args,
            environment,
        } => {
            return Ok(Command::Start {
                pane: Command::identity(*pane)?,
                program: protocol::wire::Program {
                    executable: executable.clone(),
                    cwd: cwd.clone(),
                    args: args.clone(),
                    environment: environment
                        .iter()
                        .map(|value| {
                            let (key, value) = value.split_once('=').ok_or(Failure::Payload)?;
                            Ok((key.into(), value.into()))
                        })
                        .collect::<Result<_, Failure>>()?,
                },
            });
        }
        Action::Key { pane, key } => {
            return Ok(Command::Send {
                pane: Command::identity(*pane)?,
                input: Input::Key(key.convert()),
            });
        }
        Action::Capabilities => return Ok(Command::Capabilities),
        Action::Watch { kind, id } => {
            return Ok(Command::Subscribe {
                target: target(*kind, *id)?,
                after: None,
            });
        }
        Action::Unwatch { kind, id } => return Ok(Command::Unsubscribe(target(*kind, *id)?)),
        Action::New { size } => Operation::Create(geometry(size)?),
        Action::List => Operation::Read,
        Action::Attach { session, size } => Operation::Join {
            session: Command::identity(*session)?,
            viewport: geometry(size)?,
        },
        Action::Acquire {
            session,
            fixed,
            columns,
            rows,
        } => Operation::Acquire {
            session: Command::identity(*session)?,
            fixed: acquisition(*fixed, *columns, *rows)?,
        },
        Action::Transfer { session, client } => Operation::Transfer {
            session: Command::identity(*session)?,
            next: Command::identity(*client)?,
        },
        Action::Detach { session } => Operation::Detach(Command::identity(*session)?),
        Action::Window { session } => Operation::Window(Command::identity(*session)?),
        Action::Split { pane, axis, ratio } => Operation::Split {
            pane: Command::identity(*pane)?,
            split: Split {
                axis: match axis {
                    Axis::Columns => workspace::Axis::Columns,
                    Axis::Rows => workspace::Axis::Rows,
                },
                ratio: Ratio::new(*ratio)
                    .ok_or(Failure::Workspace(workspace::Failure::Geometry))?,
            },
        },
        Action::Focus { pane } => Operation::Focus(Command::identity(*pane)?),
        Action::Close { kind, id } => Operation::Close(target(*kind, *id)?),
        Action::Resize { session, size } => Operation::Resize {
            session: Command::identity(*session)?,
            geometry: geometry(size)?,
        },
        Action::Size {
            session,
            fixed,
            size,
        } => Operation::Size {
            session: Command::identity(*session)?,
            sizing: if *fixed {
                Sizing::Fixed(geometry(size)?)
            } else {
                Sizing::Automatic(geometry(size)?)
            },
        },
        Action::Send { pane, text, paste } => {
            return Ok(Command::Send {
                pane: Command::identity(*pane)?,
                input: if *paste {
                    Input::Paste(text.clone())
                } else {
                    Input::Text(text.clone())
                },
            });
        }
    };
    Ok(Command::Workspace(operation))
}

fn geometry(size: &Dimensions) -> Result<Geometry, Failure> {
    Command::geometry(size.columns, size.rows)
}

fn acquisition(
    fixed: bool,
    columns: Option<u16>,
    rows: Option<u16>,
) -> Result<Option<Geometry>, Failure> {
    if !fixed {
        return Ok(None);
    }
    let (columns, rows) = columns
        .zip(rows)
        .ok_or(Failure::Workspace(workspace::Failure::Geometry))?;
    Ok(Some(Command::geometry(columns, rows)?))
}

fn target(kind: Kind, value: u64) -> Result<Target, Failure> {
    let id = Command::identity(value)?;
    Ok(match kind {
        Kind::Session => Target::Session(id),
        Kind::Window => Target::Window(id),
        Kind::Pane => Target::Pane(id),
    })
}

impl crate::args::Key {
    fn convert(self) -> terminal::Key {
        use crate::args::Key;
        match self {
            Key::Enter => terminal::Key::Enter,
            Key::Tab => terminal::Key::Tab,
            Key::Escape => terminal::Key::Escape,
            Key::Backspace => terminal::Key::Backspace,
            Key::Up => terminal::Key::Up,
            Key::Down => terminal::Key::Down,
            Key::Left => terminal::Key::Left,
            Key::Right => terminal::Key::Right,
            Key::Home => terminal::Key::Home,
            Key::End => terminal::Key::End,
        }
    }
}
