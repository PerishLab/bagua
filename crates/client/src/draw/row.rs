use std::{fmt::Write, ops::Range};
use terminal::{Cell, Color, Style};
use unicode_width::{UnicodeWidthChar, UnicodeWidthStr};

pub(crate) struct Row {
    cells: Vec<Cell>,
    targets: Vec<Option<workspace::Id>>,
}

pub(super) fn tone(color: bool, index: u8, active: bool) -> Style {
    Style {
        foreground: Color::Default,
        background: if color {
            Color::Indexed(index)
        } else {
            Color::Default
        },
        bold: active,
        dim: false,
        italic: false,
        underline: false,
        inverse: !color && active,
    }
}

impl Row {
    pub fn new(width: u16) -> Self {
        Self {
            cells: vec![
                Cell {
                    text: String::new(),
                    width: 1,
                    style: tone(false, 0, false)
                };
                usize::from(width)
            ],
            targets: vec![None; usize::from(width)],
        }
    }

    pub fn fill(&mut self, range: Range<u16>, style: Style) {
        for x in range {
            if let Some(cell) = self.cells.get_mut(usize::from(x)) {
                *cell = Cell {
                    text: String::new(),
                    width: 1,
                    style,
                };
                self.targets[usize::from(x)] = None;
            }
        }
    }

    pub fn mark(&mut self, range: Range<u16>, target: Option<workspace::Id>) {
        for x in range {
            if let Some(slot) = self.targets.get_mut(usize::from(x)) {
                *slot = target;
            }
        }
    }

    pub fn target(&self, column: u16) -> Option<workspace::Id> {
        self.targets.get(usize::from(column)).copied().flatten()
    }

    pub fn label(&mut self, range: Range<u16>, text: &str, style: Style) -> u16 {
        let mut x = range.start;
        let mut last = None;
        for c in text.chars() {
            let c = if c.is_control() { '?' } else { c };
            let size = UnicodeWidthChar::width(c).unwrap_or(0) as u16;
            if size == 0 {
                if let Some(index) = last {
                    self.combine(index, c);
                }
                continue;
            }
            if x + size > range.end || usize::from(x + size) > self.cells.len() {
                break;
            }
            self.cells[usize::from(x)] = Cell {
                text: c.to_string(),
                width: size as u8,
                style,
            };
            last = Some(usize::from(x));
            if size == 2 {
                self.cells[usize::from(x + 1)] = Cell {
                    text: String::new(),
                    width: 0,
                    style,
                };
            }
            x += size;
        }
        x
    }

    fn combine(&mut self, index: usize, value: char) {
        let cell = &mut self.cells[index];
        if cell.text.len() + value.len_utf8() <= 22 {
            cell.text.push(value);
        }
    }

    pub fn put(&mut self, index: u16, mut cell: Cell) {
        let index = usize::from(index);
        if index >= self.cells.len() {
            return;
        }
        match cell.width {
            0 if index == 0 || self.cells[index - 1].width != 2 => {
                cell.width = 1;
                cell.text.clear();
            }
            2 if index + 1 == self.cells.len() => {
                cell.width = 1;
                cell.text.clear();
            }
            _ => {}
        }
        self.cells[index] = cell;
    }

    pub fn encode(&self, y: u16) -> String {
        let mut output = format!("\x1b[0m\x1b[{};1H\x1b[2K", y + 1);
        let mut previous = None;
        for cell in &self.cells {
            if cell.width == 0 {
                continue;
            }
            if previous != Some(cell.style) {
                sgr(&mut output, cell.style);
                previous = Some(cell.style);
            }
            match (
                cell.text.is_empty(),
                UnicodeWidthStr::width(cell.text.as_str()) == usize::from(cell.width),
            ) {
                (false, true) => output.push_str(&cell.text),
                _ => output.push_str(&" ".repeat(usize::from(cell.width))),
            }
        }
        output.push_str("\x1b[0m");
        output
    }
}

fn sgr(output: &mut String, style: Style) {
    output.push_str("\x1b[0m");
    for (enabled, number) in [
        (style.bold, 1),
        (style.dim, 2),
        (style.italic, 3),
        (style.underline, 4),
        (style.inverse, 7),
    ] {
        if enabled {
            write!(output, "\x1b[{number}m").unwrap();
        }
    }
    for (color, prefix) in [(style.foreground, 38), (style.background, 48)] {
        match color {
            Color::Default => {}
            Color::Indexed(index) => write!(output, "\x1b[{prefix};5;{index}m").unwrap(),
            Color::Rgb(r, g, b) => write!(output, "\x1b[{prefix};2;{r};{g};{b}m").unwrap(),
        }
    }
}
