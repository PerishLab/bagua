use super::*;

pub(super) struct Fence {
    request: u64,
    started: Instant,
    revision: Option<u64>,
    finished: bool,
    focus: Option<Id>,
    action: &'static str,
}

impl Fence {
    pub fn new(request: &Request) -> Self {
        let focus = match request.command {
            Command::Workspace(Operation::Focus(pane)) => Some(pane),
            _ => None,
        };
        Self {
            request: request.id,
            started: Instant::now(),
            revision: None,
            finished: false,
            focus,
            action: action(&request.command),
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
                .find(|job| matches!(job, Job::Resolve { .. } | Job::Start(_))),
        ) {
            (None, None) => Gate::Open,
            _ => Gate::Native,
        }
    }

    pub(super) fn fenced(&self) -> bool {
        self.gate() != Gate::Open
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
        if !self.launches.is_empty() || self.jobs.values().any(|job| matches!(job, Job::Start(_))) {
            return;
        }
        if fence.focus.is_some() && fence.focus != self.app.focus() {
            self.fence = None;
            self.refuse("Focus changed; dependent input was discarded");
            return;
        }
        self.fence = None;
        self.routing.settle(&mut self.intents);
    }

    fn action(&self, request: u64) -> &'static str {
        if self.owns(request) {
            return self.fence.as_ref().unwrap().action;
        }
        match self.jobs.get(&request) {
            Some(Job::Resolve { .. }) => "Read current directory",
            Some(Job::Start(_)) => "Start pane",
            _ => "Workspace operation",
        }
    }

    fn owns(&self, request: u64) -> bool {
        self.fence
            .as_ref()
            .is_some_and(|fence| fence.request == request)
    }

    pub(super) fn settlement(&mut self, message: &Message) -> Option<super::recovery::Resolution> {
        let request = match message {
            Message::Reply(reply) => reply.request,
            Message::Finished { request, .. } => *request,
            _ => return None,
        };
        let native = matches!(
            self.jobs.get(&request),
            Some(Job::Start(_) | Job::Resolve { .. })
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
                if unknown.is_some() {
                    return unknown;
                }
                match result {
                    Err(fault) => Some(super::recovery::Resolution::Refused(format!(
                        "{}: {}",
                        self.action(request),
                        fault.message
                    ))),
                    Ok(()) => None,
                }
            }
            _ => None,
        }
    }

    fn acknowledge(&mut self, reply: &protocol::Reply) -> Option<super::recovery::Resolution> {
        if let Err(failure) = &reply.result {
            let reason = format!("{}: {}", self.action(reply.request), failure.message());
            if self.owns(reply.request) {
                self.fence = None;
            }
            return Some(super::recovery::Resolution::Refused(reason));
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
                return Some(super::recovery::Resolution::Unknown(
                    "Layout response lacks its applied revision".into(),
                ));
            }
        };
        self.fence.as_mut().unwrap().apply(change);
        None
    }
}

impl Fence {
    pub(super) fn overdue(&self) -> bool {
        self.started.elapsed() >= Duration::from_secs(5)
    }

    fn finish(&mut self) -> Option<super::recovery::Resolution> {
        self.finished = true;
        self.revision.is_none().then(|| {
            super::recovery::Resolution::Unknown(
                "Layout completion lacks its applied revision".into(),
            )
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

fn action(command: &Command) -> &'static str {
    match command {
        Command::Workspace(Operation::Split { split, .. }) => match split.axis {
            workspace::Axis::Columns => "Split left/right",
            workspace::Axis::Rows => "Split top/bottom",
        },
        Command::Workspace(Operation::Window(_)) => "Create window",
        Command::Workspace(Operation::Focus(_)) => "Select pane",
        Command::Workspace(Operation::Acquire { .. }) => "Acquire control",
        Command::Workspace(Operation::Transfer { .. }) => "Transfer control",
        Command::Workspace(Operation::Close(_)) => "Close target",
        _ => "Workspace operation",
    }
}
