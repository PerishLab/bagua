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

pub(crate) fn run(path: &Path, history: bagua::config::History) -> Result<(), Error> {
    let (_signals, stop) = listen()?;
    let mut service = bagua::ipc::Service::configured(path, history)?;
    service.run(&stop)?;
    Ok(())
}
