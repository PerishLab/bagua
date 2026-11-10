use crate::error::Error;
use signal_hook::{
    SigId,
    consts::{SIGINT, SIGTERM},
};
use std::{
    path::Path,
    sync::{Arc, atomic::AtomicBool},
};

pub(crate) struct Signals(Vec<SigId>);

impl Drop for Signals {
    fn drop(&mut self) {
        for id in self.0.drain(..) {
            signal_hook::low_level::unregister(id);
        }
    }
}

pub(crate) fn listen() -> Result<(Signals, Arc<AtomicBool>), Error> {
    let stop = Arc::new(AtomicBool::new(false));
    let mut signals = Signals(Vec::new());
    for signal in [SIGINT, SIGTERM] {
        signals
            .0
            .push(signal_hook::flag::register(signal, Arc::clone(&stop))?);
    }
    Ok((signals, stop))
}

#[locus::trace(with = crate::observation::view(), frame = context)]
pub(crate) fn run(
    path: &Path,
    history: bagua::config::History,
    identity: protocol::wire::Image,
    orphan: bool,
) -> Result<(), Error> {
    if orphan {
        rustix::process::setsid().map_err(std::io::Error::from)?;
    }
    let (_signals, stop) = listen()?;
    let mut service = bagua::ipc::Service::identified(path, history, Some(identity))?;
    crate::observation::host(&mut service, context.as_ref());
    crate::observation::event::Scope(context.as_ref())
        .service(crate::observation::event::Service::Listen);
    let result = service.run(&stop);
    crate::observation::event::Scope(context.as_ref())
        .service(crate::observation::event::Service::Stop(result.is_ok()));
    result?;
    Ok(())
}
