use crate::{Canvas, Failure, Id, Pane, Placement, Split, layout::Tree};
use std::collections::BTreeMap;

#[derive(serde::Serialize, serde::Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct Window {
    id: Id,
    pub(crate) name: String,
    focus: Id,
    tree: Tree,
    pub(crate) panes: BTreeMap<Id, Pane>,
}

impl Window {
    pub(crate) fn new(id: Id, pane: Id, name: String) -> Self {
        Self {
            id,
            name,
            focus: pane,
            tree: Tree::Pane(pane),
            panes: [(pane, Pane::new(pane))].into(),
        }
    }

    pub fn id(&self) -> Id {
        self.id
    }
    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn focus(&self) -> Id {
        self.focus
    }

    pub fn panes(&self) -> impl Iterator<Item = &Pane> {
        self.panes.values()
    }

    pub fn placements(&self, canvas: impl Into<Canvas>) -> Result<Vec<Placement>, Failure> {
        self.tree.placements(canvas.into())
    }

    pub fn dividers(&self, canvas: impl Into<Canvas>) -> Result<Vec<crate::Divider>, Failure> {
        self.tree.dividers(canvas.into())
    }

    pub fn preview(
        &self,
        divider: crate::Divider,
        ratio: crate::Ratio,
        geometry: Canvas,
    ) -> Result<Vec<Placement>, Failure> {
        self.adjusted(divider, ratio, geometry)?
            .placements(geometry)
    }

    pub(crate) fn adjust(
        &mut self,
        divider: crate::Divider,
        ratio: crate::Ratio,
        geometry: Canvas,
    ) -> Result<(), Failure> {
        self.tree = self.adjusted(divider, ratio, geometry)?;
        Ok(())
    }

    fn adjusted(
        &self,
        divider: crate::Divider,
        ratio: crate::Ratio,
        geometry: Canvas,
    ) -> Result<Tree, Failure> {
        if !self.tree.dividers(geometry)?.contains(&divider) {
            return Err(Failure::State);
        }
        let mut next = self.tree.clone();
        if !next.adjust(divider, ratio) {
            return Err(Failure::Missing);
        }
        next.placements(geometry)?;
        Ok(next)
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
        geometry: Canvas,
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
