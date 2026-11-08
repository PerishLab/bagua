use crate::{
    args::{Action, Args},
    error::Error,
};
use protocol::Failure;

pub(crate) fn execute(args: &Args, config: crate::config::Loaded) -> Result<(), Error> {
    let path = args.socket.as_deref().ok_or(Failure::Unavailable)?;
    #[cfg(all(target_os = "linux", target_arch = "x86_64"))]
    {
        if matches!(args.action, Action::Open { .. } | Action::Connect { .. }) {
            return crate::tui::run(path, args, config);
        }
        if matches!(args.action, Action::Serve) {
            return crate::serve::run(path, config.history());
        }
        let mut remote = crate::session::Session::connect(path, args)?;
        if matches!(args.action, Action::Control) {
            if args.control.is_some() || args.session.is_some() {
                return Err(Failure::Payload.into());
            }
            return remote.control();
        }
        if let Some(session) = args.control.or(args.session) {
            remote.join(session, args.control.is_some())?;
        }
        let command = crate::command::parse(&args.action)?;
        let watch = matches!(command, protocol::Command::Subscribe { .. });
        remote.call(command, true)?;
        if watch {
            remote.watch()?;
        }
        Ok(())
    }
    #[cfg(not(all(target_os = "linux", target_arch = "x86_64")))]
    {
        let _ = path;
        Err(Failure::Unavailable.into())
    }
}
