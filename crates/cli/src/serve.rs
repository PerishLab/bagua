use crate::error::Error;
use std::{
    path::Path,
    sync::{Arc, atomic::AtomicBool},
};

#[cfg(all(target_os = "linux", target_arch = "x86_64"))]
use signal_hook::{
    SigId,
    consts::{SIGINT, SIGTERM},
};

#[cfg(all(target_os = "linux", target_arch = "x86_64"))]
pub(crate) struct Signals(Vec<SigId>);

#[cfg(all(target_os = "linux", target_arch = "x86_64"))]
impl Drop for Signals {
    fn drop(&mut self) {
        for id in self.0.drain(..) {
            signal_hook::low_level::unregister(id);
        }
    }
}

#[cfg(all(target_os = "linux", target_arch = "x86_64"))]
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

#[cfg(all(target_os = "windows", target_arch = "x86_64"))]
mod ctrl {
    use std::{
        io,
        sync::{
            Arc, Mutex, OnceLock,
            atomic::{AtomicBool, AtomicUsize, Ordering},
        },
    };
    use windows_sys::Win32::System::Console::{
        CTRL_BREAK_EVENT, CTRL_C_EVENT, CTRL_CLOSE_EVENT, SetConsoleCtrlHandler,
    };

    type Entry = (usize, Arc<AtomicBool>);

    static FLAGS: OnceLock<Mutex<Vec<Entry>>> = OnceLock::new();
    static NEXT: AtomicUsize = AtomicUsize::new(1);

    pub(crate) struct Signals(usize);

    impl Drop for Signals {
        fn drop(&mut self) {
            let mut flags = table().lock().unwrap();
            flags.retain(|(id, _)| *id != self.0);
            if flags.is_empty() {
                unsafe {
                    SetConsoleCtrlHandler(Some(handler), 0);
                }
            }
        }
    }

    pub(super) fn listen() -> io::Result<(Signals, Arc<AtomicBool>)> {
        let stop = Arc::new(AtomicBool::new(false));
        let id = NEXT.fetch_add(1, Ordering::Relaxed);
        let mut flags = table().lock().unwrap();
        let first = flags.is_empty();
        flags.push((id, Arc::clone(&stop)));
        if first && unsafe { SetConsoleCtrlHandler(Some(handler), 1) } == 0 {
            flags.pop();
            return Err(io::Error::last_os_error());
        }
        Ok((Signals(id), stop))
    }

    fn table() -> &'static Mutex<Vec<Entry>> {
        FLAGS.get_or_init(|| Mutex::new(Vec::new()))
    }

    unsafe extern "system" fn handler(event: u32) -> i32 {
        if !matches!(event, CTRL_C_EVENT | CTRL_BREAK_EVENT | CTRL_CLOSE_EVENT) {
            return 0;
        }
        if let Ok(flags) = table().lock() {
            for (_, stop) in flags.iter() {
                stop.store(true, Ordering::Relaxed);
            }
        }
        1
    }
}

#[cfg(all(target_os = "windows", target_arch = "x86_64"))]
pub(crate) use ctrl::Signals;

#[cfg(all(target_os = "windows", target_arch = "x86_64"))]
pub(crate) fn listen() -> Result<(Signals, Arc<AtomicBool>), Error> {
    Ok(ctrl::listen()?)
}

#[locus::trace(with = crate::observation::view(), frame = context)]
pub(crate) fn run(
    path: &Path,
    history: bagua::config::History,
    identity: protocol::wire::Image,
    orphan: bool,
) -> Result<(), Error> {
    #[cfg(all(target_os = "linux", target_arch = "x86_64"))]
    if orphan {
        rustix::process::setsid().map_err(std::io::Error::from)?;
    }
    #[cfg(all(target_os = "windows", target_arch = "x86_64"))]
    let _ = orphan;
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
