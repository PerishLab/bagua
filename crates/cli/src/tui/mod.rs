mod queue;
mod runner;
mod setup;

use crate::{args::Args, error::Error};
use std::path::Path;

pub(crate) fn run(path: &Path, args: &Args, config: crate::config::Loaded) -> Result<(), Error> {
    if args.request == 0 {
        return Err(protocol::Failure::Identity.into());
    }
    if args.json || args.control.is_some() || args.session.is_some() {
        return Err(protocol::Failure::Payload.into());
    }
    let (_signals, stop) = crate::serve::listen()?;
    let mut terminal = client::Terminal::enter()?;
    let connection = if matches!(args.selected(), crate::args::Action::Enter { .. }) {
        crate::startup::connect(path, args, &config)
    } else {
        crate::session::Session::connect(path, args)
    };
    let result = connection
        .and_then(|rpc| setup::prepare(rpc, args, &terminal, &config))
        .and_then(|runner| runner.configure(config).run(&mut terminal, &stop));
    let restored = terminal.finish();
    match (result, restored) {
        (Err(error), Err(cleanup)) => {
            eprintln!("bagua terminal restoration failed: {cleanup}");
            Err(error)
        }
        (Err(error), _) => Err(error),
        (_, Err(error)) => Err(error.into()),
        _ => Ok(()),
    }
}

pub(super) fn extent(columns: u16, rows: u16) -> Option<terminal::Extent> {
    if columns < 2 || rows < 2 {
        return None;
    }
    terminal::Extent::new(columns.min(512), (rows - 1).min(255))
}
