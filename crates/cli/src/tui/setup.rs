use super::runner::Runner;
mod choice;
mod entry;
use crate::{
    args::{Action, Args},
    error::Error,
    session::Session,
};
use protocol::{Command, Failure};
use workspace::{Geometry, Operation, Target};

pub(super) struct Display<'a> {
    pub terminal: &'a mut client::Terminal,
    pub stop: &'a std::sync::atomic::AtomicBool,
    pub rows: u16,
}

pub(super) fn prepare(
    mut rpc: Session,
    args: &Args,
    display: &mut Display<'_>,
    config: &crate::config::Loaded,
) -> Result<Option<Runner>, Error> {
    let (columns, rows) = display.terminal.size()?;
    let viewport =
        super::extent(columns, rows, config.decor().status.rows()).ok_or(Failure::Payload)?;
    let geometry = Geometry::new(
        viewport.width().min(rpc.canvas.columns()),
        viewport.height().min(rpc.canvas.rows()),
    )
    .unwrap();
    let (session, launch) = match args.selected() {
        Action::Enter { .. } => {
            let Some(entered) = (entry::Entry {
                rpc: &mut rpc,
                args,
                display,
                config,
            })
            .prepare(geometry)?
            else {
                return Ok(None);
            };
            entered
        }
        Action::Open { launch } => {
            let program = crate::config::launch(launch)?.ok_or(Failure::Payload)?;
            rpc.call(Command::Workspace(Operation::Create(geometry)), false)?;
            let session = rpc
                .created
                .iter()
                .find_map(|target| match target {
                    Target::Session(id) => Some(*id),
                    _ => None,
                })
                .ok_or(Failure::Payload)?;
            let pane = rpc
                .created
                .iter()
                .find_map(|target| match target {
                    Target::Pane(id) => Some(*id),
                    _ => None,
                })
                .ok_or(Failure::Payload)?;
            join(&mut rpc, session, geometry)?;
            rpc.call(
                Command::Workspace(Operation::Acquire {
                    session,
                    fixed: None,
                }),
                false,
            )?;
            rpc.call(
                Command::Start {
                    pane,
                    program: program.clone(),
                },
                false,
            )?;
            (session, Some(program))
        }
        Action::Connect {
            session,
            fixed,
            launch,
        } => {
            let session = Command::identity(*session)?;
            join(&mut rpc, session, geometry)?;
            if args.take {
                acquire(&mut rpc, session, *fixed)?;
            }
            (session, crate::config::launch(launch)?)
        }
        _ => return Err(Failure::Unavailable.into()),
    };
    let snapshot = rpc
        .call(Command::Workspace(Operation::Read), false)?
        .ok_or(Failure::Payload)?;
    let mut app = client::App::new(rpc.actor, session, viewport);
    app.catalog(snapshot)?;
    rpc.call(
        Command::Subscribe {
            target: Target::Session(session),
            after: None,
        },
        false,
    )?;
    let next = rpc.next;
    let scope = rpc.scope.clone();
    Ok(Some(
        Runner::new(rpc.channel()?, app, next, launch).scoped(scope),
    ))
}

fn join(rpc: &mut Session, session: workspace::Id, viewport: Geometry) -> Result<(), Error> {
    rpc.call(
        Command::Workspace(Operation::Join { session, viewport }),
        false,
    )?;
    Ok(())
}

fn acquire(rpc: &mut Session, session: workspace::Id, fixed: bool) -> Result<(), Error> {
    let snapshot = rpc
        .call(Command::Workspace(Operation::Read), false)?
        .ok_or(Failure::Payload)?;
    let current = snapshot
        .sessions()
        .find(|current| current.id() == session)
        .ok_or(Failure::Target)?;
    rpc.call(
        Command::Workspace(Operation::Take {
            session,
            expected: current.controller(),
            fixed: fixed.then_some(current.geometry()),
        }),
        false,
    )
    .map_err(Error::taking)?;
    Ok(())
}
