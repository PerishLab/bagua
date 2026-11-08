use crate::{Line, events::Events};

impl Events {
    pub fn capture(&mut self, contents: vt100::capture::RowContents<'_>) {
        let mut state = vt100::capture::BasicFormattedCaptureState::default();
        let mut formatted = String::new();
        contents
            .write_formatted_basic(&mut formatted, &mut state)
            .unwrap();
        let mut probe = String::new();
        contents
            .write_formatted_basic(&mut probe, &mut state)
            .unwrap();
        self.history.push(Line {
            id: 0,
            columns: self.columns,
            wrapped: !probe.starts_with('\n'),
            formatted,
        });
    }
}
