use super::{Loaded, display};
use crate::{
    args::{Action, Args},
    error::Error,
};
use std::{collections::BTreeMap, ffi::OsString, path::PathBuf};

#[cfg(not(windows))]
pub(super) fn path(get: &dyn Fn(&str) -> Option<OsString>) -> Result<Option<PathBuf>, Error> {
    for key in ["XDG_RUNTIME_DIR", "XDG_STATE_HOME", "HOME"] {
        let Some(value) = get(key).filter(|value| !value.is_empty()) else {
            continue;
        };
        let mut root = PathBuf::from(value);
        if !root.is_absolute() {
            return Err(Error::config(format!(
                "{key}: default host directory must be absolute"
            )));
        }
        if key == "HOME" {
            root = root.join(".local/state");
        }
        return Ok(Some(root.join("bagua/host.sock")));
    }
    Ok(None)
}

#[cfg(windows)]
pub(super) fn path(get: &dyn Fn(&str) -> Option<OsString>) -> Result<Option<PathBuf>, Error> {
    for key in ["LOCALAPPDATA", "USERPROFILE"] {
        let Some(value) = get(key).filter(|value| !value.is_empty()) else {
            continue;
        };
        let mut root = PathBuf::from(value);
        if !root.is_absolute() {
            return Err(Error::config(format!(
                "{key}: default host directory must be absolute"
            )));
        }
        if key == "USERPROFILE" {
            root = root.join("AppData").join("Local");
        }
        return Ok(Some(root.join("bagua").join("runtime").join("host.sock")));
    }
    Ok(None)
}

#[cfg(not(windows))]
pub(super) fn shell(environment: &BTreeMap<String, String>) -> (String, Vec<String>) {
    let executable = environment
        .get("SHELL")
        .cloned()
        .unwrap_or_else(|| "/bin/sh".into());
    (executable, vec!["-i".into()])
}

#[cfg(windows)]
pub(super) fn shell(environment: &BTreeMap<String, String>) -> (String, Vec<String>) {
    let executable = lookup(environment).unwrap_or_else(|| fallback(environment));
    (executable, vec!["-NoLogo".into()])
}

#[cfg(windows)]
fn lookup(values: &BTreeMap<String, String>) -> Option<String> {
    let paths = values
        .iter()
        .find(|(key, _)| key.eq_ignore_ascii_case("PATH"))
        .map(|(_, value)| value)?;
    for entry in paths.split(';') {
        let trimmed = entry.trim().trim_matches('"');
        if trimmed.is_empty() {
            continue;
        }
        let candidate = PathBuf::from(trimmed).join("pwsh.exe");
        if candidate.is_file() {
            let resolved = std::path::absolute(&candidate).unwrap_or(candidate);
            if let Ok(text) = strip(resolved).into_os_string().into_string() {
                return Some(text);
            }
        }
    }
    None
}

#[cfg(windows)]
fn fallback(values: &BTreeMap<String, String>) -> String {
    let root = values
        .iter()
        .find(|(key, _)| key.eq_ignore_ascii_case("SYSTEMROOT"))
        .or_else(|| {
            values
                .iter()
                .find(|(key, _)| key.eq_ignore_ascii_case("WINDIR"))
        })
        .map(|(_, value)| value.as_str())
        .unwrap_or(r"C:\Windows");
    PathBuf::from(root)
        .join("System32")
        .join("WindowsPowerShell")
        .join("v1.0")
        .join("powershell.exe")
        .to_string_lossy()
        .into_owned()
}

pub(super) fn strip(path: PathBuf) -> PathBuf {
    #[cfg(windows)]
    {
        if let Some(text) = path.to_str()
            && let Some(rest) = text.strip_prefix(r"\\?\")
            && !rest.starts_with(r"UNC\")
            && rest.as_bytes().get(1) == Some(&b':')
        {
            return PathBuf::from(rest);
        }
    }
    path
}

impl Loaded {
    pub(super) fn socket(&self, args: &mut Args) {
        if args.socket.is_some() {
            return;
        }
        args.socket = self.config.host.socket.clone();
        if args.socket.is_none()
            && matches!(args.selected(), Action::Enter { .. } | Action::Serve { .. })
        {
            args.socket = self.runtime.clone();
        }
    }

    pub fn runtime(&self) -> Option<&std::path::Path> {
        self.runtime.as_deref()
    }

    pub fn defaults(&self, args: &mut Args, program: protocol::wire::Program) -> Result<(), Error> {
        super::launch::check(&program).map_err(|error| {
            Error::config(format!(
                "{}: interactive default: {}",
                display(&self.path),
                error.message()
            ))
        })?;
        super::launch::apply(args, &Some(program));
        Ok(())
    }
}
