use protocol::Failure;
#[derive(Default)]
pub(super) struct Paste {
    bytes: Vec<u8>,
    tail: Vec<u8>,
    rejected: bool,
}
impl Paste {
    pub fn feed(&mut self, byte: u8) -> Option<Result<String, Failure>> {
        const END: &[u8] = b"\x1b[201~";
        self.tail.push(byte);
        while !END.starts_with(&self.tail) {
            let byte = self.tail.remove(0);
            if self.bytes.len() == terminal::CHUNK {
                self.rejected = true;
                self.bytes.clear();
            }
            if !self.rejected {
                self.bytes.push(byte);
            }
        }
        if self.tail != END {
            return None;
        }
        if self.rejected {
            return Some(Err(Failure::Capacity));
        }
        Some(String::from_utf8(std::mem::take(&mut self.bytes)).map_err(|_| Failure::Payload))
    }
}
