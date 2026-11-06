use crate::Failure;
use std::collections::BTreeSet;
use workspace::Target;

pub const SUBSCRIPTIONS: usize = 64;

#[derive(Default)]
pub struct Subscriptions {
    targets: BTreeSet<Target>,
}

impl Subscriptions {
    pub fn targets(&self) -> impl Iterator<Item = Target> + '_ {
        self.targets.iter().copied()
    }
    pub fn contains(&self, target: Target) -> bool {
        self.targets.contains(&target)
    }
    pub fn count(&self) -> usize {
        self.targets.len()
    }
    pub fn empty(&self) -> bool {
        self.targets.is_empty()
    }

    pub fn subscribe(&mut self, target: Target) -> Result<bool, Failure> {
        if self.targets.contains(&target) {
            return Ok(false);
        }
        if self.targets.len() >= SUBSCRIPTIONS {
            return Err(Failure::Capacity);
        }
        Ok(self.targets.insert(target))
    }

    pub fn unsubscribe(&mut self, target: Target) -> bool {
        self.targets.remove(&target)
    }

    pub fn clear(&mut self) {
        self.targets.clear();
    }
}
