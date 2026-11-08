use std::collections::VecDeque;

#[derive(Default)]
pub(crate) struct Keyboard {
    primary: Stack,
    alternate: Stack,
}

#[derive(Default)]
struct Stack(VecDeque<u8>);

impl Keyboard {
    pub fn flags(&self, alternate: bool) -> u8 {
        let stack = if alternate {
            &self.alternate
        } else {
            &self.primary
        };
        stack.0.back().copied().unwrap_or(0)
    }

    pub fn apply(
        &mut self,
        alternate: bool,
        prefix: u8,
        params: &[&[u16]],
    ) -> Option<(Vec<u8>, bool)> {
        if params.len() > 2 || params.iter().any(|group| group.len() != 1) {
            return None;
        }
        let value = params.first().map(|group| group[0]).unwrap_or(0);
        if prefix == b'?' && params.len() <= 1 && value == 0 {
            return Some((
                format!("\x1b[?{}u", self.flags(alternate)).into_bytes(),
                false,
            ));
        }
        let stack = if alternate {
            &mut self.alternate
        } else {
            &mut self.primary
        };
        match prefix {
            b'>' if params.len() <= 1 => stack.push((value & 1) as u8),
            b'<' if params.len() <= 1 => stack.pop(value.max(1)),
            b'=' => {
                let mode = params.get(1).map(|group| group[0]).unwrap_or(1);
                let flags = stack.0.back().copied().unwrap_or(0);
                let mask = (value & 1) as u8;
                let next = match mode {
                    0 | 1 => mask,
                    2 => flags | mask,
                    3 => flags & !mask,
                    _ => return None,
                };
                if let Some(flags) = stack.0.back_mut() {
                    *flags = next;
                } else {
                    stack.0.push_back(next);
                }
            }
            _ => return None,
        }
        Some((Vec::new(), prefix != b'<' && value & !1 != 0))
    }
}

impl Stack {
    fn push(&mut self, flags: u8) {
        if self.0.is_empty() {
            self.0.push_back(0);
        }
        if self.0.len() == 16 {
            self.0.pop_front();
        }
        self.0.push_back(flags);
    }

    fn pop(&mut self, count: u16) {
        self.0
            .truncate(self.0.len().saturating_sub(usize::from(count)));
    }
}
