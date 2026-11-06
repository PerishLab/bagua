use crate::App;
use crossterm::event::{KeyModifiers, MouseButton, MouseEvent, MouseEventKind};
use protocol::{Command, Failure, Input};
use terminal::{Action, Button, Direction, Modifiers, Mouse, Point};
use workspace::Operation;

impl App {
    pub fn mouse(&mut self, event: MouseEvent) -> Result<Vec<Command>, Failure> {
        if event.column >= self.viewport.width() || event.row >= self.viewport.height() {
            return Ok(Vec::new());
        }
        let point = Point {
            x: event.column + self.offset.x,
            y: event.row + self.offset.y,
        };
        let place = self.placements()?.into_iter().find(|place| {
            let rect = place.rect;
            match (point.x.checked_sub(rect.x), point.y.checked_sub(rect.y)) {
                (Some(x), Some(y)) => x < rect.geometry.columns() && y < rect.geometry.rows(),
                _ => false,
            }
        });
        let Some(place) = place else {
            return Ok(Vec::new());
        };
        let local = Point {
            x: point.x - place.rect.x,
            y: point.y - place.rect.y,
        };
        if event.modifiers.contains(KeyModifiers::SHIFT) {
            let extend = match event.kind {
                MouseEventKind::Down(MouseButton::Left) => false,
                MouseEventKind::Drag(MouseButton::Left) | MouseEventKind::Up(MouseButton::Left) => {
                    true
                }
                _ => return Ok(Vec::new()),
            };
            self.select(place.pane, local, extend)?;
            return Ok(Vec::new());
        }
        if !self.controller() {
            return Err(Failure::Workspace(workspace::Failure::Observer));
        }
        let mut commands = Vec::new();
        if matches!(event.kind, MouseEventKind::Down(MouseButton::Left))
            && self.focus() != Some(place.pane)
        {
            commands.push(Command::Workspace(Operation::Focus(place.pane)));
        }
        let Some(screen) = self.screen(place.pane) else {
            return Ok(commands);
        };
        if screen.modes.tracking == terminal::Tracking::Off {
            return Ok(commands);
        }
        let action = action(event.kind)?;
        let modifiers = Modifiers {
            shift: false,
            alt: event.modifiers.contains(KeyModifiers::ALT),
            control: event.modifiers.contains(KeyModifiers::CONTROL),
        };
        let mouse = Mouse {
            position: local,
            action,
            modifiers,
        };
        match terminal::Input::Mouse(mouse).encode(screen.modes, screen.extent) {
            Ok(_) => {}
            Err(terminal::Failure::Inactive) => return Ok(commands),
            Err(_) => return Err(Failure::Unavailable),
        }
        commands.push(Command::Send {
            pane: place.pane,
            input: Input::Mouse(mouse),
        });
        Ok(commands)
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
