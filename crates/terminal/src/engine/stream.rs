use crate::{callback::Reset, events::Events};

#[derive(Default)]
pub(crate) struct Stream {
    parser: vte::Parser,
}

impl Stream {
    pub fn process(&mut self, target: &mut vt100::Parser<Events>, bytes: &[u8]) -> u64 {
        let mut reset = Reset::default();
        let mut epochs = 0;
        for byte in bytes {
            let piece = std::slice::from_ref(byte);
            self.parser.advance(&mut reset, piece);
            target.process(piece);
            if reset.clear {
                if !target.screen().alternate_screen() {
                    target.callbacks_mut().history.clear();
                    epochs += 1;
                }
                if reset.reset {
                    target.callbacks_mut().keyboard = Default::default();
                }
                reset = Reset::default();
            }
        }
        epochs
    }
}
