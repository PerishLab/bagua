use crate::{Failure, Geometry, Id};

#[derive(serde::Serialize, serde::Deserialize, Clone, Copy, Debug, PartialEq, Eq)]
pub enum Axis {
    Columns,
    Rows,
}

#[derive(serde::Serialize, serde::Deserialize, Clone, Copy, Debug, PartialEq, Eq)]
pub struct Ratio(#[serde(deserialize_with = "ratio")] u16);

impl Ratio {
    pub fn new(value: u16) -> Option<Self> {
        (1..1000).contains(&value).then_some(Self(value))
    }

    pub fn value(self) -> u16 {
        self.0
    }
}

impl Default for Ratio {
    fn default() -> Self {
        Self(500)
    }
}

#[derive(serde::Serialize, serde::Deserialize, Clone, Copy, Debug, PartialEq, Eq)]
pub struct Split {
    pub axis: Axis,
    pub ratio: Ratio,
}

#[derive(serde::Serialize, serde::Deserialize, Clone, Copy, Debug, PartialEq, Eq)]
pub struct Rect {
    pub x: u16,
    pub y: u16,
    pub geometry: Geometry,
}

#[derive(serde::Serialize, serde::Deserialize, Clone, Copy, Debug, PartialEq, Eq)]
pub struct Placement {
    pub pane: Id,
    pub rect: Rect,
    pub frame: Rect,
}

impl Rect {
    pub(crate) fn split(self, split: Split) -> Result<(Self, Self), Failure> {
        let length = match split.axis {
            Axis::Columns => self.geometry.columns(),
            Axis::Rows => self.geometry.rows(),
        };
        let available = length.checked_sub(1).ok_or(Failure::Geometry)?;
        let first = (u32::from(available) * u32::from(split.ratio.value()) / 1000) as u16;
        let second = available - first;
        let minimum = match split.axis {
            Axis::Columns => 2,
            Axis::Rows => 1,
        };
        if first < minimum || second < minimum {
            return Err(Failure::Geometry);
        }
        Ok(self.regions(split.axis, first, second))
    }

    fn regions(self, axis: Axis, first: u16, second: u16) -> (Self, Self) {
        let (mut left, mut right) = (self, self);
        let (columns, rows) = (self.geometry.columns(), self.geometry.rows());
        match axis {
            Axis::Columns => {
                left.geometry = Geometry::new(first, rows).unwrap();
                right.geometry = Geometry::new(second, rows).unwrap();
                right.x += first + 1;
            }
            Axis::Rows => {
                left.geometry = Geometry::new(columns, first).unwrap();
                right.geometry = Geometry::new(columns, second).unwrap();
                right.y += first + 1;
            }
        }
        (left, right)
    }
}

fn ratio<'de, D: serde::Deserializer<'de>>(decoder: D) -> Result<u16, D::Error> {
    let value = <u16 as serde::Deserialize>::deserialize(decoder)?;
    Ratio::new(value)
        .map(Ratio::value)
        .ok_or_else(|| serde::de::Error::custom("ratio must be between 1 and 999"))
}
