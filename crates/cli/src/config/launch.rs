use crate::{
    args::{Action, Args},
    error::Error,
};
use protocol::wire::Program;
use std::path::Path;

pub(super) fn apply(args: &mut Args, program: &Option<Program>) {
    let launch = match &mut args.action {
        Action::Open { launch } | Action::Connect { launch, .. } => launch,
        _ => return,
    };
    if launch.executable.is_some() {
        return;
    }
    if let Some(program) = program {
        launch.executable = Some(program.executable.clone());
        launch.cwd = Some(program.cwd.clone());
        launch.args = program.args.clone();
        launch.environment = program
            .environment
            .iter()
            .map(|(key, value)| format!("{key}={value}"))
            .collect();
    }
}

pub(super) fn check(program: &Program) -> Result<(), Error> {
    if !Path::new(&program.executable).is_absolute() || !Path::new(&program.cwd).is_absolute() {
        return Err(Error::config(
            "program.executable/program.cwd: expected absolute paths",
        ));
    }
    if program.args.len() > 256 || program.environment.len() > 256 {
        return Err(Error::config(
            "program: argument/environment count exceeds 256",
        ));
    }
    let paths = [&program.executable, &program.cwd];
    let values = paths.into_iter().chain(&program.args).chain(
        program
            .environment
            .iter()
            .flat_map(|(key, value)| [key, value]),
    );
    let mut bytes = 0;
    for value in values {
        if value.contains('\0') {
            return Err(Error::config("program: launch data contains NUL"));
        }
        bytes += value.len();
    }
    if bytes > 65_536
        || program
            .environment
            .keys()
            .any(|key| key.is_empty() || key.contains('='))
    {
        return Err(Error::config(
            "program: invalid environment key or launch data exceeds 65536 bytes",
        ));
    }
    Ok(())
}
