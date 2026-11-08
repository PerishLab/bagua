mod check;
mod launch;
#[cfg(test)]
mod tests;

use crate::{args::Args, error::Error};
use std::{
    ffi::OsString,
    path::{Path, PathBuf},
    time::Duration,
};

pub(crate) struct Loaded {
    path: Option<PathBuf>,
    explicit: bool,
    present: bool,
    config: bagua::config::Config,
    keys: client::Keys,
    delay: Duration,
}

pub(crate) fn load(args: &Args, get: &dyn Fn(&str) -> Option<OsString>) -> Result<Loaded, Error> {
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
        })
    }

    pub fn apply(&self, args: &mut Args) {
        if args.socket.is_none() {
            args.socket = self.config.host.socket.clone();
        }
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

    pub fn prefix(&self) -> &str {
        &self.config.client.prefix
    }

    pub fn reload(&mut self) -> Result<(), Error> {
        if self.path.is_none() {
            return Err(Error::config("no configuration source selected"));
        }
        let mut next = Self::read(self.path.clone(), true)?;
        next.explicit = self.explicit;
        if next.config.host != self.config.host
            || next.config.program != self.config.program
            || next.config.history != self.config.history
        {
            return Err(Error::config(
                "host/program/history changes require a new invocation; running host and programs were not changed",
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
            "host": {"socket": args.socket},
            "history": {"lines": self.config.history.lines, "bytes": self.config.history.bytes, "total": self.config.history.total},
            "program": self.config.program.as_ref().map(|program| serde_json::json!({
                "executable": program.executable, "cwd": program.cwd,
                "args": program.args.len(), "environment": program.environment.keys().collect::<Vec<_>>()
            })),
            "reload": ["client.prefix", "client.escape_ms"],
            "restart": ["host.socket", "program", "history"],
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
