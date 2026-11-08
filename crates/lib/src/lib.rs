pub mod config;
mod host;
pub mod ipc;
mod profile;

pub use host::{COLUMNS, Host, ROWS};
pub use profile::{EXCLUDED, profile};
use protocol::{Command, Failure, Profile, Request};

pub fn dispatch(request: Request) -> Result<Profile, Failure> {
    match request.check()? {
        Command::Capabilities => Ok(profile()),
        Command::Start { .. }
        | Command::Inspect(_)
        | Command::Describe(_)
        | Command::History { .. }
        | Command::Shutdown
        | Command::Workspace(_)
        | Command::Send { .. }
        | Command::Subscribe { .. }
        | Command::Unsubscribe(_) => Err(Failure::Unavailable),
    }
}

pub fn evaluate(
    world: &mut workspace::World,
    actor: workspace::Id,
    request: Request,
) -> protocol::Reply {
    let id = request.id;
    protocol::Reply {
        request: id,
        result: apply(world, actor, request),
    }
}

fn apply(
    world: &mut workspace::World,
    actor: workspace::Id,
    request: Request,
) -> Result<protocol::Status, Failure> {
    use protocol::{Intent, Outcome, Status};
    match request.check()? {
        Command::Capabilities => Ok(Status::Applied(Outcome::Profile(profile()))),
        Command::Start { .. }
        | Command::Inspect(_)
        | Command::Describe(_)
        | Command::History { .. }
        | Command::Shutdown => Err(Failure::Unavailable),
        Command::Subscribe { target, after } => {
            world.watch(actor, target)?;
            Ok(Status::Accepted(Intent::Subscribe { target, after }))
        }
        Command::Unsubscribe(target) => Ok(Status::Accepted(Intent::Unsubscribe(target))),
        Command::Workspace(operation) => Ok(applied(world.apply(actor, operation)?)),
        Command::Send { pane, input } => {
            world.input(actor, pane)?;
            input.check()?;
            Ok(Status::Accepted(Intent::Input { pane, input }))
        }
    }
}

fn applied(outcome: workspace::Outcome) -> protocol::Status {
    use protocol::{Intent, Outcome, Status};
    match outcome {
        workspace::Outcome::Changed(change) if !change.retired.is_empty() => {
            Status::Accepted(Intent::Retire(change))
        }
        outcome => Status::Applied(Outcome::Workspace(outcome)),
    }
}
