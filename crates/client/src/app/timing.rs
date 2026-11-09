use super::App;
use std::time::{Instant, SystemTime, UNIX_EPOCH};

impl App {
    pub fn decorate(&mut self, decor: crate::Decor) {
        self.cancel();
        self.decor = decor;
        self.next = None;
        self.dirty = true;
    }

    pub fn prefix(&mut self, value: Option<&str>) {
        let value = value.map(|value| value.chars().filter(|c| !c.is_control()).take(64).collect());
        if self.leader != value {
            self.leader = value;
            self.dirty = true;
        }
    }

    pub fn tick(&mut self, now: Instant, wall: SystemTime) {
        self.now = now;
        if self.expires.is_some_and(|expires| now >= expires) {
            self.expires = None;
            self.notice.clear();
            self.urgent = false;
            self.dirty = true;
        }
        if self.next.is_some_and(|next| now < next) {
            return;
        }
        self.next = now.checked_add(self.decor.status.refresh);
        self.clock = clock(wall, self.decor.status.offset);
        self.dirty = true;
    }
}

fn clock(wall: SystemTime, offset: u64) -> String {
    let Ok(time) = wall.duration_since(UNIX_EPOCH) else {
        return "--:--".into();
    };
    let minutes = (time.as_secs() / 60 + offset) % 1440;
    format!("{:02}:{:02}", minutes / 60, minutes % 60)
}
