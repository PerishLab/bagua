use super::{App, validate};
use protocol::{
    Command, Failure,
    wire::{History, Read},
};
use terminal::Key;
use workspace::Id;

#[derive(Default)]
pub(super) struct Past {
    pub view: Option<History>,
    wanted: Option<(Id, Read)>,
    request: Option<u64>,
    ticket: u64,
}

impl Past {
    pub(super) fn active(&self) -> bool {
        self.view.is_some() || self.wanted.is_some()
    }
}

impl App {
    pub(super) fn browsing(&mut self, input: protocol::Input) -> Result<Vec<Command>, Failure> {
        match input {
            protocol::Input::Key(Key::Escape | Key::End) => {
                self.live();
                Ok(Vec::new())
            }
            protocol::Input::Key(Key::Previous) => self.handle(crate::Gesture::History(-1)),
            protocol::Input::Key(Key::Next) => self.handle(crate::Gesture::History(1)),
            _ => Err(Failure::Unavailable),
        }
    }

    pub fn history(&self, pane: Id) -> Option<&History> {
        self.past
            .view
            .as_ref()
            .filter(|view| view.scope.pane == pane)
    }

    pub fn current(&self, pane: Id, read: &Read) -> bool {
        self.past
            .wanted
            .as_ref()
            .is_some_and(|wanted| wanted == &(pane, read.clone()))
    }

    pub fn bind(&mut self, request: u64, pane: Id, read: &Read) {
        if self.current(pane, read) {
            self.past.request = Some(request);
        }
    }

    pub(super) fn live(&mut self) {
        self.cancel();
        self.past.view = None;
        self.past.wanted = None;
        self.past.request = None;
        self.selection = None;
        self.status("Live");
    }

    pub(crate) fn scroll(&mut self, pane: Id, rows: i32) -> Result<Vec<Command>, Failure> {
        let mut read = Read::default();
        if let Some(view) = self.history(pane) {
            let top = view
                .top
                .saturating_add_signed(i64::from(rows))
                .max(view.first);
            if top >= view.next {
                self.live();
                return Ok(Vec::new());
            }
            read.scope = Some(view.scope);
            read.top = Some(top);
            read.column = view.column;
        } else if rows >= 0 {
            return Ok(Vec::new());
        } else {
            read.back = rows.unsigned_abs().min(u32::from(terminal::ROWS)) as u16;
        }
        self.read(pane, read)
    }

    fn read(&mut self, pane: Id, mut read: Read) -> Result<Vec<Command>, Failure> {
        self.past.ticket = self.past.ticket.checked_add(1).ok_or(Failure::Identity)?;
        read.ticket = self.past.ticket;
        self.past.wanted = Some((pane, read.clone()));
        self.past.request = None;
        self.selection = None;
        Ok(vec![Command::History { pane, read }])
    }

    pub(super) fn horizontal(&mut self, key: Key) -> Result<Vec<Command>, Failure> {
        let pane = self.focus().ok_or(Failure::Target)?;
        let view = self.history(pane).ok_or(Failure::Target)?;
        let column = match key {
            Key::Left => view.column.saturating_sub(4),
            Key::Right => view
                .column
                .saturating_add(4)
                .min(terminal::COLUMNS - view.screen.extent.width()),
            _ => return Ok(Vec::new()),
        };
        self.read(
            pane,
            Read {
                scope: Some(view.scope),
                top: Some(view.top),
                column,
                ticket: 0,
                back: 0,
            },
        )
    }

    pub(super) fn past(&mut self, request: u64, view: History) -> Result<(), Failure> {
        let pane = view.scope.pane;
        let Some((wanted, read)) = &self.past.wanted else {
            return Ok(());
        };
        if self.past.request != Some(request) || *wanted != pane || read.ticket != view.ticket {
            return Ok(());
        }
        validate::screen(&view.screen)?;
        if view.first > view.top
            || view.top >= view.next
            || view.next - view.first > terminal::HISTORY as u64
        {
            return Err(Failure::Baseline);
        }
        if read.scope.is_some_and(|scope| scope != view.scope)
            || read.top.is_some_and(|top| top != view.top)
            || read.column != view.column
        {
            return Err(Failure::Baseline);
        }
        if view.screen.cursor.visible
            || view.screen.modes.alternate
            || view.bytes > terminal::STORAGE
        {
            return Err(Failure::Payload);
        }
        let Some(live) = self.screens.get(&pane).and_then(|replica| replica.state()) else {
            return Ok(());
        };
        if self.focus() != Some(pane) || live.extent != view.screen.extent {
            return Ok(());
        }
        self.status(format!(
            "History {}..{} / {} retained; column {}; PgUp/PgDn, End live",
            view.top,
            view.top + u64::from(view.screen.extent.height()),
            view.next,
            view.column
        ));
        self.past.view = Some(view);
        self.past.wanted = None;
        self.past.request = None;
        Ok(())
    }

    pub(super) fn finished(&mut self, request: u64) {
        if self.past.request == Some(request) {
            self.past.wanted = None;
            self.past.request = None;
        }
    }
}
