use crate::{
    args::{Action, Args},
    error::Error,
};
use protocol::Failure;

pub(crate) fn execute(args: &Args, config: crate::config::Loaded) -> Result<(), Error> {
    let command = if matches!(
        args.selected(),
        Action::Open { .. }
            | Action::Connect { .. }
            | Action::Enter { .. }
            | Action::Serve { .. }
            | Action::Control
    ) {
        None
    } else {
        Some(crate::command::parse(args.selected())?)
    };
    if args.request == 0 {
        return Err(Failure::Identity.into());
    }
    if matches!(args.selected(), Action::Control)
        && (args.control.is_some() || args.session.is_some())
    {
        return Err(Failure::Payload.into());
    }
    let path = args.socket.as_deref().ok_or(Failure::Unavailable)?;
    #[cfg(any(
        all(target_os = "linux", target_arch = "x86_64"),
        all(target_os = "windows", target_arch = "x86_64")
    ))]
    {
        if matches!(
            args.selected(),
            Action::Open { .. } | Action::Connect { .. } | Action::Enter { .. }
        ) {
            return crate::tui::run(path, args, config);
        }
        if let Action::Serve { orphan } = args.selected() {
            let path = crate::startup::location(path, config.runtime().is_some(), true)?;
            let identity = config
                .invocation
                .as_ref()
                .ok_or(Failure::Unavailable)?
                .image;
            return crate::serve::run(&path, config.history(), identity, *orphan);
        }
        let resolved = crate::startup::location(path, config.runtime().is_some(), false)?;
        let mut remote = crate::session::Session::connect(&resolved, args)?;
        let Some(command) = command else {
            return remote.control();
        };
        if let Some(session) = args.control.or(args.session) {
            remote.join(session, args.control.is_some())?;
        }
        let watch = matches!(command, protocol::Command::Subscribe { .. });
        if matches!(args.selected(), Action::Inspect { text: true, .. }) {
            remote.plain(command)?;
        } else {
            remote.call(command, true)?;
        }
        if watch {
            remote.watch()?;
        }
        Ok(())
    }
    #[cfg(not(any(
        all(target_os = "linux", target_arch = "x86_64"),
        all(target_os = "windows", target_arch = "x86_64")
    )))]
    {
        let _ = (path, command);
        Err(Failure::Unavailable.into())
    }
}
