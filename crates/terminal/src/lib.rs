use std::num::NonZeroU16;

mod callback;
mod engine;
mod events;
mod history;
mod input;
mod mouse;
mod profile;
mod screen;

pub use engine::Engine;
pub use events::Report;
pub use history::{History, Line, PAGE, Range, STORAGE};
pub use input::{Input, Key};
pub use mouse::{Action, Button, Direction, Modifiers, Mouse, Point};
pub use profile::{CELLS, CHUNK, COLUMNS, EXCLUDED, HISTORY, PROFILE, ROWS, SOURCE};
pub use screen::Snapshot;
pub use screen::{Cell, Color, Cursor, Encoding, Modes, Style, Tracking};

#[derive(serde::Serialize, serde::Deserialize, Clone, Copy, Debug, PartialEq, Eq)]
pub enum Failure {
    Capacity,
    Input,
    Unsupported,
    Inactive,
    Position,
    Revision,
}

#[derive(serde::Serialize, serde::Deserialize, Clone, Copy, Debug, PartialEq, Eq)]
pub struct Extent {
    width: NonZeroU16,
    height: NonZeroU16,
}

impl Extent {
    pub fn new(width: u16, height: u16) -> Option<Self> {
        Some(Self {
            width: NonZeroU16::new(width)?,
            height: NonZeroU16::new(height)?,
        })
    }

    pub fn width(self) -> u16 {
        self.width.get()
    }

    pub fn height(self) -> u16 {
        self.height.get()
    }

    pub fn remaining(self, x: u16, y: u16) -> Option<Self> {
        Self::new(self.width().checked_sub(x)?, self.height().checked_sub(y)?)
    }

    pub fn fit(self, available: Self) -> Self {
        Self {
            width: self.width.min(available.width),
            height: self.height.min(available.height),
        }
    }
}

#[derive(
    serde::Serialize,
    serde::Deserialize,
    Clone,
    Copy,
    Debug,
    Default,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
)]
pub struct Revision(u64);

impl Revision {
    pub const fn new(value: u64) -> Self {
        Self(value)
    }

    pub fn next(self) -> Option<Self> {
        Some(Self(self.0.checked_add(1)?))
    }

    pub const fn value(self) -> u64 {
        self.0
    }
}
