use protocol::{
    Failure, Subscriptions,
    wire::{self, Message},
};
use std::collections::{BTreeMap, VecDeque};
use terminal::Revision;
use workspace::Id;

const BYTES: usize = 262_144;
const MESSAGES: usize = 64;

pub(super) struct Client {
    pub last: u64,
    pub watches: Subscriptions,
    pub wanted: BTreeMap<Id, u64>,
    pub seen: BTreeMap<Id, Revision>,
    pub cursor: Option<Id>,
    pub catalog: u64,
    pub catalogue: bool,
    pub history: Option<(u64, Id, wire::Read)>,
    queue: VecDeque<Vec<u8>>,
    bytes: usize,
    writing: Option<(Vec<u8>, usize)>,
    quota: u8,
}

impl Client {
    pub fn new(id: Id) -> Result<Self, Failure> {
        let mut client = Self {
            last: 0,
            watches: Subscriptions::default(),
            wanted: BTreeMap::new(),
            seen: BTreeMap::new(),
            cursor: None,
            catalog: 0,
            catalogue: false,
            history: None,
            queue: VecDeque::new(),
            bytes: 0,
            writing: None,
            quota: 0,
        };
        client.push(Message::Hello {
            version: protocol::VERSION,
            client: id,
            canvas: workspace::Geometry::new(super::COLUMNS, super::ROWS).unwrap(),
        })?;
        Ok(client)
    }

    pub fn room(&self) -> Result<(), Failure> {
        if self.bytes > BYTES - 16_384 || self.queue.len() > MESSAGES - 8 {
            return Err(Failure::Capacity);
        }
        Ok(())
    }

    pub fn push(&mut self, message: Message) -> Result<(), Failure> {
        let bytes = wire::encode(&message, BYTES)?;
        if self.bytes + bytes.len() > BYTES || self.queue.len() >= MESSAGES {
            return Err(Failure::Capacity);
        }
        self.bytes += bytes.len();
        self.queue.push_back(bytes);
        Ok(())
    }

    pub fn present(&mut self, message: Message) -> Result<bool, Failure> {
        if !self.available() {
            return Ok(false);
        }
        let bytes = wire::encode(&message, wire::FRAME)?;
        self.writing = Some((bytes, 0));
        self.quota = 0;
        Ok(true)
    }

    pub fn idle(&self) -> bool {
        self.queue.is_empty() && self.writing.is_none()
    }

    pub fn available(&self) -> bool {
        self.writing.is_none() && (self.queue.is_empty() || self.quota >= 4)
    }

    pub fn resume(&mut self) {
        self.quota = 0;
    }

    pub fn output(&mut self) -> Option<&[u8]> {
        if self.writing.is_none() {
            let pending =
                !self.watches.empty() || !self.wanted.is_empty() || self.history.is_some();
            if self.quota >= 4 && pending {
                return None;
            }
            let bytes = self.queue.pop_front()?;
            self.bytes -= bytes.len();
            self.writing = Some((bytes, 0));
            self.quota = self.quota.saturating_add(1);
        }
        let (bytes, offset) = self.writing.as_ref()?;
        Some(&bytes[*offset..])
    }

    pub fn consume(&mut self, count: usize) {
        if let Some((bytes, offset)) = &mut self.writing {
            *offset += count.min(bytes.len() - *offset);
            if *offset == bytes.len() {
                self.writing = None;
            }
        }
    }
}
