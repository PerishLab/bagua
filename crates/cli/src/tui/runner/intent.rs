use super::*;

pub(super) struct Fence {
    request: u64,
    revision: Option<u64>,
    finished: bool,
    focus: Option<Id>,
}

impl Fence {
    pub fn new(request: &Request) -> Self {
        let focus = match request.command {
            Command::Workspace(Operation::Focus(pane)) => Some(pane),
            _ => None,
        };
        Self {
            request: request.id,
            revision: None,
            finished: false,
            focus,
        }
    }
}

#[derive(PartialEq)]
pub(super) enum Gate {
    Open,
    Queued,
    Native,
    Waiting,
}

pub(super) fn mutation(command: &Command) -> bool {
    matches!(command, Command::Workspace(operation) if !matches!(operation, Operation::Read | Operation::Resize { .. }))
}

pub(super) fn changing(gesture: &Gesture) -> bool {
    matches!(
        gesture,
        Gesture::Split(_)
            | Gesture::Window
            | Gesture::Index(_)
            | Gesture::Pane
            | Gesture::Next
            | Gesture::Acquire
            | Gesture::Transfer
            | Gesture::Fixed
            | Gesture::Automatic
            | Gesture::Close
            | Gesture::Destroy
    )
}

impl Runner {
    pub(super) fn gate(&self) -> Gate {
        if self.fence.is_some() {
            return Gate::Waiting;
        }
        if self.queue.iter().any(|item| mutation(&item.command)) {
            return Gate::Queued;
        }
        match (
            self.launches.front(),
            self.jobs
                .values()
                .find(|job| matches!(job, Job::Resolve { .. } | Job::Start)),
        ) {
            (None, None) => Gate::Open,
            _ => Gate::Native,
        }
    }

    pub(super) fn fenced(&self) -> bool {
        self.gate() != Gate::Open
    }

    pub(super) fn pause(&mut self, reason: &str) {
        self.paused = true;
        self.intents.clear();
        self.staged = 0;
        self.leader = false;
        self.app.status(format!(
            "Input paused: {reason}; Escape resumes without replaying buffered gestures"
        ));
    }

    pub(super) fn release(&mut self) {
        let Some(fence) = &self.fence else { return };
        if !fence.finished
            || !fence.revision.is_some_and(|revision| {
                self.app
                    .revision()
                    .is_some_and(|current| current >= revision)
            })
        {
            return;
        }
        if !self.launches.is_empty() || self.jobs.values().any(|job| matches!(job, Job::Start)) {
            return;
        }
        if fence.focus.is_some() && fence.focus != self.app.focus() {
            self.fence = None;
            self.pause("the intended focus disappeared or changed");
            return;
        }
        self.fence = None;
    }

    fn owns(&self, request: u64) -> bool {
        self.fence
            .as_ref()
            .is_some_and(|fence| fence.request == request)
    }

    pub(super) fn settlement(&mut self, message: &Message) -> Option<String> {
        let request = match message {
            Message::Reply(reply) => reply.request,
            Message::Finished { request, .. } => *request,
            _ => return None,
        };
        let native = matches!(
            self.jobs.get(&request),
            Some(Job::Start | Job::Resolve { .. })
        );
        if !self.owns(request) && !native {
            return None;
        }
        match message {
            Message::Reply(reply) => self.acknowledge(reply),
            Message::Finished { result, .. } => {
                let unknown = if self.owns(request) {
                    self.fence.as_mut().unwrap().finish()
                } else {
                    None
                };
                unknown.or_else(|| result.as_ref().err().map(|fault| fault.message.clone()))
            }
            _ => None,
        }
    }

    fn acknowledge(&mut self, reply: &protocol::Reply) -> Option<String> {
        if let Err(failure) = &reply.result {
            if self.owns(reply.request) {
                self.fence = None;
            }
            return Some(failure.message().into());
        }
        if !self.owns(reply.request) {
            return None;
        }
        let change = match &reply.result {
            Ok(protocol::Status::Applied(protocol::Outcome::Workspace(
                workspace::Outcome::Changed(change),
            )))
            | Ok(protocol::Status::Accepted(protocol::Intent::Retire(change))) => change,
            _ => {
                return Some(
                    "layout response lacks its applied revision; inspect host state".into(),
                );
            }
        };
        self.fence.as_mut().unwrap().apply(change);
        None
    }
}

impl Fence {
    fn finish(&mut self) -> Option<String> {
        self.finished = true;
        self.revision.is_none().then(|| {
            "layout outcome unknown: completion lacks its applied revision; inspect host state"
                .into()
        })
    }

    fn apply(&mut self, change: &workspace::Change) {
        self.revision = Some(change.revision);
        if let Some(pane) = change.created.iter().find_map(|target| match target {
            workspace::Target::Pane(pane) => Some(*pane),
            _ => None,
        }) {
            self.focus = Some(pane);
        }
    }
}
