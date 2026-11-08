use crate::{Canvas, Failure, Geometry, Id, Placement, Rect, Split};

#[derive(serde::Serialize, serde::Deserialize, Clone, Debug, PartialEq, Eq)]
pub(crate) enum Tree {
    Pane(Id),
    Split {
        split: Split,
        first: Box<Self>,
        second: Box<Self>,
    },
}

impl Tree {
    pub fn insert(&mut self, pane: Id, next: Id, split: Split) -> bool {
        match self {
            Self::Pane(id) if *id == pane => {
                *self = Self::Split {
                    split,
                    first: Box::new(Self::Pane(pane)),
                    second: Box::new(Self::Pane(next)),
                };
                true
            }
            Self::Pane(_) => false,
            Self::Split { first, second, .. } => {
                first.insert(pane, next, split) || second.insert(pane, next, split)
            }
        }
    }

    pub fn contains(&self, pane: Id) -> bool {
        match self {
            Self::Pane(id) => *id == pane,
            Self::Split { first, second, .. } => first.contains(pane) || second.contains(pane),
        }
    }

    pub fn remove(&self, pane: Id) -> Option<Self> {
        match self {
            Self::Pane(id) => (*id != pane).then_some(self.clone()),
            Self::Split {
                split,
                first,
                second,
            } => match (first.remove(pane), second.remove(pane)) {
                (Some(first), Some(second)) => Some(Self::Split {
                    split: *split,
                    first: Box::new(first),
                    second: Box::new(second),
                }),
                (first, second) => first.or(second),
            },
        }
    }

    pub fn depth(&self) -> usize {
        match self {
            Self::Pane(_) => 1,
            Self::Split { first, second, .. } => 1 + first.depth().max(second.depth()),
        }
    }

    pub fn placements(&self, canvas: Canvas) -> Result<Vec<Placement>, Failure> {
        let geometry = canvas.geometry;
        if !geometry.valid() {
            return Err(Failure::Geometry);
        }
        let mut placements = Vec::new();
        self.collect(
            Rect {
                x: 0,
                y: 0,
                geometry,
            },
            &mut placements,
            canvas.header,
        )?;
        Ok(placements)
    }

    fn collect(
        &self,
        rect: Rect,
        output: &mut Vec<Placement>,
        header: bool,
    ) -> Result<(), Failure> {
        match self {
            Self::Pane(pane) => {
                let top = u16::from(header);
                let height = rect
                    .geometry
                    .rows()
                    .checked_sub(top)
                    .ok_or(Failure::Geometry)?;
                let geometry =
                    Geometry::new(rect.geometry.columns(), height).ok_or(Failure::Geometry)?;
                output.push(Placement {
                    pane: *pane,
                    frame: rect,
                    rect: Rect {
                        y: rect.y + top,
                        geometry,
                        ..rect
                    },
                });
            }
            Self::Split {
                split,
                first,
                second,
            } => {
                let (left, right) = rect.split(*split)?;
                first.collect(left, output, header)?;
                second.collect(right, output, header)?;
            }
        }
        Ok(())
    }
}
