use crate::{Cell, Cursor, Extent, Failure, History, Modes, Revision};

#[derive(Clone)]
pub struct Snapshot {
    screen: vt100::Screen,
    revision: Revision,
    history: History,
    offset: usize,
    rows: Vec<(Vec<Cell>, bool)>,
    column: u16,
}

impl Snapshot {
    pub(crate) fn new(screen: vt100::Screen, revision: Revision, history: History) -> Self {
        Self {
            screen,
            revision,
            history,
            offset: 0,
            rows: Vec::new(),
            column: 0,
        }
    }

    pub fn revision(&self) -> Revision {
        self.revision
    }

    pub fn extent(&self) -> Extent {
        let (height, width) = self.screen.size();
        Extent { width, height }
    }

    pub fn contents(&self) -> String {
        if self.offset == 0 {
            return self.screen.contents();
        }
        let mut text = String::new();
        for (index, (cells, wrapped)) in self.rows.iter().enumerate() {
            let row: String = cells
                .iter()
                .map(|cell| {
                    if cell.text.is_empty() && cell.width != 0 {
                        " "
                    } else {
                        &cell.text
                    }
                })
                .collect();
            text.push_str(row.trim_end_matches(' '));
            if !wrapped && index + 1 < self.rows.len() {
                text.push('\n');
            }
        }
        text.trim_end_matches('\n').into()
    }

    pub fn cell(&self, position: crate::Point) -> Option<Cell> {
        if self.offset != 0 {
            return self
                .rows
                .get(usize::from(position.y))?
                .0
                .get(usize::from(position.x))
                .cloned();
        }
        self.screen.cell(position.y, position.x).map(Cell::from)
    }

    pub fn wrapped(&self, row: u16) -> bool {
        if self.offset != 0 {
            return self.rows.get(usize::from(row)).is_some_and(|row| row.1);
        }
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
            visible: !self.screen.hide_cursor() && self.offset == 0,
            pending: x >= width,
        }
    }

    pub fn modes(&self) -> Modes {
        Modes::from(&self.screen)
    }

    pub fn history(&self) -> usize {
        if self.modes().alternate {
            0
        } else {
            self.history.count()
        }
    }

    pub fn offset(&self) -> usize {
        self.offset
    }

    pub fn view(&self, offset: usize) -> Result<Self, Failure> {
        self.window(offset, 0)
    }

    pub fn window(&self, offset: usize, column: u16) -> Result<Self, Failure> {
        if offset == 0 && column != 0 {
            return Err(Failure::Position);
        }
        if u32::from(column) + u32::from(self.extent().width()) > u32::from(crate::COLUMNS) {
            return Err(Failure::Position);
        }
        if offset > self.history() {
            return Err(Failure::Position);
        }
        let mut view = self.clone();
        view.offset = offset;
        view.column = column;
        view.rows.clear();
        if offset != 0 {
            view.populate()?;
        }
        Ok(view)
    }

    fn populate(&mut self) -> Result<(), Failure> {
        let height = usize::from(self.extent().height());
        let count = self.offset.min(height);
        let first = self.history.range().next - self.offset as u64;
        for start in (0..count).step_by(crate::PAGE) {
            self.extend(first + start as u64, (count - start).min(crate::PAGE))?;
        }
        let empty = vt100::Parser::new(self.extent().height, self.extent().width, 0);
        for y in 0..(height - count) as u16 {
            let cells = (0..self.extent().width())
                .map(|x| {
                    Cell::from(
                        self.screen
                            .cell(y, x + self.column)
                            .unwrap_or_else(|| empty.screen().cell(0, 0).unwrap()),
                    )
                })
                .collect();
            self.rows.push((edges(cells), self.screen.row_wrapped(y)));
        }
        Ok(())
    }

    fn extend(&mut self, first: u64, count: usize) -> Result<(), Failure> {
        for line in self.history.page(first, count)? {
            let mut parser = vt100::Parser::new(
                std::num::NonZeroU16::new(1).unwrap(),
                std::num::NonZeroU16::new(line.columns.max(self.extent().width() + self.column))
                    .unwrap(),
                0,
            );
            parser.process(line.formatted.as_bytes());
            let cells = (0..self.extent().width())
                .map(|x| Cell::from(parser.screen().cell(0, x + self.column).unwrap()))
                .collect();
            self.rows.push((edges(cells), line.wrapped));
        }
        Ok(())
    }
}

fn edges(mut cells: Vec<Cell>) -> Vec<Cell> {
    if let Some(cell) = cells.first_mut()
        && cell.width == 0
    {
        cell.text.clear();
        cell.width = 1;
    }
    if let Some(cell) = cells.last_mut()
        && cell.width == 2
    {
        cell.text.clear();
        cell.width = 1;
    }
    cells
}
