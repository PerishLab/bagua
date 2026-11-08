use protocol::Failure;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Mouse {
    pub enabled: bool,
    pub resize: bool,
    pub release: bool,
}

impl Default for Mouse {
    fn default() -> Self {
        Self {
            enabled: true,
            resize: true,
            release: true,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Clipboard {
    pub(crate) outer: bool,
    pub(crate) limit: usize,
}

impl Clipboard {
    pub fn new(target: &str, limit: usize) -> Result<Self, Failure> {
        if !(1..=1_048_576).contains(&limit) {
            return Err(Failure::Capacity);
        }
        let outer = match target {
            "local" => false,
            "osc52" => true,
            _ => return Err(Failure::Payload),
        };
        Ok(Self { outer, limit })
    }
}

impl Default for Clipboard {
    fn default() -> Self {
        Self {
            outer: false,
            limit: 262_144,
        }
    }
}
