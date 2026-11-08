use protocol::{Failure, wire::Screen};
use std::collections::BTreeSet;
use workspace::Snapshot;

pub(super) fn screen(screen: &Screen) -> Result<(), Failure> {
    let width = usize::from(screen.extent.width());
    let height = usize::from(screen.extent.height());
    if !(2..=256).contains(&width) || height > 64 {
        return Err(Failure::Capacity);
    }
    if screen.cells.len() != width * height || screen.wrapped.len() != height {
        return Err(Failure::Payload);
    }
    if screen.cursor.position.x >= screen.extent.width()
        || screen.cursor.position.y >= screen.extent.height()
    {
        return Err(Failure::Payload);
    }
    for (index, cell) in screen.cells.iter().enumerate() {
        if cell.width > 2 || cell.text.len() > 22 || cell.text.chars().any(char::is_control) {
            return Err(Failure::Payload);
        }
        match cell.width {
            0 if index % width == 0
                || screen.cells[index - 1].width != 2
                || !cell.text.is_empty() =>
            {
                return Err(Failure::Payload);
            }
            2 if index % width + 1 == width || screen.cells[index + 1].width != 0 => {
                return Err(Failure::Payload);
            }
            _ => {}
        }
    }
    Ok(())
}

pub(super) fn catalog(snapshot: &Snapshot) -> Result<(), Failure> {
    if snapshot.sessions().count() > 32 {
        return Err(Failure::Capacity);
    }
    let mut ids = BTreeSet::new();
    let mut names = BTreeSet::new();
    let mut count = 0;
    let mut cells = 0;
    for session in snapshot.sessions() {
        if !workspace::valid(session.name()) || !names.insert(session.name()) {
            return Err(Failure::Payload);
        }
        let mut windows = BTreeSet::new();
        let size = session.geometry();
        if !size.valid() || size.columns() > 256 || size.rows() > 64 {
            return Err(Failure::Capacity);
        }
        if !ids.insert(session.id()) {
            return Err(Failure::Identity);
        }
        for window in session.windows() {
            if !workspace::valid(window.name()) || !windows.insert(window.name()) {
                return Err(Failure::Payload);
            }
            if !ids.insert(window.id()) {
                return Err(Failure::Identity);
            }
            let panes: BTreeSet<_> = window.panes().map(|p| p.id()).collect();
            let placements = window.placements(size)?;
            cells += placements
                .iter()
                .map(|p| {
                    usize::from(p.rect.geometry.columns()) * usize::from(p.rect.geometry.rows())
                })
                .sum::<usize>();
            let places: BTreeSet<_> = placements.into_iter().map(|p| p.pane).collect();
            if panes != places || !panes.contains(&window.focus()) {
                return Err(Failure::Payload);
            }
            count += panes.len();
            if panes.into_iter().any(|id| !ids.insert(id)) {
                return Err(Failure::Identity);
            }
        }
    }
    if count > 32 || cells > 262_144 {
        return Err(Failure::Capacity);
    }
    Ok(())
}
