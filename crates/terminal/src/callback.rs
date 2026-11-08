use crate::events::Events;

#[derive(Default)]
pub(crate) struct Reset(pub bool);

impl vte::Perform for Reset {
    fn csi_dispatch(&mut self, params: &vte::Params, intermediates: &[u8], _: bool, action: char) {
        let first = params.iter().next().and_then(|group| group.first());
        self.0 = intermediates.is_empty() && action == 'J' && first == Some(&3);
    }

    fn esc_dispatch(&mut self, intermediates: &[u8], _: bool, byte: u8) {
        self.0 = intermediates.is_empty() && byte == b'c';
    }
}

impl vt100::Callbacks for Events {
    fn on_scroll(&mut self, contents: vt100::capture::RowContents<'_>, alternate_screen: bool) {
        if !alternate_screen {
            self.capture(contents);
        }
    }
    fn audible_bell(&mut self, _: &mut vt100::Screen) {
        self.bell();
    }

    fn visual_bell(&mut self, _: &mut vt100::Screen) {
        self.bell();
    }

    fn unhandled_char(&mut self, _: &mut vt100::Screen, _: char) {
        self.discard();
    }

    fn unhandled_control(&mut self, _: &mut vt100::Screen, _: u8) {
        self.discard();
    }

    fn unhandled_escape(&mut self, _: &mut vt100::Screen, _: Option<u8>, _: Option<u8>, _: u8) {
        self.discard();
    }

    fn unhandled_osc(&mut self, _: &mut vt100::Screen, _: &[&[u8]]) {
        self.discard();
    }

    fn unhandled_csi(
        &mut self,
        screen: &mut vt100::Screen,
        first: Option<u8>,
        second: Option<u8>,
        params: &[&[u16]],
        action: char,
    ) {
        self.respond(screen, (first, second), params, action);
    }
}
