use super::App;
use crate::Gesture;
use protocol::{Command, Failure, Input};
use workspace::{Geometry, Operation, Ratio, Sizing, Split, Target};

impl App {
    pub fn handle(&mut self, gesture: Gesture) -> Result<Vec<Command>, Failure> {
        let operation = match gesture {
            Gesture::Ignore | Gesture::Leader => return Ok(Vec::new()),
            Gesture::Detach => {
                self.done = true;
                return Ok(Vec::new());
            }
            Gesture::Help => {
                self.status("Prefix: d detach v/h split n window Tab pane w window a acquire t transfer f fixed u auto x close ! session [ select c copy p paste r reload PgUp/PgDn history End live; Shift-drag select");
                return Ok(Vec::new());
            }
            Gesture::Reload => return Err(Failure::Unavailable),
            Gesture::History(direction) => {
                let pane = self.focus().ok_or(Failure::Target)?;
                let rows = i32::from(self.screen(pane).ok_or(Failure::Baseline)?.extent.height());
                return self.scroll(pane, direction.saturating_mul(rows));
            }
            Gesture::Live => {
                self.live();
                return Ok(Vec::new());
            }
            Gesture::Select => {
                self.begin()?;
                return Ok(Vec::new());
            }
            Gesture::Copy => {
                self.copy()?;
                return Ok(Vec::new());
            }
            Gesture::Pan(key) => {
                if self
                    .focus()
                    .is_some_and(|pane| self.history(pane).is_some())
                {
                    return self.horizontal(key);
                }
                self.pan(key);
                return Ok(Vec::new());
            }
            Gesture::Paste => return self.input(Input::Paste(self.clipboard.clone())),
            Gesture::Input(input) => {
                if self.selection.is_some() {
                    self.selecting(input)?;
                    return Ok(Vec::new());
                }
                if self
                    .focus()
                    .is_some_and(|pane| self.history(pane).is_some())
                {
                    return self.browsing(input);
                }
                return self.input(input);
            }
            Gesture::Acquire => {
                return Ok(vec![Command::Workspace(Operation::Acquire {
                    session: self.session,
                    fixed: None,
                })]);
            }
            Gesture::Split(axis) => Operation::Split {
                pane: self.focus().ok_or(Failure::Target)?,
                split: Split {
                    axis,
                    ratio: Ratio::default(),
                },
            },
            Gesture::Window => Operation::Window(self.session),
            Gesture::Index(index) => {
                let session = self.session().ok_or(Failure::Target)?;
                let count = session.windows().count();
                let pane = index.checked_sub(1).and_then(|index| {
                    session
                        .windows()
                        .nth(usize::from(index))
                        .map(|window| window.focus())
                });
                let Some(pane) = pane else {
                    self.error(format!("No window {index}; available: 1–{count}"));
                    return Ok(Vec::new());
                };
                Operation::Focus(pane)
            }
            Gesture::Pane => Operation::Focus(self.cycle(false)?),
            Gesture::Next => Operation::Focus(self.cycle(true)?),
            Gesture::Transfer => {
                let next = self
                    .session()
                    .ok_or(Failure::Target)?
                    .clients()
                    .map(|(id, _)| id)
                    .find(|id| *id != self.actor)
                    .ok_or(Failure::Target)?;
                Operation::Transfer {
                    session: self.session,
                    next,
                }
            }
            Gesture::Fixed => Operation::Size {
                session: self.session,
                sizing: Sizing::Fixed(self.geometry()),
            },
            Gesture::Automatic => Operation::Size {
                session: self.session,
                sizing: Sizing::Automatic(self.geometry()),
            },
            Gesture::Close => Operation::Close(Target::Pane(self.focus().ok_or(Failure::Target)?)),
            Gesture::Destroy => Operation::Close(Target::Session(self.session)),
        };
        if matches!(operation, Operation::Focus(pane) if self.focus() == Some(pane)) {
            return Ok(Vec::new());
        }
        if !self.controller() {
            return Err(Failure::Workspace(workspace::Failure::Observer));
        }
        Ok(vec![Command::Workspace(operation)])
    }

    pub fn geometry(&self) -> Geometry {
        Geometry::new(
            self.viewport.width().clamp(2, 256),
            self.viewport.height().min(64),
        )
        .unwrap()
    }

    fn input(&self, input: Input) -> Result<Vec<Command>, Failure> {
        if !self.controller() {
            return Err(Failure::Workspace(workspace::Failure::Observer));
        }
        input.check()?;
        Ok(vec![Command::Send {
            pane: self.focus().ok_or(Failure::Target)?,
            input,
        }])
    }

    fn cycle(&self, windows: bool) -> Result<workspace::Id, Failure> {
        let session = self.session().ok_or(Failure::Target)?;
        let ids: Vec<_> = if windows {
            session.windows().map(|w| w.focus()).collect()
        } else {
            self.placements()?.iter().map(|p| p.pane).collect()
        };
        let focus = self.focus().ok_or(Failure::Target)?;
        let index = ids
            .iter()
            .position(|id| *id == focus)
            .ok_or(Failure::Target)?;
        Ok(ids[(index + 1) % ids.len()])
    }

    pub(super) fn clamp(&mut self) {
        if let Some(session) = self.session() {
            let size = session.geometry();
            let maximum = terminal::Point {
                x: size.columns().saturating_sub(self.viewport.width()),
                y: size.rows().saturating_sub(self.viewport.height()),
            };
            self.offset.x = self.offset.x.min(maximum.x);
            self.offset.y = self.offset.y.min(maximum.y);
        }
    }
    fn pan(&mut self, key: terminal::Key) {
        use terminal::Key;
        let Some(session) = self.session() else {
            return;
        };
        let size = session.geometry();
        let maximum = terminal::Point {
            x: size.columns().saturating_sub(self.viewport.width()),
            y: size.rows().saturating_sub(self.viewport.height()),
        };
        match key {
            Key::Left => self.offset.x = self.offset.x.saturating_sub(4),
            Key::Right => self.offset.x = self.offset.x.saturating_add(4).min(maximum.x),
            Key::Up => self.offset.y = self.offset.y.saturating_sub(1),
            Key::Down => self.offset.y = self.offset.y.saturating_add(1).min(maximum.y),
            _ => {}
        }
        self.dirty = true;
    }
}
