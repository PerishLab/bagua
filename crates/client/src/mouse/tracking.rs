use crate::App;
use terminal::Tracking;

impl App {
    pub(crate) fn tracking(&self) -> Tracking {
        if !self.mouse.enabled {
            return Tracking::Off;
        }
        let forwarding = matches!(
            (self.controller(), self.recovering, self.done),
            (true, false, false)
        );
        if !forwarding || self.pointing() {
            return Tracking::Drag;
        }
        let motion = self
            .placements()
            .unwrap_or_default()
            .into_iter()
            .any(|place| {
                let rect = place.rect;
                let left = rect.x.max(self.offset.x);
                let top = rect.y.max(self.offset.y);
                let right = rect
                    .x
                    .saturating_add(rect.geometry.columns())
                    .min(self.offset.x.saturating_add(self.viewport.width()));
                let bottom = rect
                    .y
                    .saturating_add(rect.geometry.rows())
                    .min(self.offset.y.saturating_add(self.viewport.height()));
                let visible = left < right && top < bottom;
                let frozen = self.history(place.pane).is_some()
                    || self
                        .selection
                        .as_ref()
                        .is_some_and(|s| s.pane == place.pane);
                match (self.running(place.pane), frozen, visible) {
                    (true, false, true) => self
                        .screen(place.pane)
                        .is_some_and(|screen| screen.modes.tracking == Tracking::Motion),
                    _ => false,
                }
            });
        if motion {
            Tracking::Motion
        } else {
            Tracking::Drag
        }
    }
}
