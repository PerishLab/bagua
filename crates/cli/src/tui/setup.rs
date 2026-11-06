use super::runner::Runner;
use crate::{
    args::{Action, Args, Launch},
    error::Error,
    session::Session,
};
use protocol::{Command, Failure, wire::Program};
use std::path::Path;
use workspace::{Geometry, Operation, Target};

pub(super) fn prepare(
    path: &Path,
    args: &Args,
    terminal: &client::Terminal,
) -> Result<Runner, Error> {
    let (columns, rows) = terminal.size()?;
    let viewport = super::extent(columns, rows).ok_or(Failure::Payload)?;
    let mut rpc = Session::connect(path, args)?;
    let geometry = Geometry::new(
        viewport.width().min(rpc.canvas.columns()),
        viewport.height().min(rpc.canvas.rows()),
    )
    .unwrap();
    let (session, launch) = match &args.action {
        Action::Open { launch } => {
            let program = program(launch)?.ok_or(Failure::Payload)?;
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
            take,
            fixed,
            launch,
        } => {
            let session = Command::identity(*session)?;
            join(&mut rpc, session, geometry)?;
            if *take {
                acquire(&mut rpc, session, *fixed)?;
            }
            (session, program(launch)?)
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
    Ok(Runner::new(rpc.channel()?, app, next, launch))
}

fn join(rpc: &mut Session, session: workspace::Id, viewport: Geometry) -> Result<(), Error> {
    rpc.call(
        Command::Workspace(Operation::Join { session, viewport }),
        false,
    )?;
    Ok(())
}

fn program(launch: &Launch) -> Result<Option<Program>, Failure> {
    let Some(executable) = &launch.executable else {
        return Ok(None);
    };
    let environment = launch
        .environment
        .iter()
        .map(|entry| {
            let (key, value) = entry.split_once('=').ok_or(Failure::Payload)?;
            Ok((key.into(), value.into()))
        })
        .collect::<Result<_, Failure>>()?;
    Ok(Some(Program {
        executable: executable.clone(),
        args: launch.args.clone(),
        cwd: launch.cwd.clone().ok_or(Failure::Payload)?,
        environment,
    }))
}

fn acquire(rpc: &mut Session, session: workspace::Id, fixed: bool) -> Result<(), Error> {
    let fixed = if fixed {
        let snapshot = rpc
            .call(Command::Workspace(Operation::Read), false)?
            .ok_or(Failure::Payload)?;
        Some(
            snapshot
                .sessions()
                .find(|s| s.id() == session)
                .ok_or(Failure::Target)?
                .geometry(),
        )
    } else {
        None
    };
    rpc.call(
        Command::Workspace(Operation::Acquire { session, fixed }),
        false,
    )?;
    Ok(())
}
