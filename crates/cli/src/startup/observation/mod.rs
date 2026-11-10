pub(crate) mod event;
mod hook;
#[cfg(test)]
mod tests;

use hook::Code;
pub(crate) use hook::Notes;
use plumb::observation::{Declaration, Run, Settings};
use std::{
    ffi::OsString,
    path::PathBuf,
    sync::{Arc, OnceLock},
};

static CONTEXT: OnceLock<(&'static locus::Engine, locus::Context)> = OnceLock::new();

pub(crate) struct Inputs {
    pub state: Option<PathBuf>,
    pub endpoint: Option<OsString>,
    pub parent: Option<OsString>,
}

pub(crate) struct Observer {
    pub notes: Arc<Notes>,
    run: Option<Run>,
}

impl Observer {
    pub fn start(config: &crate::config::Loaded, command: &str, inputs: Inputs) -> Self {
        let mut observer = Self {
            notes: Arc::new(Notes::default()),
            run: None,
        };
        match observer.open(config, command, inputs) {
            Ok(run) => observer.install(run),
            Err(code) => observer.notes.mark(code),
        }
        observer
    }

    fn install(&mut self, run: Option<Run>) {
        if let Some(run) = &run {
            let (engine, base) = run.view();
            if identity(engine, base).is_err() {
                self.notes.mark(Code::Setup);
            }
        }
        self.run = run;
    }

    fn open(
        &self,
        config: &crate::config::Loaded,
        command: &str,
        inputs: Inputs,
    ) -> Result<Option<Run>, Code> {
        let mut settings = config
            .locus()
            .map(|value| serde_json::from_value::<Settings>(value.clone()))
            .transpose()
            .map_err(|_| Code::Config)?
            .unwrap_or_default();
        if !settings.enabled {
            return Ok(None);
        }
        if let Some(parent) = inputs.parent {
            locus::Key::new(parent.into_string().map_err(|_| Code::Parent)?)
                .map_err(|_| Code::Parent)?;
        }
        if settings.endpoint.is_none() {
            settings.endpoint = inputs
                .endpoint
                .map(OsString::into_string)
                .transpose()
                .map_err(|_| Code::Endpoint)?;
        }
        let buffer = buffer(inputs.state)?;
        let file = config
            .source()
            .unwrap_or(std::path::Path::new("bagua.toml"));
        Run::configured(
            Declaration {
                producer: "bagua",
                file,
                buffer,
            },
            command,
            settings,
            self.notes.clone(),
        )
        .map_err(|_| Code::Setup)
    }

    pub fn finish(self, code: i32, json: bool) {
        if let Some(run) = self.run {
            if code != 0 {
                run.fault("command.failed");
            }
            run.finish(code);
        }
        self.notes.flush(json);
    }
}

fn buffer(state: Option<PathBuf>) -> Result<PathBuf, Code> {
    use std::os::unix::fs::DirBuilderExt;
    let state = state.filter(|path| path.is_absolute()).ok_or(Code::State)?;
    let buffer = state.join("bagua/locus");
    std::fs::DirBuilder::new()
        .recursive(true)
        .mode(0o700)
        .create(&buffer)
        .map_err(|_| Code::State)?;
    if !bagua::ipc::private(&buffer).map_err(|_| Code::State)? {
        return Err(Code::State);
    }
    Ok(buffer)
}

pub(crate) fn view() -> Option<(&'static locus::Engine, &'static locus::Context)> {
    plumb::observation::view().map(|(engine, base)| {
        let context = CONTEXT
            .get()
            .filter(|(owner, _)| std::ptr::eq(*owner, engine))
            .map(|(_, context)| context)
            .unwrap_or(base);
        (engine, context)
    })
}

pub(crate) fn parent(command: &mut std::process::Command, frame: Option<&locus::Context>) {
    if let Some(frame) = frame {
        plumb::observation::parent(command, frame);
    }
}

pub(crate) fn host(service: &mut bagua::ipc::Service, frame: Option<&locus::Context>) {
    if let Some((engine, root)) = view() {
        let context = frame.unwrap_or(root).clone();
        service.events(move |event| event::native(engine, &context, event));
    }
}

fn identity(engine: &'static locus::Engine, base: &locus::Context) -> Result<(), locus::Error> {
    let identity = plumb::identity::Reader("BAGUA");
    let version = identity
        .binding()
        .map(|binding| binding.marker.as_str())
        .unwrap_or("unknown");
    let candidate = locus::Candidate::context()
        .explicit(
            locus::Role::new("bagua.version")?,
            locus::Key::new(version)?,
        )
        .explicit(
            locus::Role::new("bagua.source")?,
            locus::Key::new(identity.commit().unwrap_or("unknown"))?,
        );
    let context = locus::record!(engine, base, candidate)?.context();
    let _ = CONTEXT.set((engine, context));
    Ok(())
}
