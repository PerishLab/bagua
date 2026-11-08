use crate::{Axis, Geometry, Id, Ratio, Rect, Split};

#[derive(serde::Serialize, serde::Deserialize, Clone, Copy, Debug, PartialEq, Eq)]
pub struct Divider {
    pub first: Id,
    pub second: Id,
    pub split: Split,
    pub rect: Rect,
}

impl Divider {
    pub fn contains(self, x: u16, y: u16) -> bool {
        let Ok((first, _)) = self.rect.split(self.split) else {
            return false;
        };
        match self.split.axis {
            Axis::Columns => {
                x == first.x + first.geometry.columns()
                    && (self.rect.y..self.rect.y + self.rect.geometry.rows()).contains(&y)
            }
            Axis::Rows => {
                y == first.y + first.geometry.rows()
                    && (self.rect.x..self.rect.x + self.rect.geometry.columns()).contains(&x)
            }
        }
    }

    pub fn ratio(self, position: u16) -> Option<Ratio> {
        let (origin, length) = match self.split.axis {
            Axis::Columns => (self.rect.x, self.rect.geometry.columns()),
            Axis::Rows => (self.rect.y, self.rect.geometry.rows()),
        };
        let first = position.checked_sub(origin)?;
        let available = length.checked_sub(1)?;
        if available == 0 {
            return None;
        }
        Ratio::new(
            (u32::from(first) * 1000)
                .div_ceil(u32::from(available))
                .try_into()
                .ok()?,
        )
    }
}

impl super::Tree {
    pub fn dividers(&self, geometry: Geometry) -> Result<Vec<Divider>, crate::Failure> {
        self.placements(geometry)?;
        let mut output = Vec::new();
        self.divisions(
            Rect {
                x: 0,
                y: 0,
                geometry,
            },
            &mut output,
        )?;
        Ok(output)
    }

    fn divisions(&self, rect: Rect, output: &mut Vec<Divider>) -> Result<(), crate::Failure> {
        if let Self::Split {
            split,
            first,
            second,
        } = self
        {
            output.push(Divider {
                first: first.anchor(),
                second: second.anchor(),
                split: *split,
                rect,
            });
            let (left, right) = rect.split(*split)?;
            first.divisions(left, output)?;
            second.divisions(right, output)?;
        }
        Ok(())
    }

    fn anchor(&self) -> Id {
        match self {
            Self::Pane(id) => *id,
            Self::Split { first, second, .. } => first.anchor().min(second.anchor()),
        }
    }

    pub fn adjust(&mut self, divider: Divider, ratio: Ratio) -> bool {
        let Self::Split {
            split,
            first,
            second,
        } = self
        else {
            return false;
        };
        if first.anchor() == divider.first && second.anchor() == divider.second {
            split.ratio = ratio;
            return true;
        }
        first.adjust(divider, ratio) || second.adjust(divider, ratio)
    }
}
