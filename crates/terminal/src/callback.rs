use crate::events::Events;

impl vt100::Callbacks for Events {
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
