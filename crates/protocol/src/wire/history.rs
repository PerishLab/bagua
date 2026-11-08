use super::Screen;
use serde::{Deserialize, Serialize};
use workspace::Id;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Scope {
    pub host: [u8; 16],
    pub pane: Id,
    pub epoch: u64,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Read {
    pub ticket: u64,
    pub back: u16,
    pub scope: Option<Scope>,
    pub top: Option<u64>,
    pub column: u16,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct History {
    pub ticket: u64,
    pub scope: Scope,
    pub first: u64,
    pub next: u64,
    pub top: u64,
    pub column: u16,
    pub bytes: usize,
    pub evicted: u64,
    pub screen: Screen,
}
