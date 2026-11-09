mod context;
mod controls;
mod feedback;
mod history;
mod install;
pub(super) mod preferences;
mod timing;
mod validate;

use protocol::{
    Failure, Replica,
    wire::{Message, Screen},
};
use std::collections::BTreeMap;
use terminal::{Extent, Point};
use workspace::{Id, Placement, Session, Snapshot};

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
    pub(super) urgent: bool,
    pub(super) expires: Option<std::time::Instant>,
    pub(super) recovering: bool,
    pub(super) done: bool,
    pub(super) dirty: bool,
    past: history::Past,
    pub(super) mouse: crate::Mouse,
    pub(super) copying: crate::Clipboard,
    pub(super) pointer: Option<crate::mouse::Pointer>,
    pub(super) copies: crate::select::Copies,
    pub(super) decor: crate::Decor,
    pub(super) clock: String,
    pub(super) now: std::time::Instant,
    pub(super) next: Option<std::time::Instant>,
    pub(super) leader: Option<String>,
    pub(super) records: BTreeMap<Id, context::Record>,
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
            notice: "Ready".into(),
            urgent: false,
            expires: Some(std::time::Instant::now() + std::time::Duration::from_secs(2)),
            recovering: false,
            done: false,
            dirty: true,
            past: history::Past::default(),
            mouse: crate::Mouse::default(),
            copying: crate::Clipboard::default(),
            pointer: None,
            copies: crate::select::Copies::default(),
            decor: crate::Decor::default(),
            clock: "--:--".into(),
            now: std::time::Instant::now(),
            next: None,
            leader: None,
            records: BTreeMap::new(),
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
    pub fn screen(&self, pane: Id) -> Option<&Screen> {
        if let Some(selection) = &self.selection
            && selection.pane == pane
        {
            return Some(&selection.screen);
        }
        if let Some(view) = self.history(pane) {
            return Some(&view.screen);
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
    pub fn revision(&self) -> Option<u64> {
        self.catalog.as_ref().map(Snapshot::revision)
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
        Ok(window.placements(session.canvas())?)
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
            Message::History { request, state } => self.past(request, state)?,
            Message::Catalog(snapshot) => self.catalog(snapshot)?,
            Message::Screen { pane, state } => self.install(pane, state)?,
            Message::Reply(reply) => {
                if let Err(error) = reply.result {
                    self.finished(reply.request);
                    self.error(error.message());
                }
            }
            Message::Finished { request, result } => {
                self.finished(request);
                if let Err(fault) = result {
                    self.error(fault.message);
                }
            }
            Message::Process {
                pane,
                exited: true,
                code,
                signal,
                ..
            } => {
                self.records.remove(&pane);
                self.status(format!(
                    "pane {} exited: code={code:?} signal={signal:?}",
                    pane.value()
                ));
            }
            Message::Process {
                pane,
                fault: Some(fault),
                ..
            } => {
                self.records.remove(&pane);
                self.error(fault.message);
            }
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
            self.records.clear();
            self.cancel();
            self.live();
            self.catalog = Some(snapshot);
            return Ok(());
        };
        let mut extents = BTreeMap::new();
        for window in session.windows() {
            for placement in window.placements(session.canvas())? {
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
        self.retain();
        self.pointer();
        self.clamp();
        if focus != self.focus() && (self.past.active() || self.selection.is_some()) {
            self.live();
        }
        if let Some(view) = &self.past.view
            && extents.get(&view.scope.pane) != Some(&view.screen.extent)
        {
            self.live();
        }
        if let Some(selection) = &self.selection
            && extents.get(&selection.pane) != Some(&selection.screen.extent)
        {
            self.selection = None;
        }
        self.dirty = true;
        Ok(())
    }
}

fn bounded(extent: Extent) -> Extent {
    Extent::new(extent.width().clamp(2, 512), extent.height().min(255)).unwrap()
}
