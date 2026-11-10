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
    pub fn new(extent: Extent, history: crate::config::History) -> Result<Self, protocol::Failure> {
        Ok(Self {
            engine: Engine::bounded(extent, history.lines, history.bytes)
                .map_err(|_| protocol::Failure::Payload)?,
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

    pub fn facts(&self) -> [bool; 5] {
        [
            self.eof,
            self.exit.is_some(),
            self.process.is_none(),
            self.fault.is_some(),
            self.cleanup.is_some(),
        ]
    }

    pub fn changes(&self, id: Id, before: [bool; 5]) -> [Option<super::Event>; 5] {
        let after = self.facts();
        std::array::from_fn(|index| {
            if before[index] || !after[index] {
                return None;
            }
            Some(match index {
                0 => super::Event::Eof(id),
                1 => super::Event::Exit(id, self.exit.unwrap()),
                2 => super::Event::Release(id),
                3 => super::Event::Fault(id),
                _ => super::Event::Cleanup(id),
            })
        })
    }

    pub fn finished(&self) -> bool {
        match self {
            Self {
                exit: Some(_),
                eof: true,
                process: None,
                retiring: false,
                fault: None,
                cleanup: None,
                ..
            } => self.input.is_empty() && self.replies.is_empty(),
            _ => false,
        }
    }

    pub fn pending(&self, actor: Id) -> bool {
        self.input.iter().any(|write| write.actor == actor)
    }

    pub fn queued(&self) -> usize {
        self.input.iter().map(|write| write.bytes.len()).sum()
    }
}
