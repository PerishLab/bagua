use crate::{Failure, Size};
use radicula_process::Environment;
use std::{
    ffi::{OsStr, OsString},
    path::PathBuf,
};

pub struct Launch {
    pub program: PathBuf,
    pub args: Vec<OsString>,
    pub cwd: PathBuf,
    pub environment: Environment,
    pub size: Size,
}

impl Launch {
    pub fn new(program: PathBuf, cwd: PathBuf, environment: Environment, size: Size) -> Self {
        Self {
            program,
            args: Vec::new(),
            cwd,
            environment,
            size,
        }
    }

    pub(crate) fn check(&self) -> Result<(), Failure> {
        if !self.program.is_absolute() || !self.cwd.is_absolute() {
            return Err(Failure::Input(
                "program and cwd must be explicit absolute paths",
            ));
        }
        if self.args.len() > 256 || self.environment.values().len() > 256 {
            return Err(Failure::Input(
                "launch argument or environment count exceeds 256",
            ));
        }
        let paths = [self.program.as_os_str(), self.cwd.as_os_str()];
        let arguments = self.args.iter().map(OsString::as_os_str);
        let environment = self
            .environment
            .values()
            .iter()
            .flat_map(|(key, value)| [key.as_os_str(), value.as_os_str()]);
        let mut total = 0;
        for value in paths.into_iter().chain(arguments).chain(environment) {
            let bytes = value.as_encoded_bytes();
            if bytes.len() > 65_536 || bytes.contains(&0) {
                return Err(Failure::Input(
                    "launch data contains NUL or exceeds its byte bound",
                ));
            }
            total += bytes.len();
        }
        if total > 65_536 {
            return Err(Failure::Input("launch data exceeds 65536 bytes"));
        }
        for (key, _) in self.environment.values() {
            if invalid(key) {
                return Err(Failure::Input(
                    "environment key must be nonempty and contain no equals sign",
                ));
            }
        }
        Ok(())
    }
}

fn invalid(key: &OsStr) -> bool {
    key.is_empty() || key.as_encoded_bytes().contains(&b'=')
}
