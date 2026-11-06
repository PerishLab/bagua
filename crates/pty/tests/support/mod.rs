use pty::{Launch, Size};
use radicula_process::Environment;
use std::path::PathBuf;

pub fn shell(script: &str) -> Launch {
    let mut launch = Launch::new(
        PathBuf::from("/bin/sh"),
        PathBuf::from("/"),
        Environment::new([
            ("PATH".into(), "/usr/bin:/bin".into()),
            ("TERM".into(), "dumb".into()),
        ]),
        Size::new(80, 24).unwrap(),
    );
    launch.args = vec!["-c".into(), script.into()];
    launch
}
