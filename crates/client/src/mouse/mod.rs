mod pointer;
use crate::App;
use crossterm::event::{KeyModifiers, MouseButton, MouseEvent, MouseEventKind};
pub(crate) use pointer::Pointer;
use protocol::{Command, Failure, Input};
use terminal::{Action, Button, Direction, Modifiers, Mouse, Point};
use workspace::{Operation, Placement};

impl App {
    pub fn mouse(&mut self, event: MouseEvent) -> Result<Vec<Command>, Failure> {
        if !self.mouse.enabled {
            self.cancel();
            return Ok(Vec::new());
        }
        if event.column >= self.viewport.width() || event.row >= self.viewport.height() {
            self.cancel();
            return Ok(Vec::new());
        }
        let point = Point {
            x: event.column + self.offset.x,
            y: event.row + self.offset.y,
        };
        if let Some(commands) = self.captured(event, point)? {
            return Ok(commands);
        }
        if let Some(commands) = self.heading(event, point)? {
            return Ok(commands);
        }
        if self.divider(event, point)? {
            return Ok(Vec::new());
        }
        let Some(place) = self
            .placements()?
            .into_iter()
            .find(|place| inside(*place, point))
        else {
            return Ok(Vec::new());
        };
        let tracked = self
            .screen(place.pane)
            .is_some_and(|screen| screen.modes.tracking != terminal::Tracking::Off);
        let local = event.modifiers.contains(KeyModifiers::SHIFT)
            || !tracked
            || self.history(place.pane).is_some();
        if let Some(rows) = wheel(event.kind)
            && local
        {
            self.cancel();
            if self.focus() != Some(place.pane) {
                return Err(Failure::Target);
            }
            return self.scroll(place.pane, rows);
        }
        let mut commands = Vec::new();
        if matches!(event.kind, MouseEventKind::Down(MouseButton::Left)) {
            if self.controller() && self.focus() != Some(place.pane) {
                commands.push(Command::Workspace(Operation::Focus(place.pane)));
            }
            if local || !self.controller() {
                self.arm(place, point);
                return Ok(commands);
            }
        }
        if local {
            return Ok(commands);
        }
        if let MouseEventKind::Down(button) = event.kind {
            self.pointer = Some(Pointer::Application {
                pane: place.pane,
                rect: place.rect,
                button,
            });
        }
        commands.extend(self.application(place, point, event)?);
        Ok(commands)
    }

    fn heading(&self, event: MouseEvent, point: Point) -> Result<Option<Vec<Command>>, Failure> {
        if event.kind != MouseEventKind::Down(MouseButton::Left)
            || !self.session().is_some_and(|session| session.header())
        {
            return Ok(None);
        }
        let place = self.placements()?.into_iter().find(|place| {
            point.y == place.frame.y
                && (place.frame.x..place.frame.x + place.frame.geometry.columns())
                    .contains(&point.x)
        });
        let Some(place) = place else {
            return Ok(None);
        };
        if self.controller() && self.focus() != Some(place.pane) {
            return Ok(Some(vec![Command::Workspace(Operation::Focus(place.pane))]));
        }
        Ok(Some(Vec::new()))
    }

    fn application(
        &self,
        place: Placement,
        point: Point,
        event: MouseEvent,
    ) -> Result<Vec<Command>, Failure> {
        if !self.controller() {
            return Err(Failure::Workspace(workspace::Failure::Observer));
        }
        let screen = self
            .screens
            .get(&place.pane)
            .and_then(protocol::Replica::state)
            .ok_or(Failure::Baseline)?;
        let mouse = Mouse {
            position: Point {
                x: point
                    .x
                    .saturating_sub(place.rect.x)
                    .min(place.rect.geometry.columns() - 1),
                y: point
                    .y
                    .saturating_sub(place.rect.y)
                    .min(place.rect.geometry.rows() - 1),
            },
            action: action(event.kind)?,
            modifiers: Modifiers {
                shift: event.modifiers.contains(KeyModifiers::SHIFT),
                alt: event.modifiers.contains(KeyModifiers::ALT),
                control: event.modifiers.contains(KeyModifiers::CONTROL),
            },
        };
        match terminal::Input::Mouse(mouse).encode(screen.modes, screen.extent) {
            Ok(_) => Ok(vec![Command::Send {
                pane: place.pane,
                input: Input::Mouse(mouse),
            }]),
            Err(terminal::Failure::Inactive) => Ok(Vec::new()),
            Err(_) => Err(Failure::Unavailable),
        }
    }
}

fn inside(place: Placement, point: Point) -> bool {
    let rect = place.rect;
    match (point.x.checked_sub(rect.x), point.y.checked_sub(rect.y)) {
        (Some(x), Some(y)) => x < rect.geometry.columns() && y < rect.geometry.rows(),
        _ => false,
    }
}

fn wheel(kind: MouseEventKind) -> Option<i32> {
    match kind {
        MouseEventKind::ScrollUp => Some(-3),
        MouseEventKind::ScrollDown => Some(3),
        _ => None,
    }
}

fn button(button: MouseButton) -> Button {
    match button {
        MouseButton::Left => Button::Left,
        MouseButton::Middle => Button::Middle,
        MouseButton::Right => Button::Right,
    }
}

fn action(kind: MouseEventKind) -> Result<Action, Failure> {
    Ok(match kind {
        MouseEventKind::Down(value) => Action::Press(button(value)),
        MouseEventKind::Up(value) => Action::Release(button(value)),
        MouseEventKind::Drag(value) => Action::Move(Some(button(value))),
        MouseEventKind::Moved => Action::Move(None),
        MouseEventKind::ScrollUp => Action::Wheel(Direction::Up),
        MouseEventKind::ScrollDown => Action::Wheel(Direction::Down),
        _ => return Err(Failure::Unavailable),
    })
}
