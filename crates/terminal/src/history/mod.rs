mod capture;

use std::collections::VecDeque;
use std::sync::Arc;

use crate::{Failure, HISTORY};

const SEGMENT: usize = 64;
pub const STORAGE: usize = 64 * 1024 * 1024;
pub const PAGE: usize = 128;

#[derive(Clone, Debug)]
pub struct Line {
    pub id: u64,
    pub columns: u16,
    pub wrapped: bool,
    pub formatted: String,
}

impl Line {
    pub fn text(&self) -> String {
        vt100::capture::basic_formatted_to_plain(&self.formatted).collect()
    }

    fn cost(&self) -> usize {
        self.formatted.capacity() + std::mem::size_of::<Self>() + 32
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Range {
    pub first: u64,
    pub next: u64,
    pub bytes: usize,
    pub evicted: u64,
}

#[derive(Clone, Default)]
struct Index {
    pages: VecDeque<Arc<VecDeque<Arc<Line>>>>,
    count: usize,
    bytes: usize,
    next: u64,
    evicted: u64,
}

#[derive(Clone, Default)]
pub struct History {
    index: Arc<Index>,
    limit: usize,
    budget: usize,
}

impl History {
    pub fn new(limit: usize, budget: usize) -> Result<Self, Failure> {
        if limit > HISTORY || budget > STORAGE {
            return Err(Failure::Capacity);
        }
        Ok(Self {
            limit,
            budget,
            ..Self::default()
        })
    }

    pub fn count(&self) -> usize {
        self.index.count
    }

    pub fn empty(&self) -> bool {
        self.index.count == 0
    }

    pub fn range(&self) -> Range {
        Range {
            first: self.index.next - self.index.count as u64,
            next: self.index.next,
            bytes: self.index.bytes,
            evicted: self.index.evicted,
        }
    }

    pub fn page(&self, first: u64, count: usize) -> Result<Vec<Arc<Line>>, Failure> {
        let range = self.range();
        if count > PAGE || first < range.first || first > range.next {
            return Err(Failure::Position);
        }
        if first
            .checked_add(count as u64)
            .is_none_or(|end| end > range.next)
        {
            return Err(Failure::Position);
        }
        let mut skip = (first - range.first) as usize;
        let mut lines = Vec::with_capacity(count);
        for page in &self.index.pages {
            if skip >= page.len() {
                skip -= page.len();
                continue;
            }
            lines.extend(page.iter().skip(skip).take(count - lines.len()).cloned());
            skip = 0;
            if lines.len() == count {
                break;
            }
        }
        Ok(lines)
    }

    pub(crate) fn push(&mut self, mut line: Line) {
        let index = Arc::make_mut(&mut self.index);
        line.id = index.next;
        index.next = index
            .next
            .checked_add(1)
            .expect("history identity exhausted");
        index.bytes += line.cost();
        index.count += 1;
        if index.pages.back().is_none_or(|page| page.len() == SEGMENT) {
            index
                .pages
                .push_back(Arc::new(VecDeque::with_capacity(SEGMENT)));
        }
        Arc::make_mut(index.pages.back_mut().unwrap()).push_back(Arc::new(line));
        self.trim(self.budget);
    }

    pub fn trim(&mut self, budget: usize) {
        let index = Arc::make_mut(&mut self.index);
        while index.count > self.limit || index.bytes > budget {
            let page = Arc::make_mut(index.pages.front_mut().unwrap());
            let line = page.pop_front().unwrap();
            index.bytes -= line.cost();
            index.count -= 1;
            index.evicted = index.evicted.saturating_add(1);
            if page.is_empty() {
                index.pages.pop_front();
            }
        }
    }

    pub(crate) fn budget(&mut self, bytes: usize) {
        self.budget = bytes.min(STORAGE);
        self.trim(self.budget);
    }

    pub(crate) fn clear(&mut self) {
        let index = Arc::make_mut(&mut self.index);
        index.pages.clear();
        index.bytes = 0;
        index.evicted = index.evicted.saturating_add(index.count as u64);
        index.count = 0;
    }
}
