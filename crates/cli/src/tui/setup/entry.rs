use super::{Action, Command, Error, Failure, Geometry, Operation, Session, Target};
use protocol::wire::Program;
use workspace::{Entry, Id, State};

pub(super) fn prepare(
    rpc: &mut Session,
    action: &Action,
    viewport: Geometry,
    config: &bagua::config::Workspace,
) -> Result<(Id, Option<Program>), Error> {
    let Action::Enter {
        name,
        existing,
        observe,
        fixed,
        launch,
    } = action
    else {
        return Err(Failure::Payload.into());
    };
    let program = crate::config::launch(launch)?;
    if !observe && program.is_none() {
        return Err(Failure::Payload.into());
    }
    rpc.call(
        Command::Workspace(Operation::Enter(Entry {
            name: name.as_ref().unwrap_or(&config.name).clone(),
            viewport,
            create: !existing && !observe,
            observe: *observe,
            fixed: *fixed,
        })),
        false,
    )?;
    let session = rpc.entered.take().ok_or(Failure::Payload)?;
    if !observe {
        start(rpc, session, program.as_ref().unwrap())?;
    }
    Ok((session, program))
}

fn start(rpc: &mut Session, session: Id, program: &Program) -> Result<(), Error> {
    let snapshot = rpc
        .call(Command::Workspace(Operation::Read), false)?
        .ok_or(Failure::Payload)?;
    let current = snapshot
        .sessions()
        .find(|current| current.id() == session)
        .ok_or(Failure::Target)?;
    let selected = current
        .windows()
        .find(|window| Some(window.id()) == current.selected());
    let pane = match selected {
        Some(window) => {
            let pane = window
                .panes()
                .find(|pane| pane.id() == window.focus())
                .ok_or(Failure::Target)?;
            if pane.state() != State::Pending {
                return Ok(());
            }
            pane.id()
        }
        None => {
            rpc.call(Command::Workspace(Operation::Window(session)), false)?;
            rpc.created
                .iter()
                .find_map(|target| match target {
                    Target::Pane(pane) => Some(*pane),
                    _ => None,
                })
                .ok_or(Failure::Payload)?
        }
    };
    rpc.call(
        Command::Start {
            pane,
            program: program.clone(),
        },
        false,
    )?;
    Ok(())
}
