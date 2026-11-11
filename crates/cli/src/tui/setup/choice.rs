use super::{Display, Error};
use client::{App, Gesture, Keyboard};
use crossterm::event::{Event, KeyCode};
use std::{sync::atomic::Ordering, time::Instant};

pub(super) enum Choice {
    Take,
    Observe,
    Cancel,
}

pub(super) fn choose(
    display: &mut Display<'_>,
    app: &mut App,
    caption: &str,
    config: &crate::config::Loaded,
) -> Result<Choice, Error> {
    let mut keyboard = Keyboard::new(config.delay());
    let mut size = (0, 0);
    loop {
        if display.stop.load(Ordering::Relaxed) {
            return Ok(Choice::Cancel);
        }
        let current = display.terminal.size()?;
        if current != size {
            size = current;
            if let Some(viewport) = super::super::extent(size.0, size.1, display.rows) {
                app.resize(viewport);
                app.error(caption);
                display.terminal.present(app.paint()?)?;
            }
        }
        display.terminal.flush()?;
        let mut bytes = [0; 4096];
        let count = display.terminal.read(&mut bytes)?;
        keyboard.push(&bytes[..count])?;
        while let Some(event) = keyboard.next(Instant::now()) {
            if let Ok(Event::Key(key)) = event
                && let Some(choice) = selected(key, config.keys())
            {
                return Ok(choice);
            }
        }
        std::thread::sleep(std::time::Duration::from_millis(2));
    }
}

fn selected(key: crossterm::event::KeyEvent, keys: &client::Keys) -> Option<Choice> {
    if key.kind == crossterm::event::KeyEventKind::Release {
        return None;
    }
    if keys.translate(key, false) == Ok(Gesture::Detach) {
        return Some(Choice::Cancel);
    }
    if !key.modifiers.is_empty() && key.modifiers != crossterm::event::KeyModifiers::SHIFT {
        return None;
    }
    match key.code {
        KeyCode::Char('t' | 'T') => Some(Choice::Take),
        KeyCode::Char('o' | 'O') => Some(Choice::Observe),
        KeyCode::Char('q' | 'Q') | KeyCode::Esc => Some(Choice::Cancel),
        _ => None,
    }
}
