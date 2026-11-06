use crate::Revision;

#[derive(Debug, PartialEq, Eq)]
pub struct Report {
    pub revision: Revision,
    pub unsupported: u64,
    pub bells: u64,
    pub replies: Vec<u8>,
}

#[derive(Default)]
pub(crate) struct Events {
    unsupported: u64,
    bells: u64,
    replies: Vec<u8>,
}

impl Events {
    pub fn bell(&mut self) {
        self.bells += 1;
    }

    pub fn discard(&mut self) {
        self.unsupported += 1;
    }

    pub fn respond(
        &mut self,
        screen: &vt100::Screen,
        prefix: (Option<u8>, Option<u8>),
        params: &[&[u16]],
        action: char,
    ) {
        match query(screen, prefix, params, action) {
            Some(reply) => self.replies.extend_from_slice(reply.as_bytes()),
            None => self.discard(),
        }
    }

    pub fn take(&mut self, revision: Revision) -> Report {
        let events = std::mem::take(self);
        Report {
            revision,
            unsupported: events.unsupported,
            bells: events.bells,
            replies: events.replies,
        }
    }
}

fn query(
    screen: &vt100::Screen,
    prefix: (Option<u8>, Option<u8>),
    params: &[&[u16]],
    action: char,
) -> Option<String> {
    if prefix != (None, None) || params.len() != 1 || params[0].len() != 1 {
        return None;
    }
    match (action, params[0][0]) {
        ('n', 5) => Some("\x1b[0n".into()),
        ('n', 6) => {
            let (row, col) = screen.cursor_position();
            let (_, columns) = screen.size();
            Some(format!(
                "\x1b[{};{}R",
                row + 1,
                col.min(columns.get() - 1) + 1
            ))
        }
        ('t', 18) => {
            let (rows, columns) = screen.size();
            Some(format!("\x1b[8;{};{}t", rows, columns))
        }
        _ => None,
    }
}
