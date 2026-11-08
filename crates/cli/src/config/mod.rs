mod check;
mod launch;
mod runtime;
#[cfg(test)]
mod tests;

use crate::{args::Args, error::Error};
use std::{
    ffi::OsString,
    path::{Path, PathBuf},
    time::Duration,
};

pub(crate) fn verify(program: &protocol::wire::Program) -> Result<(), Error> {
    launch::check(program)
}

pub(crate) struct Loaded {
    path: Option<PathBuf>,
    explicit: bool,
    present: bool,
    config: bagua::config::Config,
    keys: client::Keys,
    delay: Duration,
    runtime: Option<PathBuf>,
    #[cfg(all(target_os = "linux", target_arch = "x86_64"))]
    pub(crate) invocation: Option<crate::startup::Invocation>,
}

pub(crate) fn launch(
    launch: &crate::args::Launch,
) -> Result<Option<protocol::wire::Program>, Error> {
    launch::resolve(launch)
}

pub(crate) fn validate(args: &Args) -> Result<(), Error> {
    match args.selected() {
        crate::args::Action::Enter {
            name, launch: data, ..
        } => {
            if name.as_ref().is_some_and(|name| !workspace::valid(name)) {
                return Err(protocol::Failure::Workspace(workspace::Failure::Name).into());
            }
            launch(data)?;
        }
        crate::args::Action::Open { launch: data }
        | crate::args::Action::Connect { launch: data, .. } => {
            launch(data)?;
        }
        _ => {}
    }
    Ok(())
}

pub(crate) fn load(args: &Args, get: &dyn Fn(&str) -> Option<OsString>) -> Result<Loaded, Error> {
    let mut loaded = discover(args, get)?;
    let normal = matches!(
        args.selected(),
        crate::args::Action::Enter { .. }
            | crate::args::Action::Serve { .. }
            | crate::args::Action::Config
    );
    if normal && args.socket.is_none() && loaded.config.host.socket.is_none() {
        loaded.runtime = runtime::path(get)?;
    }
    Ok(loaded)
}

fn discover(args: &Args, get: &dyn Fn(&str) -> Option<OsString>) -> Result<Loaded, Error> {
    if args.bare {
        return Loaded::read(None, false);
    }
    if let Some(path) = &args.config {
        return Loaded::read(Some(std::path::absolute(path)?), true);
    }
    let home = get("XDG_CONFIG_HOME")
        .filter(|value| !value.is_empty())
        .map(PathBuf::from)
        .or_else(|| {
            get("HOME")
                .filter(|value| !value.is_empty())
                .map(|home| PathBuf::from(home).join(".config"))
        });
    let path = home.map(|home| home.join("bagua/bagua.toml"));
    if path.as_ref().is_some_and(|path| !path.is_absolute()) {
        return Err(Error::config(
            "configuration home must be an absolute directory",
        ));
    }
    Loaded::read(path, false)
}

impl Loaded {
    pub fn disabled() -> Self {
        Self::read(None, false).unwrap()
    }

    fn read(path: Option<PathBuf>, explicit: bool) -> Result<Self, Error> {
        let source = match path.as_deref().map(std::fs::metadata).transpose() {
            Ok(Some(metadata)) => {
                if !metadata.is_file() || metadata.len() > 65_536 {
                    return Err(Error::config(
                        "configuration source must be a regular file of at most 65536 bytes",
                    ));
                }
                path.as_deref()
            }
            Ok(None) => None,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound && !explicit => None,
            Err(error) => return Err(Error::config(format!("{}: {error}", display(&path)))),
        };
        let present = source.is_some();
        let config = bagua::config::Config::resolve(source, "BAGUA", &|_| None).map_err(fault)?;
        let (keys, delay) = check::check(&config)
            .map_err(|error| Error::config(format!("{}: {}", display(&path), error.message())))?;
        Ok(Self {
            path,
            explicit,
            present,
            config,
            keys,
            delay,
            runtime: None,
            #[cfg(all(target_os = "linux", target_arch = "x86_64"))]
            invocation: None,
        })
    }

    pub fn apply(&self, args: &mut Args) {
        self.socket(args);
        launch::apply(args, &self.config.program);
    }

    pub fn keys(&self) -> &client::Keys {
        &self.keys
    }

    pub fn delay(&self) -> Duration {
        self.delay
    }

    pub fn history(&self) -> bagua::config::History {
        self.config.history
    }

    pub fn workspace(&self) -> &bagua::config::Workspace {
        &self.config.workspace
    }

    pub fn prefix(&self) -> &str {
        &self.config.client.prefix
    }

    pub fn source(&self) -> Option<&Path> {
        self.present.then_some(self.path.as_deref()).flatten()
    }

    pub fn reload(&mut self) -> Result<(), Error> {
        if self.path.is_none() {
            return Err(Error::config("no configuration source selected"));
        }
        let mut next = Self::read(self.path.clone(), true)?;
        next.runtime = self.runtime.clone();
        #[cfg(all(target_os = "linux", target_arch = "x86_64"))]
        {
            next.invocation = self.invocation.clone();
        }
        next.explicit = self.explicit;
        let mut startup = next.config.clone();
        startup.client = self.config.client.clone();
        if startup != self.config {
            return Err(Error::config(
                "host/program/history/workspace changes require a new invocation; running host and programs were not changed",
            ));
        }
        *self = next;
        Ok(())
    }

    pub fn report(&self, args: &Args) -> serde_json::Value {
        serde_json::json!({
            "version": self.config.version,
            "source": self.path,
            "explicit": self.explicit,
            "loaded": self.present,
            "client": {"prefix": self.prefix(), "escape_ms": self.config.client.escape},
            "host": {"socket": args.socket.as_ref().or(self.runtime.as_ref()), "default": self.runtime.is_some()},
            "history": {"lines": self.config.history.lines, "bytes": self.config.history.bytes, "total": self.config.history.total},
            "workspace": {"name": self.config.workspace.name, "cwd": self.config.workspace.cwd},
            "program": self.config.program.as_ref().map(|program| serde_json::json!({
                "executable": program.executable, "cwd": program.cwd,
                "args": program.args.len(), "environment": program.environment.keys().collect::<Vec<_>>()
            })),
            "interactive_defaults": if self.config.program.is_none() { Some("enter uses invocation SHELL or /bin/sh, invocation cwd and an explicit snapshot of the invocation environment; TERM=xterm-256color") } else { None },
            "reload": ["client.prefix", "client.escape_ms"],
            "restart": ["host.socket", "program", "history", "workspace"],
            "unsupported": ["bindings", "clipboard", "status"],
        })
    }
}

fn display(path: &Option<PathBuf>) -> String {
    path.as_deref()
        .unwrap_or(Path::new("<defaults>"))
        .display()
        .to_string()
}

fn fault(error: radicula_config::Error) -> Error {
    match error {
        radicula_config::Error::Parse { path, source } => Error::config(format!(
            "{}: {} at bytes {:?}",
            path.display(),
            source.message(),
            source.span()
        )),
        other => Error::config(other.to_string()),
    }
}
