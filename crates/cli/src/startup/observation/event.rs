use locus::{Candidate, Context, Engine};
use serde_json::{Value, json};

pub(super) fn native(engine: &Engine, context: &Context, event: bagua::Event) {
    use bagua::Event;
    let value = match event {
        Event::Connect(id) => json!({"event": "host.connect", "actor": id.value()}),
        Event::Disconnect(id) => json!({"event": "host.disconnect", "actor": id.value()}),
        Event::Start(id) => json!({"event": "pane.start", "pane": id.value()}),
        Event::Eof(id) => json!({"event": "pane.eof", "pane": id.value()}),
        Event::Exit(id, exit) => {
            json!({"event": "pane.reaped", "pane": id.value(), "code": exit.code, "signal": exit.signal})
        }
        Event::Release(id) => json!({"event": "pane.release", "pane": id.value()}),
        Event::Retire(id) => json!({"event": "pane.retire", "pane": id.value()}),
        Event::Fault(id) => json!({"event": "pane.fault", "pane": id.value()}),
        Event::Cleanup(id) => json!({"event": "pane.cleanup", "pane": id.value()}),
    };
    append(engine, context, value);
}

fn append(engine: &Engine, context: &Context, value: Value) {
    let _ = locus::record!(engine, context, Candidate::event(value));
}

#[derive(Clone, Copy, PartialEq)]
pub(crate) enum Kind {
    Workspace,
    Start,
    Describe,
    Shutdown,
}

#[derive(Clone, Copy)]
pub(crate) enum Stage {
    Sent,
    Applied,
    Admitted,
    Refused,
    Finished,
    Failed,
    Unknown,
}

pub(crate) fn kind(command: &protocol::Command) -> Option<Kind> {
    use protocol::Command;
    match command {
        Command::Workspace(workspace::Operation::Read | workspace::Operation::Resize { .. }) => {
            None
        }
        Command::Workspace(_) => Some(Kind::Workspace),
        Command::Start { .. } => Some(Kind::Start),
        Command::Describe(_) => Some(Kind::Describe),
        Command::Shutdown => Some(Kind::Shutdown),
        _ => None,
    }
}

pub(crate) fn reply(result: &Result<protocol::Status, protocol::Failure>) -> Stage {
    match result {
        Ok(protocol::Status::Applied(_)) => Stage::Applied,
        Ok(_) => Stage::Admitted,
        Err(_) => Stage::Refused,
    }
}

pub(crate) fn finish(success: bool) -> Stage {
    if success {
        Stage::Finished
    } else {
        Stage::Failed
    }
}

pub(crate) fn terminal(restored: bool) {
    if let Some((engine, context)) = super::view() {
        append(
            engine,
            context,
            json!({"event": "terminal.finish", "restored": restored}),
        );
    }
}

pub(crate) enum Service {
    Listen,
    Stop(bool),
}

pub(crate) struct Scope<'a>(pub Option<&'a Context>);

impl Scope<'_> {
    pub(crate) fn connection(self, actor: workspace::Id) {
        if let Some((engine, _)) = super::view()
            && let Some(context) = self.0
        {
            append(
                engine,
                context,
                json!({"event": "client.connect", "actor": actor.value()}),
            );
        }
    }

    pub(crate) fn receipt(self, kind: Option<Kind>, receipt: (u64, Stage)) {
        let Some((engine, _)) = super::view() else {
            return;
        };
        let (Some(context), Some(kind)) = (self.0, kind) else {
            return;
        };
        if kind == Kind::Describe
            && !matches!(receipt.1, Stage::Refused | Stage::Failed | Stage::Unknown)
        {
            return;
        }
        let operation = match kind {
            Kind::Workspace => "workspace",
            Kind::Start => "start",
            Kind::Describe => "describe",
            Kind::Shutdown => "shutdown",
        };
        let stage = match receipt.1 {
            Stage::Sent => "sent",
            Stage::Applied => "applied",
            Stage::Admitted => "admitted",
            Stage::Refused => "refused",
            Stage::Finished => "finished",
            Stage::Failed => "failed",
            Stage::Unknown => "unknown",
        };
        append(
            engine,
            context,
            json!({"event": "request", "request": receipt.0, "operation": operation, "stage": stage}),
        );
    }

    pub(crate) fn closing(self, pending: [usize; 4], success: bool) {
        if let Some((engine, _)) = super::view()
            && let Some(context) = self.0
        {
            append(
                engine,
                context,
                json!({"event": "client.return", "success": success,
            "jobs": pending[0], "intents": pending[1], "queued": pending[2], "launches": pending[3]}),
            );
        }
    }

    pub(crate) fn service(self, state: Service) {
        let Some((engine, _)) = super::view() else {
            return;
        };
        let Some(context) = self.0 else {
            return;
        };
        let value = match state {
            Service::Listen => json!({"event": "host.listen"}),
            Service::Stop(success) => json!({"event": "host.stop", "success": success}),
        };
        append(engine, context, value);
    }
}
