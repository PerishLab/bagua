use protocol::wire::{Fault, Message, Screen};
use workspace::Id;

pub(crate) struct Summary {
    pub pane: Id,
    pub io: bool,
    pub closing: bool,
    pub exited: bool,
    pub code: Option<i32>,
    pub signal: Option<i32>,
    pub eof: bool,
    pub fault: Option<Fault>,
}

pub(crate) fn capture(
    message: &Message,
    summary: &mut Option<Summary>,
    screen: &mut Option<Screen>,
) {
    match message {
        Message::Process {
            pane,
            io,
            closing,
            exited,
            code,
            signal,
            eof,
            fault,
        } => {
            *summary = Some(Summary {
                pane: *pane,
                io: *io,
                closing: *closing,
                exited: *exited,
                code: *code,
                signal: *signal,
                eof: *eof,
                fault: fault.clone(),
            });
        }
        Message::Screen { state, .. } => {
            *screen = Some(state.clone());
        }
        _ => {}
    }
}

pub(crate) fn render(summary: Option<&Summary>, state: Option<&Screen>) -> String {
    let mut lines = Vec::new();
    if let Some(info) = summary {
        lines.push(header(info, state));
    }
    if let Some(grid) = state {
        lines.extend(screen(grid));
    }
    lines.join("\n")
}

fn header(info: &Summary, state: Option<&Screen>) -> String {
    let mut parts = vec![format!("pane {}", info.pane.value()), status(info)];
    if info.eof {
        parts.push("eof".into());
    }
    if !info.io {
        parts.push("io=closed".into());
    }
    if let Some(fault) = &info.fault {
        parts.push(format!("fault={}", fault.code));
    }
    if let Some(grid) = state {
        parts.push(format!("{}x{}", grid.extent.width(), grid.extent.height()));
        parts.push(format!(
            "cursor {},{}",
            grid.cursor.position.x, grid.cursor.position.y
        ));
    }
    parts.join(" | ")
}

fn status(info: &Summary) -> String {
    if info.exited {
        if let Some(code) = info.code {
            return format!("exited({code})");
        }
        if let Some(signal) = info.signal {
            return format!("signal({signal})");
        }
        return "exited".into();
    }
    if info.closing {
        return "closing".into();
    }
    "running".into()
}

pub(crate) fn screen(state: &Screen) -> Vec<String> {
    let width = state.extent.width() as usize;
    let height = state.extent.height() as usize;
    let mut rows = Vec::with_capacity(height);
    for row in 0..height {
        let start = row * width;
        let end = (start + width).min(state.cells.len());
        if start >= end {
            rows.push(String::new());
            continue;
        }
        let mut line = String::new();
        for cell in &state.cells[start..end] {
            if cell.width == 0 {
                continue;
            }
            if cell.text.is_empty() {
                line.push(' ');
            } else {
                line.push_str(&cell.text);
            }
        }
        rows.push(line.trim_end_matches(' ').into());
    }
    while rows.last().is_some_and(String::is_empty) {
        rows.pop();
    }
    rows
}

#[cfg(test)]
mod tests {
    use super::{Summary, render, screen};
    use protocol::wire::Screen;
    use terminal::{
        Cell, Color, Cursor, Encoding, Extent, Modes, Point, Revision, Style, Tracking,
    };
    use workspace::Id;

    fn blank() -> Style {
        Style {
            foreground: Color::Default,
            background: Color::Default,
            bold: false,
            dim: false,
            italic: false,
            underline: false,
            inverse: false,
        }
    }

    fn cell(text: &str, width: u8) -> Cell {
        Cell {
            text: text.into(),
            width,
            style: blank(),
        }
    }

    fn sample() -> Screen {
        let cells = vec![
            cell("h", 1),
            cell("i", 1),
            cell(" ", 1),
            cell("中", 2),
            cell("", 0),
            cell("", 1),
            cell("o", 1),
            cell("k", 1),
            cell("", 1),
            cell("", 1),
            cell("", 1),
            cell("", 1),
            cell("", 1),
            cell("", 1),
            cell("", 1),
            cell("", 1),
            cell("", 1),
            cell("", 1),
        ];
        Screen {
            bytes: 12,
            unsupported: 0,
            bells: 0,
            revision: Revision::new(1),
            extent: Extent::new(6, 3).unwrap(),
            cursor: Cursor {
                position: Point { x: 2, y: 1 },
                visible: true,
                pending: false,
            },
            modes: Modes {
                keyboard: 0,
                cursor: false,
                keypad: false,
                paste: false,
                alternate: false,
                tracking: Tracking::Off,
                encoding: Encoding::Legacy,
            },
            cells,
            wrapped: vec![false; 3],
        }
    }

    #[test]
    fn wide() {
        let state = sample();
        assert_eq!(screen(&state), vec!["hi 中", "ok"]);
    }

    #[test]
    fn summary() {
        let state = sample();
        let info = Summary {
            pane: Id::new(6).unwrap(),
            io: true,
            closing: false,
            exited: false,
            code: None,
            signal: None,
            eof: false,
            fault: None,
        };
        let text = render(Some(&info), Some(&state));
        assert_eq!(text, "pane 6 | running | 6x3 | cursor 2,1\nhi 中\nok");

        let exited = Summary {
            exited: true,
            code: Some(0),
            io: false,
            eof: true,
            ..info
        };
        let done = render(Some(&exited), Some(&state));
        assert!(done.starts_with("pane 6 | exited(0) | eof | io=closed | 6x3 | cursor 2,1\n"));
    }
}
