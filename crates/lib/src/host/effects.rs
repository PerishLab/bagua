use super::{Host, pane::Pane};
use protocol::{Failure, Request, Status, wire::Fault};
use terminal::Extent;
use workspace::{Id, Operation, Target};

pub(super) struct Closing {
    pub actor: Id,
    pub request: u64,
    pub panes: Vec<Id>,
}

impl Host {
    pub(super) fn operation(
        &mut self,
        actor: Id,
        request: u64,
        operation: Operation,
    ) -> Result<(), Failure> {
        let mut world = self.world.clone();
        let reply = crate::evaluate(
            &mut world,
            actor,
            Request::identified(request, protocol::Command::Workspace(operation))?,
        );
        let status = reply.result?;
        validate(&world, &self.panes)?;
        self.world = world;
        let retired = match &status {
            Status::Accepted(protocol::Intent::Retire(change)) => change.retired.clone(),
            _ => Vec::new(),
        };
        self.reply(actor, request, Ok(status))?;
        let result = self.geometry();
        if !retired.is_empty() {
            self.retire(&retired);
            self.closing.push(Closing {
                actor,
                request,
                panes: retired,
            });
            return Ok(());
        }
        self.complete(actor, request, result)
    }

    pub(super) fn extent(&self, id: Id) -> Result<Extent, Failure> {
        let (session, window) = self.world.location(Target::Pane(id))?;
        let session = self.world.session(session).unwrap();
        let window = session.windows().find(|w| Some(w.id()) == window).unwrap();
        let placement = window
            .placements(session.geometry())?
            .into_iter()
            .find(|p| p.pane == id)
            .unwrap();
        Ok(Extent::new(
            placement.rect.geometry.columns(),
            placement.rect.geometry.rows(),
        )
        .unwrap())
    }

    fn geometry(&mut self) -> Result<(), Fault> {
        let ids: Vec<_> = self
            .world
            .snapshot()
            .sessions()
            .flat_map(|s| s.windows())
            .flat_map(|w| w.panes())
            .map(|p| p.id())
            .collect();
        let mut result = Ok(());
        for id in ids {
            let extent = self.extent(id).map_err(Fault::from)?;
            if let std::collections::btree_map::Entry::Vacant(entry) = self.panes.entry(id) {
                entry.insert(Pane::new(extent, self.history).map_err(Fault::from)?);
            }
            let pane = self.panes.get_mut(&id).unwrap();
            if pane.engine.extent() == extent {
                continue;
            }
            pane.engine
                .resize(extent)
                .map_err(|_| Fault::from(Failure::Payload))?;
            if let Some(process) = &pane.process
                && let Err(error) =
                    process.resize(pty::Size::new(extent.width(), extent.height()).unwrap())
            {
                let error = fault(error);
                pane.fault = Some(error.clone());
                result = Err(error);
            }
        }
        result
    }

    fn retire(&mut self, ids: &[Id]) {
        for id in ids {
            if let Some(pane) = self.panes.get_mut(id) {
                pane.retiring = true;
                if let Some(process) = &mut pane.process
                    && let Err(error) = process.retire()
                {
                    let error = fault(error);
                    pane.cleanup = Some(error.clone());
                    pane.fault = Some(error);
                }
            }
        }
    }

    pub(super) fn shutdown(&mut self, actor: Id, request: u64) -> Result<(), Failure> {
        self.reply(actor, request, Ok(Status::Admitted))?;
        self.stopping = Some((actor, request));
        let ids: Vec<_> = self.panes.keys().copied().collect();
        self.retire(&ids);
        Ok(())
    }

    pub(super) fn settle(&mut self) -> Result<(), Failure> {
        let mut remaining = Vec::new();
        for item in std::mem::take(&mut self.closing) {
            if item
                .panes
                .iter()
                .any(|id| self.panes.get(id).is_some_and(|p| p.process.is_some()))
            {
                remaining.push(item);
                continue;
            }
            let error = item
                .panes
                .iter()
                .filter_map(|id| self.panes.remove(id))
                .find_map(|p| p.cleanup);
            self.complete(item.actor, item.request, error.map_or(Ok(()), Err))?;
        }
        self.closing = remaining;
        if self.stopped() && !self.finished {
            let (actor, request) = self.stopping.unwrap();
            let error = self.panes.values().find_map(|pane| pane.cleanup.clone());
            self.complete(actor, request, error.map_or(Ok(()), Err))?;
            self.finished = true;
        }
        Ok(())
    }
}

fn validate(
    world: &workspace::World,
    retained: &std::collections::BTreeMap<Id, Pane>,
) -> Result<(), Failure> {
    let snapshot = world.snapshot();
    let mut count = 0;
    let mut cells = 0;
    for session in snapshot.sessions() {
        let size = session.geometry();
        if size.columns() > super::COLUMNS || size.rows() > super::ROWS {
            return Err(Failure::Capacity);
        }
        for window in session.windows() {
            count += window.panes().count();
            for placement in window.placements(size)? {
                let size = placement.rect.geometry;
                cells += usize::from(size.columns()) * usize::from(size.rows());
            }
        }
    }
    for (id, pane) in retained {
        if world.pane(*id).is_none() {
            count += 1;
            let extent = pane.engine.extent();
            cells += usize::from(extent.width()) * usize::from(extent.height());
        }
    }
    if count > super::PANES || cells > super::CELLS {
        return Err(Failure::Capacity);
    }
    Ok(())
}

pub(super) fn fault(error: pty::Failure) -> Fault {
    Fault {
        code: error.code().into(),
        message: error.to_string().chars().take(512).collect(),
    }
}
