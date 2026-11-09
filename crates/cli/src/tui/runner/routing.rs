use super::*;

pub(super) struct Deferred {
    pub gesture: Gesture,
    after: Option<u64>,
    token: Option<u64>,
}

#[derive(Default)]
pub(super) struct Routing {
    serial: u64,
    tail: Option<u64>,
    active: Option<u64>,
}

impl Routing {
    pub fn bind(&mut self, gesture: Gesture) -> Result<Deferred, Failure> {
        let after = self.tail;
        let token = if intent::changing(&gesture) {
            self.serial = self.serial.checked_add(1).ok_or(Failure::Identity)?;
            Some(self.serial)
        } else {
            None
        };
        self.tail = token.or(after);
        Ok(Deferred {
            gesture,
            after,
            token,
        })
    }

    pub fn command(&mut self) -> Result<(), Failure> {
        if self.active.is_none() {
            self.serial = self.serial.checked_add(1).ok_or(Failure::Identity)?;
            self.active = Some(self.serial);
            self.tail = self.active;
        }
        Ok(())
    }

    pub fn abandon(&mut self) {
        self.tail = None;
        self.active = None;
    }

    pub fn adjacent(&self, deferred: &Deferred) -> bool {
        deferred.after == self.tail
    }

    pub fn begin(&mut self, deferred: &Deferred) {
        self.active = deferred.token;
    }

    pub fn settle(&mut self, deferred: &mut VecDeque<Deferred>) {
        let Some(token) = self.active.take() else {
            return;
        };
        for item in deferred {
            if item.after == Some(token) {
                item.after = None;
            }
        }
        if self.tail == Some(token) {
            self.tail = None;
        }
    }

    pub fn reject(&mut self, deferred: &mut VecDeque<Deferred>) {
        let Some(token) = self.active.take() else {
            return;
        };
        let mut rejected = std::collections::BTreeSet::from([token]);
        deferred.retain(|item| {
            if item.after.is_some_and(|after| rejected.contains(&after)) {
                rejected.extend(item.token);
                false
            } else {
                true
            }
        });
        if self.tail.is_some_and(|tail| rejected.contains(&tail)) {
            self.tail = None;
        }
    }
}
