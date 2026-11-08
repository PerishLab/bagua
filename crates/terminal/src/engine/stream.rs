use crate::{callback::Reset, events::Events};

#[derive(Default)]
pub(crate) struct Stream {
    parser: vte::Parser,
}

impl Stream {
    pub fn process(&mut self, target: &mut vt100::Parser<Events>, bytes: &[u8]) -> u64 {
        let mut reset = Reset::default();
        let mut epochs = 0;
        let mut start = 0;
        for (index, byte) in bytes.iter().enumerate() {
            self.parser.advance(&mut reset, std::slice::from_ref(byte));
            if reset.0 {
                target.process(&bytes[start..=index]);
                if !target.screen().alternate_screen() {
                    target.callbacks_mut().history.clear();
                    epochs += 1;
                }
                start = index + 1;
                reset.0 = false;
            }
        }
        target.process(&bytes[start..]);
        epochs
    }
}
