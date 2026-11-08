mod stream;

use crate::{CHUNK, Extent, Failure, Input, Modes, Report, Revision, Snapshot, events, profile};

pub struct Engine {
    parser: vt100::Parser<events::Events>,
    stream: stream::Stream,
    extent: Extent,
    history: usize,
    revision: Revision,
    epoch: u64,
}

impl Engine {
    pub fn new(extent: Extent, history: usize) -> Result<Self, Failure> {
        Self::bounded(extent, history, crate::STORAGE)
    }

    pub fn bounded(extent: Extent, history: usize, bytes: usize) -> Result<Self, Failure> {
        profile::check(extent, history)?;
        Ok(Self {
            parser: vt100::Parser::new_with_callbacks(
                extent.height,
                extent.width,
                0,
                events::Events::new(crate::History::new(history, bytes)?, extent.width()),
            ),
            extent,
            stream: stream::Stream::default(),
            history,
            revision: Revision::default(),
            epoch: 0,
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
            self.epoch = self
                .epoch
                .checked_add(self.stream.process(&mut self.parser, bytes))
                .ok_or(Failure::Revision)?;
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
        let epoch = self.epoch.checked_add(1).ok_or(Failure::Revision)?;
        self.parser.set_size(extent.height, extent.width);
        self.parser.callbacks_mut().columns = extent.width();
        self.extent = extent;
        self.revision = revision;
        self.epoch = epoch;
        Ok(revision)
    }

    pub fn snapshot(&self) -> Snapshot {
        Snapshot::new(
            self.parser.screen().clone(),
            self.revision,
            self.history().clone(),
            self.modes().keyboard,
        )
    }

    pub fn history(&self) -> &crate::History {
        &self.parser.callbacks().history
    }

    pub fn epoch(&self) -> u64 {
        self.epoch
    }

    pub fn trim(&mut self, bytes: usize) {
        self.parser.callbacks_mut().history.trim(bytes);
    }

    pub fn budget(&mut self, bytes: usize) {
        self.parser.callbacks_mut().history.budget(bytes);
    }

    pub fn modes(&self) -> Modes {
        let mut modes = Modes::from(self.parser.screen());
        modes.keyboard = self.parser.callbacks().keyboard.flags(modes.alternate);
        modes
    }

    pub fn encode(&self, input: Input<'_>) -> Result<Vec<u8>, Failure> {
        input.encode(self.modes(), self.extent)
    }
}
