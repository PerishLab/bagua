mod codec;
mod context;
mod history;
mod image;
mod screen;

use crate::{Failure, Reply};
pub use codec::{Decoder, encode};
pub use context::Context;
pub use history::{History, Read, Scope};
pub use image::Image;
pub use screen::Screen;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use workspace::Id;

pub const REQUEST: usize = 131_072;
pub const FRAME: usize = 8_388_608;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Program {
    pub executable: String,
    pub args: Vec<String>,
    pub cwd: String,
    pub environment: BTreeMap<String, String>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Fault {
    pub code: String,
    pub message: String,
}

impl From<Failure> for Fault {
    fn from(error: Failure) -> Self {
        Self {
            code: error.code().into(),
            message: error.message().into(),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Message {
    Context {
        request: u64,
        pane: Id,
        state: Context,
    },
    History {
        request: u64,
        state: History,
    },
    Hello {
        version: u16,
        #[serde(default)]
        identity: Option<Image>,
        client: Id,
        canvas: workspace::Geometry,
    },
    Catalog(workspace::Snapshot),
    Reply(Reply),
    Finished {
        request: u64,
        result: Result<(), Fault>,
    },
    Process {
        pane: Id,
        io: bool,
        closing: bool,
        exited: bool,
        code: Option<i32>,
        signal: Option<i32>,
        eof: bool,
        fault: Option<Fault>,
    },
    Screen {
        pane: Id,
        state: Screen,
    },
    Event(crate::Event),
}
