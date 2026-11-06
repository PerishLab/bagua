use std::num::NonZeroU64;

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct Id(NonZeroU64);

impl Id {
    pub fn new(value: u64) -> Option<Self> {
        Some(Self(NonZeroU64::new(value)?))
    }

    pub fn value(self) -> u64 {
        self.0.get()
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Target {
    Session(Id),
    Window(Id),
    Pane(Id),
}
