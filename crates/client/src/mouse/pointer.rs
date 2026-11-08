use crate::App;
use crossterm::event::{KeyModifiers, MouseButton, MouseEvent, MouseEventKind};
use protocol::{Command, Failure};
use terminal::Point;
use workspace::{Divider, Id, Operation, Placement, Ratio, Rect};

pub(crate) enum Pointer {
    Selection {
        pane: Id,
        rect: Rect,
        start: Point,
        dragged: bool,
    },
    Resize {
        window: Id,
        divider: Divider,
        ratio: Ratio,
    },
    Application {
        pane: Id,
        rect: Rect,
        button: MouseButton,
    },
}

impl App {
    pub fn pointing(&self) -> bool {
        matches!(
            self.pointer,
            Some(Pointer::Selection { .. } | Pointer::Resize { .. })
        )
    }

    pub(crate) fn pointer(&mut self) {
        let valid = match &self.pointer {
            Some(Pointer::Selection { pane, rect, .. }) => {
                (self.focus() == Some(*pane) || !self.controller()) && self.located(*pane, *rect)
            }
            Some(Pointer::Application { pane, rect, .. }) => {
                self.controller() && self.located(*pane, *rect)
            }
            Some(Pointer::Resize {
                window, divider, ..
            }) => self.dividing(*window, *divider),
            None => true,
        };
        if !valid {
            self.cancel();
        }
    }

    fn located(&self, pane: Id, rect: Rect) -> bool {
        self.placements().is_ok_and(|places| {
            places
                .iter()
                .any(|place| place.pane == pane && place.rect == rect)
        })
    }

    fn dividing(&self, window: Id, divider: Divider) -> bool {
        if !self.controller() {
            return false;
        }
        let Some(session) = self.session() else {
            return false;
        };
        if session.selected() != Some(window) {
            return false;
        }
        let Some(view) = session.windows().find(|view| view.id() == window) else {
            return false;
        };
        view.dividers(session.canvas())
            .is_ok_and(|dividers| dividers.contains(&divider))
    }

    pub fn preferences(&mut self, mouse: crate::Mouse, clipboard: crate::Clipboard) {
        self.cancel();
        self.mouse = mouse;
        self.copying = clipboard;
    }

    pub fn cancel(&mut self) {
        if matches!(self.pointer, Some(Pointer::Selection { dragged: true, .. })) {
            self.selection = None;
            self.dirty = true;
        }
        self.pointer = None;
    }

    pub(super) fn arm(&mut self, place: Placement, point: Point) {
        self.selection = None;
        self.pointer = Some(Pointer::Selection {
            pane: place.pane,
            rect: place.rect,
            start: Point {
                x: point.x - place.rect.x,
                y: point.y - place.rect.y,
            },
            dragged: false,
        });
    }

    pub(super) fn divider(&mut self, event: MouseEvent, point: Point) -> Result<bool, Failure> {
        if event.kind != MouseEventKind::Down(MouseButton::Left)
            || event.modifiers.contains(KeyModifiers::SHIFT)
            || !self.mouse.resize
        {
            return Ok(false);
        }
        let session = self.session().ok_or(Failure::Target)?;
        let window = session
            .windows()
            .find(|w| Some(w.id()) == session.selected())
            .ok_or(Failure::Target)?;
        let Some(divider) = window
            .dividers(session.canvas())?
            .into_iter()
            .find(|d| d.contains(point.x, point.y))
        else {
            return Ok(false);
        };
        if !self.controller() {
            return Err(Failure::Workspace(workspace::Failure::Observer));
        }
        self.pointer = Some(Pointer::Resize {
            window: window.id(),
            divider,
            ratio: divider.split.ratio,
        });
        self.status("Drag divider; release commits a validated layout, Escape cancels");
        Ok(true)
    }

    pub(super) fn captured(
        &mut self,
        event: MouseEvent,
        point: Point,
    ) -> Result<Option<Vec<Command>>, Failure> {
        if matches!(event.kind, MouseEventKind::Down(_))
            || (self.pointing() && super::wheel(event.kind).is_some())
        {
            self.cancel();
            return Ok(None);
        }
        let Some(pointer) = self.pointer.take() else {
            return Ok(None);
        };
        match pointer {
            pointer @ Pointer::Selection { .. } => self.dragged(pointer, event, point).map(Some),
            Pointer::Resize {
                window,
                divider,
                mut ratio,
            } => {
                if !self.controller() {
                    return Err(Failure::Workspace(workspace::Failure::Observer));
                }
                let session = self.session().ok_or(Failure::Target)?;
                let view = session
                    .windows()
                    .find(|w| w.id() == window)
                    .ok_or(Failure::Target)?;
                let position = match divider.split.axis {
                    workspace::Axis::Columns => point.x,
                    workspace::Axis::Rows => point.y,
                };
                if let Some(next) = divider.ratio(position)
                    && view.preview(divider, next, session.canvas()).is_ok()
                {
                    ratio = next;
                }
                if event.kind == MouseEventKind::Up(MouseButton::Left) {
                    return Ok(Some(vec![Command::Workspace(Operation::Adjust {
                        window,
                        divider,
                        ratio,
                    })]));
                }
                self.pointer = Some(Pointer::Resize {
                    window,
                    divider,
                    ratio,
                });
                self.status(format!(
                    "Divider {} / 1000; release commits, Escape cancels",
                    ratio.value()
                ));
                Ok(Some(Vec::new()))
            }
            Pointer::Application { pane, rect, button } => {
                let commands = self.application(
                    Placement {
                        pane,
                        rect,
                        frame: rect,
                    },
                    point,
                    event,
                )?;
                if event.kind != MouseEventKind::Up(button) {
                    self.pointer = Some(Pointer::Application { pane, rect, button });
                }
                Ok(Some(commands))
            }
        }
    }
    fn dragged(
        &mut self,
        pointer: Pointer,
        event: MouseEvent,
        point: Point,
    ) -> Result<Vec<Command>, Failure> {
        let Pointer::Selection {
            pane,
            rect,
            start,
            mut dragged,
        } = pointer
        else {
            return Err(Failure::Payload);
        };
        let end = Point {
            x: point
                .x
                .saturating_sub(rect.x)
                .min(rect.geometry.columns() - 1),
            y: point.y.saturating_sub(rect.y).min(rect.geometry.rows() - 1),
        };
        if matches!(
            event.kind,
            MouseEventKind::Drag(MouseButton::Left) | MouseEventKind::Up(MouseButton::Left)
        ) && (dragged || end != start)
        {
            if !dragged {
                self.select(pane, start, false)?;
            }
            dragged = true;
            self.select(pane, end, true)?;
        }
        if event.kind == MouseEventKind::Up(MouseButton::Left) {
            if dragged && self.mouse.release {
                self.copy()?;
            }
        } else {
            self.pointer = Some(Pointer::Selection {
                pane,
                rect,
                start,
                dragged,
            });
        }
        Ok(Vec::new())
    }
}
