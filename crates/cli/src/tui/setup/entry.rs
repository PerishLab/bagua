use super::{
    Action, Command, Display, Error, Failure, Geometry, Operation, Session, Target, choice,
};
use protocol::wire::Program;
use workspace::{Id, State};

pub(super) struct Entry<'a, 'b> {
    pub rpc: &'a mut Session,
    pub args: &'a crate::args::Args,
    pub display: &'a mut Display<'b>,
    pub config: &'a crate::config::Loaded,
}

struct Joined {
    session: Id,
    control: bool,
}

impl Entry<'_, '_> {
    pub fn prepare(&mut self, viewport: Geometry) -> Result<Option<(Id, Option<Program>)>, Error> {
        let Action::Enter {
            name,
            existing,
            fixed,
            launch,
        } = self.args.selected()
        else {
            return Err(Failure::Payload.into());
        };
        let name = name
            .as_ref()
            .unwrap_or(&self.config.workspace().name)
            .clone();
        let program = crate::config::launch(launch)?;
        if !self.args.observe && program.is_none() {
            return Err(Failure::Payload.into());
        }
        let result = self.rpc.call(
            Command::Workspace(Operation::Enter(workspace::Entry {
                name: name.clone(),
                viewport,
                create: !existing && !self.args.observe,
                observe: self.args.observe,
                fixed: *fixed,
            })),
            false,
        );
        match result {
            Ok(_) => {}
            Err(error) if error.refused(Failure::Workspace(workspace::Failure::Busy)) => {
                let Some(session) = self.occupied(&name, viewport, *fixed)? else {
                    return Ok(None);
                };
                if session.control {
                    start(
                        self.rpc,
                        session.session,
                        program.as_ref().ok_or(Failure::Payload)?,
                    )?;
                }
                return Ok(Some((session.session, program)));
            }
            Err(error) => return Err(error),
        }
        let session = self.rpc.entered.take().ok_or(Failure::Payload)?;
        if !self.args.observe {
            start(self.rpc, session, program.as_ref().unwrap())?;
        }
        Ok(Some((session, program)))
    }

    fn occupied(
        &mut self,
        name: &str,
        viewport: Geometry,
        fixed: bool,
    ) -> Result<Option<Joined>, Error> {
        let snapshot = self
            .rpc
            .call(Command::Workspace(Operation::Read), false)?
            .ok_or(Failure::Payload)?;
        let current = snapshot
            .sessions()
            .find(|session| session.name() == name)
            .ok_or(Failure::Target)?;
        let session = current.id();
        let expected = current.controller();
        let take = if let Some(owner) = expected.filter(|_| !self.args.take) {
            let size = current.viewport(owner).ok_or(Failure::Payload)?;
            let mut app = client::App::new(
                self.rpc.actor,
                session,
                terminal::Extent::new(viewport.columns(), viewport.rows()).unwrap(),
            );
            app.catalog(snapshot.clone())?;
            app.decorate(self.config.decor().clone());
            let caption = format!(
                "[t] Take [o] Observe [q/Esc] Cancel | owner #{} {}x{}",
                owner.value(),
                size.columns(),
                size.rows()
            );
            match choice::choose(self.display, &mut app, &caption, self.config)? {
                choice::Choice::Take => true,
                choice::Choice::Observe => false,
                choice::Choice::Cancel => return Ok(None),
            }
        } else {
            true
        };
        let (columns, rows) = self.display.terminal.size()?;
        let viewport =
            super::super::extent(columns, rows, self.display.separator).ok_or(Failure::Payload)?;
        let viewport = Geometry::new(
            viewport.width().min(self.rpc.canvas.columns()),
            viewport.height().min(self.rpc.canvas.rows()),
        )
        .unwrap();
        self.rpc.call(
            Command::Workspace(Operation::Join { session, viewport }),
            false,
        )?;
        if take {
            self.rpc
                .call(
                    Command::Workspace(Operation::Take {
                        session,
                        expected,
                        fixed: fixed.then_some(current.geometry()),
                    }),
                    false,
                )
                .map_err(Error::taking)?;
            Ok(Some(Joined {
                session,
                control: true,
            }))
        } else {
            Ok(Some(Joined {
                session,
                control: false,
            }))
        }
    }
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
