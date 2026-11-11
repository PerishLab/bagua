use std::num::NonZeroU16;

#[derive(serde::Serialize, serde::Deserialize, Clone, Copy, Debug, Default, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum Edge {
    #[default]
    Top,
    Bottom,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Canvas {
    pub geometry: Geometry,
    pub header: bool,
    pub edge: Edge,
}

impl From<Geometry> for Canvas {
    fn from(geometry: Geometry) -> Self {
        Self {
            geometry,
            header: false,
            edge: Edge::Top,
        }
    }
}

#[derive(serde::Serialize, serde::Deserialize, Clone, Copy, Debug, PartialEq, Eq)]
pub struct Geometry {
    columns: NonZeroU16,
    rows: NonZeroU16,
}

impl Geometry {
    pub fn new(columns: u16, rows: u16) -> Option<Self> {
        Some(Self {
            columns: NonZeroU16::new(columns)?,
            rows: NonZeroU16::new(rows)?,
        })
    }

    pub fn valid(self) -> bool {
        self.columns() >= 2
    }

    pub fn columns(self) -> u16 {
        self.columns.get()
    }

    pub fn rows(self) -> u16 {
        self.rows.get()
    }
}

#[derive(serde::Serialize, serde::Deserialize, Clone, Copy, Debug, PartialEq, Eq)]
pub enum Sizing {
    Automatic(Geometry),
    Fixed(Geometry),
}

impl Sizing {
    pub fn geometry(self) -> Geometry {
        match self {
            Self::Automatic(geometry) | Self::Fixed(geometry) => geometry,
        }
    }
}
