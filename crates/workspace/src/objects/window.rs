use crate::{Failure, Geometry, Id, Pane, Placement, Split, layout::Tree};
use std::collections::BTreeMap;

#[derive(serde::Serialize, serde::Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct Window {
    id: Id,
    focus: Id,
    tree: Tree,
    pub(crate) panes: BTreeMap<Id, Pane>,
}

impl Window {
    pub(crate) fn new(id: Id, pane: Id) -> Self {
        Self {
            id,
            focus: pane,
            tree: Tree::Pane(pane),
            panes: [(pane, Pane::new(pane))].into(),
        }
    }

    pub fn id(&self) -> Id {
        self.id
    }

    pub fn focus(&self) -> Id {
        self.focus
    }

    pub fn panes(&self) -> impl Iterator<Item = &Pane> {
        self.panes.values()
    }

    pub fn placements(&self, geometry: Geometry) -> Result<Vec<Placement>, Failure> {
        self.tree.placements(geometry)
    }

    pub(crate) fn select(&mut self, pane: Id) -> Result<(), Failure> {
        if !self.tree.contains(pane) {
            return Err(Failure::Missing);
        }
        self.focus = pane;
        Ok(())
    }

    pub(crate) fn split(
        &mut self,
        pane: Id,
        next: Id,
        split: Split,
        geometry: Geometry,
    ) -> Result<(), Failure> {
        if self.panes.len() >= 64 {
            return Err(Failure::Capacity);
        }
        let mut tree = self.tree.clone();
        if !tree.insert(pane, next, split) {
            return Err(Failure::Missing);
        }
        if tree.depth() > 16 {
            return Err(Failure::Capacity);
        }
        tree.placements(geometry)?;
        self.tree = tree;
        self.panes.insert(next, Pane::new(next));
        self.focus = next;
        Ok(())
    }

    pub(crate) fn remove(&mut self, pane: Id) {
        self.panes.remove(&pane);
        if let Some(tree) = self.tree.remove(pane) {
            self.tree = tree;
        }
        if self.focus == pane
            && let Some(id) = self.panes.keys().next()
        {
            self.focus = *id;
        }
    }
}
