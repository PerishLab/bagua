mod controls;
mod validate;

use protocol::{
    Failure, Frame, Packet, Replica,
    wire::{Message, Screen},
};
use std::collections::BTreeMap;
use terminal::{Extent, Point};
use workspace::{Id, Placement, Session, Snapshot, Target};

pub struct App {
    pub(super) actor: Id,
    pub(super) session: Id,
    pub(super) catalog: Option<Snapshot>,
    pub(super) screens: BTreeMap<Id, Replica<Screen>>,
    pub(super) viewport: Extent,
    pub(super) offset: Point,
    pub(super) selection: Option<crate::select::Selection>,
    pub(super) clipboard: String,
    pub(super) notice: String,
    pub(super) done: bool,
    pub(super) dirty: bool,
}

impl App {
    pub fn new(actor: Id, session: Id, viewport: Extent) -> Self {
        Self {
            actor,
            session,
            viewport: bounded(viewport),
            catalog: None,
            screens: BTreeMap::new(),
            offset: Point::default(),
            selection: None,
            clipboard: String::new(),
            notice: "Ctrl-G ? help".into(),
            done: false,
            dirty: true,
        }
    }

    pub fn actor(&self) -> Id {
        self.actor
    }
    pub fn target(&self) -> Id {
        self.session
    }
    pub fn done(&self) -> bool {
        self.done
    }
    pub fn dirty(&self) -> bool {
        self.dirty
    }
    pub fn clean(&mut self) {
        self.dirty = false;
    }
    pub fn status(&mut self, value: impl Into<String>) {
        self.notice = value.into().chars().take(512).collect();
        self.dirty = true;
    }
    pub fn screen(&self, pane: Id) -> Option<&Screen> {
        if let Some(selection) = &self.selection
            && selection.pane == pane
        {
            return Some(&selection.screen);
        }
        self.screens.get(&pane)?.state()
    }
    pub fn session(&self) -> Option<&Session> {
        self.catalog
            .as_ref()?
            .sessions()
            .find(|session| session.id() == self.session)
    }
    pub fn controller(&self) -> bool {
        self.session()
            .is_some_and(|s| s.controller() == Some(self.actor))
    }
    pub fn focus(&self) -> Option<Id> {
        let session = self.session()?;
        session
            .windows()
            .find(|w| Some(w.id()) == session.selected())
            .map(|w| w.focus())
    }
    pub fn placements(&self) -> Result<Vec<Placement>, Failure> {
        let Some(session) = self.session() else {
            return Ok(Vec::new());
        };
        let Some(window) = session
            .windows()
            .find(|w| Some(w.id()) == session.selected())
        else {
            return Ok(Vec::new());
        };
        Ok(window.placements(session.geometry())?)
    }

    pub fn missing(&self) -> Vec<Id> {
        self.session()
            .into_iter()
            .flat_map(|s| s.windows())
            .flat_map(|w| w.panes())
            .map(|pane| pane.id())
            .filter(|id| !self.screens.contains_key(id))
            .collect()
    }

    pub fn resize(&mut self, extent: Extent) {
        self.viewport = bounded(extent);
        self.clamp();
        self.dirty = true;
    }

    pub fn accept(&mut self, message: Message) -> Result<(), Failure> {
        match message {
            Message::Catalog(snapshot) => self.catalog(snapshot)?,
            Message::Screen { pane, state } => self.install(pane, state)?,
            Message::Reply(reply) => {
                if let Err(error) = reply.result {
                    self.status(error.message());
                }
            }
            Message::Finished {
                result: Err(fault), ..
            } => self.status(fault.message),
            Message::Process {
                pane,
                exited: true,
                code,
                signal,
                ..
            } => {
                self.status(format!(
                    "pane {} exited: code={code:?} signal={signal:?}",
                    pane.value()
                ));
            }
            Message::Process {
                fault: Some(fault), ..
            } => self.status(fault.message),
            Message::Hello { .. } => return Err(Failure::Version),
            _ => {}
        }
        Ok(())
    }

    pub fn catalog(&mut self, snapshot: Snapshot) -> Result<(), Failure> {
        let focus = self.focus();
        validate::catalog(&snapshot)?;
        if self
            .catalog
            .as_ref()
            .is_some_and(|current| current.revision() > snapshot.revision())
        {
            return Ok(());
        }
        let Some(session) = snapshot
            .sessions()
            .find(|session| session.id() == self.session)
        else {
            self.done = true;
            self.screens.clear();
            self.catalog = Some(snapshot);
            return Ok(());
        };
        let mut extents = BTreeMap::new();
        for window in session.windows() {
            for placement in window.placements(session.geometry())? {
                let size = placement.rect.geometry;
                extents.insert(
                    placement.pane,
                    Extent::new(size.columns(), size.rows()).unwrap(),
                );
            }
        }
        self.screens
            .retain(|id, replica| match (extents.get(id), replica.state()) {
                (Some(extent), Some(screen)) => *extent == screen.extent,
                (Some(_), None) => true,
                _ => false,
            });
        self.catalog = Some(snapshot);
        self.clamp();
        if focus != self.focus() {
            self.selection = None;
        }
        if let Some(selection) = &self.selection
            && extents.get(&selection.pane) != Some(&selection.screen.extent)
        {
            self.selection = None;
        }
        self.dirty = true;
        Ok(())
    }

    fn install(&mut self, pane: Id, screen: Screen) -> Result<(), Failure> {
        validate::screen(&screen)?;
        let Some(session) = self.session() else {
            return Ok(());
        };
        let Some(window) = session
            .windows()
            .find(|w| w.panes().any(|p| p.id() == pane))
        else {
            return Ok(());
        };
        let expected = window
            .placements(session.geometry())?
            .into_iter()
            .find(|p| p.pane == pane)
            .map(|p| p.rect.geometry);
        let Some(size) = expected else { return Ok(()) };
        if (size.columns(), size.rows()) != (screen.extent.width(), screen.extent.height()) {
            return Ok(());
        }
        let replica = self
            .screens
            .entry(pane)
            .or_insert_with(|| Replica::new(Target::Pane(pane)));
        if replica
            .revision()
            .is_some_and(|revision| revision >= screen.revision)
        {
            return Ok(());
        }
        replica.apply(
            Packet::<_, ()>::Snapshot(Frame::snapshot(Target::Pane(pane), screen)),
            |_, ()| Err(Failure::Baseline),
        )?;
        self.dirty = true;
        Ok(())
    }
}

fn bounded(extent: Extent) -> Extent {
    Extent::new(extent.width().clamp(2, 512), extent.height().min(255)).unwrap()
}
