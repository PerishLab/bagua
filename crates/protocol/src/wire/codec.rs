use crate::Failure;
use serde::{Serialize, de::DeserializeOwned};

pub fn encode(value: &impl Serialize, limit: usize) -> Result<Vec<u8>, Failure> {
    let payload = serde_json::to_vec(value).map_err(|_| Failure::Payload)?;
    if payload.is_empty() || payload.len() > limit.min(super::FRAME) {
        return Err(Failure::Payload);
    }
    let length = u32::try_from(payload.len()).map_err(|_| Failure::Payload)?;
    let mut bytes = Vec::with_capacity(payload.len() + 4);
    bytes.extend_from_slice(&length.to_be_bytes());
    bytes.extend_from_slice(&payload);
    Ok(bytes)
}

pub struct Decoder {
    bytes: Vec<u8>,
    limit: usize,
}

impl Decoder {
    pub fn new(limit: usize) -> Self {
        Self {
            bytes: Vec::new(),
            limit: limit.min(super::FRAME),
        }
    }

    pub fn push(&mut self, bytes: &[u8]) -> Result<(), Failure> {
        if self.bytes.len().saturating_add(bytes.len()) > self.limit + 4 {
            return Err(Failure::Payload);
        }
        self.bytes.extend_from_slice(bytes);
        self.length()?;
        Ok(())
    }

    pub fn room(&self) -> usize {
        self.limit + 4 - self.bytes.len()
    }

    pub fn empty(&self) -> bool {
        self.bytes.is_empty()
    }

    pub fn take<T: DeserializeOwned>(&mut self) -> Result<Option<T>, Failure> {
        let Some(length) = self.length()? else {
            return Ok(None);
        };
        if self.bytes.len() < length + 4 {
            return Ok(None);
        }
        let value =
            serde_json::from_slice(&self.bytes[4..length + 4]).map_err(|_| Failure::Payload)?;
        self.bytes.drain(..length + 4);
        Ok(Some(value))
    }

    fn length(&self) -> Result<Option<usize>, Failure> {
        let Some(header) = self.bytes.get(..4) else {
            return Ok(None);
        };
        let length = u32::from_be_bytes(header.try_into().unwrap()) as usize;
        if length == 0 || length > self.limit {
            return Err(Failure::Payload);
        }
        Ok(Some(length))
    }
}
