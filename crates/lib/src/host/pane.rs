use protocol::wire::Fault;
use std::collections::VecDeque;
use terminal::{Engine, Extent};
use workspace::Id;

pub(super) struct Write {
    pub actor: Id,
    pub request: u64,
    pub bytes: Vec<u8>,
    pub offset: usize,
}

pub(super) struct Pane {
    pub engine: Engine,
    pub process: Option<pty::Process>,
    pub input: VecDeque<Write>,
    pub replies: VecDeque<u8>,
    pub eof: bool,
    pub exit: Option<pty::Exit>,
    pub fault: Option<Fault>,
    pub cleanup: Option<Fault>,
    pub bytes: u64,
    pub unsupported: u64,
    pub bells: u64,
    pub retiring: bool,
    pub reported: bool,
}

impl Pane {
    pub fn new(extent: Extent) -> Result<Self, protocol::Failure> {
        Ok(Self {
            engine: Engine::new(extent, 0).map_err(|_| protocol::Failure::Payload)?,
            process: None,
            input: VecDeque::new(),
            replies: VecDeque::new(),
            eof: false,
            exit: None,
            fault: None,
            cleanup: None,
            bytes: 0,
            unsupported: 0,
            bells: 0,
            retiring: false,
            reported: false,
        })
    }

    pub fn pending(&self, actor: Id) -> bool {
        self.input.iter().any(|write| write.actor == actor)
    }

    pub fn queued(&self) -> usize {
        self.input.iter().map(|write| write.bytes.len()).sum()
    }
}
