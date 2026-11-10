use locus::{Hook, Observation, Status};
use std::sync::Mutex;

#[derive(Clone, Copy)]
pub(super) enum Code {
    Config,
    State,
    Parent,
    Endpoint,
    Setup,
    Engine,
    Delivery,
}

const CODES: [&str; 7] = [
    "config", "state", "parent", "endpoint", "setup", "engine", "delivery",
];

pub(crate) struct Notes(Mutex<State>);

struct State {
    safe: bool,
    counts: [u32; 7],
}

impl Default for Notes {
    fn default() -> Self {
        Self(Mutex::new(State {
            safe: true,
            counts: [0; 7],
        }))
    }
}

impl Notes {
    pub(super) fn mark(&self, code: Code) {
        let mut state = self.0.lock().unwrap_or_else(|error| error.into_inner());
        let count = &mut state.counts[code as usize];
        *count = count.saturating_add(1);
    }

    pub(crate) fn safe(&self, safe: bool) {
        self.0
            .lock()
            .unwrap_or_else(|error| error.into_inner())
            .safe = safe;
    }

    pub(super) fn drain(&self) -> [u32; 7] {
        let mut state = self.0.lock().unwrap_or_else(|error| error.into_inner());
        if !state.safe {
            return [0; 7];
        }
        std::mem::take(&mut state.counts)
    }

    pub(super) fn flush(&self, json: bool) {
        for (code, count) in CODES.into_iter().zip(self.drain()) {
            if count == 0 {
                continue;
            }
            print(code, count, json);
        }
    }
}

impl Hook for Notes {
    fn observe(&self, observation: &Observation) {
        match observation {
            Observation::Diagnostic(_) => self.mark(Code::Engine),
            Observation::Report(outcome)
                if matches!(outcome.status(), Status::Failed | Status::Gated) =>
            {
                self.mark(Code::Delivery)
            }
            _ => {}
        }
    }
}

fn print(code: &str, count: u32, json: bool) {
    if json {
        eprintln!(
            "{}",
            serde_json::json!({"diagnostic": {"source": "bagua.locus", "code": code, "count": count}})
        );
    } else {
        eprintln!("bagua locus: {code} ({count}); observation may be incomplete");
    }
}

#[cfg(test)]
mod tests {
    use super::{Code, Notes};

    #[test]
    fn deferred() {
        let notes = Notes::default();
        notes.safe(false);
        for _ in 0..1000 {
            notes.mark(Code::Delivery);
        }
        assert_eq!(notes.drain(), [0; 7]);
        notes.safe(true);
        assert_eq!(notes.drain(), [0, 0, 0, 0, 0, 0, 1000]);
        assert_eq!(notes.drain(), [0; 7]);
    }
}
