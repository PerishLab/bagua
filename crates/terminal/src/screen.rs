use crate::{Extent, Failure, Revision};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Color {
    Default,
    Indexed(u8),
    Rgb(u8, u8, u8),
}

impl From<vt100::Color> for Color {
    fn from(color: vt100::Color) -> Self {
        match color {
            vt100::Color::Default => Self::Default,
            vt100::Color::Idx(index) => Self::Indexed(index),
            vt100::Color::Rgb(red, green, blue) => Self::Rgb(red, green, blue),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Style {
    pub foreground: Color,
    pub background: Color,
    pub bold: bool,
    pub dim: bool,
    pub italic: bool,
    pub underline: bool,
    pub inverse: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Cell {
    pub text: String,
    pub width: u8,
    pub style: Style,
}

impl From<&vt100::Cell> for Cell {
    fn from(cell: &vt100::Cell) -> Self {
        let width = if cell.is_wide_continuation() {
            0
        } else if cell.is_wide() {
            2
        } else {
            1
        };
        Self {
            text: cell.contents().into(),
            width,
            style: Style {
                foreground: cell.fgcolor().into(),
                background: cell.bgcolor().into(),
                bold: cell.bold(),
                dim: cell.dim(),
                italic: cell.italic(),
                underline: cell.underline(),
                inverse: cell.inverse(),
            },
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Tracking {
    Off,
    Press,
    Button,
    Drag,
    Motion,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Encoding {
    Legacy,
    Utf8,
    Sgr,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Modes {
    pub cursor: bool,
    pub keypad: bool,
    pub paste: bool,
    pub alternate: bool,
    pub tracking: Tracking,
    pub encoding: Encoding,
}

impl From<&vt100::Screen> for Modes {
    fn from(screen: &vt100::Screen) -> Self {
        let tracking = match screen.mouse_protocol_mode() {
            vt100::MouseProtocolMode::None => Tracking::Off,
            vt100::MouseProtocolMode::Press => Tracking::Press,
            vt100::MouseProtocolMode::PressRelease => Tracking::Button,
            vt100::MouseProtocolMode::ButtonMotion => Tracking::Drag,
            vt100::MouseProtocolMode::AnyMotion => Tracking::Motion,
        };
        let encoding = match screen.mouse_protocol_encoding() {
            vt100::MouseProtocolEncoding::Default => Encoding::Legacy,
            vt100::MouseProtocolEncoding::Utf8 => Encoding::Utf8,
            vt100::MouseProtocolEncoding::Sgr => Encoding::Sgr,
        };
        Self {
            cursor: screen.application_cursor(),
            keypad: screen.application_keypad(),
            paste: screen.bracketed_paste(),
            alternate: screen.alternate_screen(),
            tracking,
            encoding,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Cursor {
    pub position: crate::Point,
    pub visible: bool,
    pub pending: bool,
}

#[derive(Clone)]
pub struct Snapshot {
    screen: vt100::Screen,
    revision: Revision,
}

impl Snapshot {
    pub(crate) fn new(screen: vt100::Screen, revision: Revision) -> Self {
        Self { screen, revision }
    }

    pub fn revision(&self) -> Revision {
        self.revision
    }

    pub fn extent(&self) -> Extent {
        let (height, width) = self.screen.size();
        Extent { width, height }
    }

    pub fn contents(&self) -> String {
        self.screen.contents()
    }

    pub fn cell(&self, position: crate::Point) -> Option<Cell> {
        self.screen.cell(position.y, position.x).map(Cell::from)
    }

    pub fn wrapped(&self, row: u16) -> bool {
        row < self.extent().height() && self.screen.row_wrapped(row)
    }

    pub fn cursor(&self) -> Cursor {
        let (y, x) = self.screen.cursor_position();
        let width = self.extent().width();
        Cursor {
            position: crate::Point {
                x: x.min(width - 1),
                y,
            },
            visible: !self.screen.hide_cursor() && self.screen.scrollback() == 0,
            pending: x >= width,
        }
    }

    pub fn modes(&self) -> Modes {
        Modes::from(&self.screen)
    }

    pub fn history(&self) -> usize {
        self.screen.scrollback_len()
    }

    pub fn offset(&self) -> usize {
        self.screen.scrollback()
    }

    pub fn view(&self, offset: usize) -> Result<Self, Failure> {
        if offset > self.history() {
            return Err(Failure::Position);
        }
        let mut view = self.clone();
        view.screen.set_scrollback(offset);
        Ok(view)
    }
}
