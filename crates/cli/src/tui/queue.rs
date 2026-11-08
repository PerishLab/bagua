use super::runner::Runner;
use protocol::Command;

#[derive(Clone)]
pub(super) struct Queued {
    pub command: Command,
    pub source: Option<workspace::Id>,
    pub program: Option<protocol::wire::Program>,
}

impl Runner {
    pub(super) fn enqueue(&mut self, command: Command) {
        let bytes = weight(&command);
        if self.bytes + bytes > 65_536 {
            self.app.status("Input byte budget full; operation refused");
            return;
        }
        if let Some(pending) = self.queue.back_mut()
            && let Some(count) = append(&mut pending.command, &command)
        {
            self.bytes += count;
            return;
        }
        if self.queue.len() >= 64 {
            self.app
                .status("Input operation budget full; operation refused");
            return;
        }
        self.bytes += bytes;
        let source = match &command {
            Command::Workspace(workspace::Operation::Split { pane, .. }) => Some(*pane),
            Command::Workspace(workspace::Operation::Window(_)) => self.app.focus(),
            _ => None,
        };
        self.queue.push_back(Queued {
            command,
            source,
            program: None,
        });
    }
}
pub(super) fn weight(command: &Command) -> usize {
    match command {
        Command::Send {
            input: protocol::Input::Text(text) | protocol::Input::Paste(text),
            ..
        } => text.len(),
        _ => 64,
    }
}

fn append(pending: &mut Command, next: &Command) -> Option<usize> {
    match (pending, next) {
        (
            Command::Send {
                pane: previous,
                input: protocol::Input::Text(text),
            },
            Command::Send {
                pane,
                input: protocol::Input::Text(extra),
            },
        ) if previous == pane && text.len() + extra.len() <= terminal::CHUNK => {
            text.push_str(extra);
            Some(extra.len())
        }
        _ => None,
    }
}
