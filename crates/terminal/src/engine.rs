use crate::{CHUNK, Extent, Failure, Input, Modes, Report, Revision, Snapshot, events, profile};

pub struct Engine {
    parser: vt100::Parser<events::Events>,
    extent: Extent,
    history: usize,
    revision: Revision,
}

impl Engine {
    pub fn new(extent: Extent, history: usize) -> Result<Self, Failure> {
        profile::check(extent, history)?;
        Ok(Self {
            parser: vt100::Parser::new_with_callbacks(
                extent.height,
                extent.width,
                history,
                events::Events::default(),
            ),
            extent,
            history,
            revision: Revision::default(),
        })
    }

    pub fn revision(&self) -> Revision {
        self.revision
    }

    pub fn extent(&self) -> Extent {
        self.extent
    }

    pub fn feed(&mut self, bytes: &[u8]) -> Result<Report, Failure> {
        if bytes.len() > CHUNK {
            return Err(Failure::Capacity);
        }
        if !bytes.is_empty() {
            let revision = self.revision.next().ok_or(Failure::Revision)?;
            self.parser.process(bytes);
            self.revision = revision;
        }
        Ok(self.parser.callbacks_mut().take(self.revision))
    }

    pub fn resize(&mut self, extent: Extent) -> Result<Revision, Failure> {
        profile::check(extent, self.history)?;
        if extent == self.extent {
            return Ok(self.revision);
        }
        let revision = self.revision.next().ok_or(Failure::Revision)?;
        self.parser.set_size(extent.height, extent.width);
        self.extent = extent;
        self.revision = revision;
        Ok(revision)
    }

    pub fn snapshot(&self) -> Snapshot {
        Snapshot::new(self.parser.screen().clone(), self.revision)
    }

    pub fn modes(&self) -> Modes {
        Modes::from(self.parser.screen())
    }

    pub fn encode(&self, input: Input<'_>) -> Result<Vec<u8>, Failure> {
        input.encode(self.modes(), self.extent)
    }
}
