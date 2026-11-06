use std::{num::NonZeroU16, time::Duration};

pub const CHUNK: usize = 4096;
pub const GRACE: Duration = Duration::from_millis(250);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Size {
    columns: NonZeroU16,
    rows: NonZeroU16,
}

impl Size {
    pub fn new(columns: u16, rows: u16) -> Option<Self> {
        Some(Self {
            columns: NonZeroU16::new(columns)?,
            rows: NonZeroU16::new(rows)?,
        })
    }

    pub fn columns(self) -> u16 {
        self.columns.get()
    }
    pub fn rows(self) -> u16 {
        self.rows.get()
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Exit {
    pub code: Option<i32>,
    pub signal: Option<i32>,
}

impl Exit {
    pub fn success(self) -> bool {
        self.code == Some(0)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Status {
    pub exit: Option<Exit>,
    pub io: bool,
    pub closing: bool,
}

impl Status {
    pub fn closed(self) -> bool {
        !self.io && self.exit.is_some()
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Transfer {
    Bytes(usize),
    Pending,
    Eof,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Interest {
    Read,
    Write,
    Both,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Ready {
    pub read: bool,
    pub write: bool,
    pub hangup: bool,
}
