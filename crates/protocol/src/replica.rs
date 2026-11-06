use crate::{Cursor, Failure, Stamped, Update};
use terminal::Revision;
use workspace::Target;

pub struct Frame<T> {
    pub update: Update,
    pub state: T,
}

impl<T: Stamped> Frame<T> {
    pub fn snapshot(target: Target, state: T) -> Self {
        Self {
            update: Update {
                target,
                base: None,
                revision: state.revision(),
            },
            state,
        }
    }
}

pub enum Packet<S, D> {
    Snapshot(Frame<S>),
    Delta { update: Update, change: D },
}

pub struct Replica<S> {
    cursor: Cursor,
    state: Option<S>,
}

impl<S> std::fmt::Debug for Replica<S> {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("Replica")
            .field("cursor", &self.cursor)
            .field("installed", &self.state.is_some())
            .finish()
    }
}

impl<S: Stamped> Replica<S> {
    pub fn target(&self) -> Target {
        self.cursor.target()
    }
    pub fn new(target: Target) -> Self {
        Self {
            cursor: Cursor::new(target),
            state: None,
        }
    }
    pub fn state(&self) -> Option<&S> {
        self.state.as_ref()
    }
    pub fn revision(&self) -> Option<Revision> {
        self.cursor.revision()
    }

    pub fn apply<D>(
        &mut self,
        packet: Packet<S, D>,
        merge: impl FnOnce(&S, D) -> Result<S, Failure>,
    ) -> Result<(), Failure> {
        let mut cursor = self.cursor.clone();
        let (update, state) = match packet {
            Packet::Snapshot(frame) => {
                if frame.update.base.is_some() {
                    return Err(Failure::Baseline);
                }
                cursor.accept(frame.update)?;
                (frame.update, frame.state)
            }
            Packet::Delta { update, change } => {
                if update.base.is_none() {
                    return Err(Failure::Baseline);
                }
                cursor.accept(update)?;
                (
                    update,
                    merge(self.state.as_ref().ok_or(Failure::Baseline)?, change)?,
                )
            }
        };
        if state.revision() != update.revision {
            return Err(Failure::Payload);
        }
        self.state = Some(state);
        self.cursor = cursor;
        Ok(())
    }
}

pub struct Latest<T> {
    target: Target,
    revision: Option<Revision>,
    pending: Option<Frame<T>>,
}

impl<T: Stamped> Latest<T> {
    pub fn new(target: Target) -> Self {
        Self {
            target,
            revision: None,
            pending: None,
        }
    }

    pub fn offer(&mut self, state: T) -> Result<(), Failure> {
        let revision = state.revision();
        if self.revision.is_some_and(|current| revision <= current) {
            return Err(Failure::Stale);
        }
        self.pending = Some(Frame {
            update: Update {
                target: self.target,
                base: None,
                revision,
            },
            state,
        });
        self.revision = Some(revision);
        Ok(())
    }

    pub fn take(&mut self) -> Option<Frame<T>> {
        self.pending.take()
    }
}
