use crate::App;
use base64::{Engine, engine::general_purpose::STANDARD};
use protocol::Failure;
use std::collections::VecDeque;

#[derive(Clone)]
pub struct Copy(pub(crate) Vec<u8>);

#[derive(Default)]
pub(crate) struct Copies {
    queue: VecDeque<Copy>,
    bytes: usize,
}

impl Copies {
    pub fn push(&mut self, text: &str, outer: bool) -> Result<(), Failure> {
        if !outer {
            return Ok(());
        }
        let bytes = format!("\x1b]52;c;{}\x1b\\", STANDARD.encode(text)).into_bytes();
        if self.queue.len() >= 4 || self.bytes + bytes.len() > 2_097_152 {
            return Err(Failure::Capacity);
        }
        self.bytes += bytes.len();
        self.queue.push_back(Copy(bytes));
        Ok(())
    }
}

impl App {
    pub fn copying(&self) -> Option<&Copy> {
        self.copies.queue.front()
    }

    pub fn copied(&mut self) {
        if let Some(copy) = self.copies.queue.pop_front() {
            self.copies.bytes -= copy.0.len();
        }
    }
}
