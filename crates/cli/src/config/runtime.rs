use super::{Loaded, display};
use crate::{
    args::{Action, Args},
    error::Error,
};
use std::{ffi::OsString, path::PathBuf};

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
