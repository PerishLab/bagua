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
        self.date = date(wall, self.decor.status.offset);
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

fn date(wall: SystemTime, offset: u64) -> String {
    let Some(seconds) = wall
        .duration_since(UNIX_EPOCH)
        .ok()
        .and_then(|t| t.as_secs().checked_add(offset * 60))
    else {
        return "-- --".into();
    };
    let z = (seconds / 86400) as i64 + 719468;
    let era = z / 146097;
    let day = z - era * 146097;
    let year = (day - day / 1460 + day / 36524 - day / 146096) / 365;
    let ordinal = day - (365 * year + year / 4 - year / 100);
    let part = (5 * ordinal + 2) / 153;
    let date = ordinal - (153 * part + 2) / 5 + 1;
    let month = part + if part < 10 { 3 } else { -9 };
    format!("{month:02}-{date:02}")
}
