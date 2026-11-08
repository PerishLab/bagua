use super::{App, validate};
use protocol::{Failure, Frame, Packet, Replica, wire::Screen};
use workspace::{Id, Target};

impl App {
    pub(super) fn install(&mut self, pane: Id, screen: Screen) -> Result<(), Failure> {
        validate::screen(&screen)?;
        let Some(session) = self.session() else {
            return Ok(());
        };
        let Some(window) = session
            .windows()
            .find(|w| w.panes().any(|p| p.id() == pane))
        else {
            return Ok(());
        };
        let expected = window
            .placements(session.canvas())?
            .into_iter()
            .find(|p| p.pane == pane)
            .map(|p| p.rect.geometry);
        let Some(size) = expected else { return Ok(()) };
        if (size.columns(), size.rows()) != (screen.extent.width(), screen.extent.height()) {
            return Ok(());
        }
        let replica = self
            .screens
            .entry(pane)
            .or_insert_with(|| Replica::new(Target::Pane(pane)));
        if replica
            .revision()
            .is_some_and(|revision| revision >= screen.revision)
        {
            return Ok(());
        }
        replica.apply(
            Packet::<_, ()>::Snapshot(Frame::snapshot(Target::Pane(pane), screen)),
            |_, ()| Err(Failure::Baseline),
        )?;
        self.dirty = true;
        Ok(())
    }
}
