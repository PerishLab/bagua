use protocol::wire::Program;
use radicula_config::Cascade;
use serde::Deserialize;
use std::path::PathBuf;

#[derive(Clone, Debug, PartialEq, Cascade)]
#[cascade(strict)]
pub struct Config {
    pub version: u16,
    #[cascade(section)]
    pub client: Client,
    #[cascade(section)]
    pub host: Host,
    #[cascade(section)]
    pub history: History,
    #[cascade(section)]
    pub workspace: Workspace,
    pub program: Option<Program>,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            version: 1,
            client: Client::default(),
            host: Host::default(),
            history: History::default(),
            workspace: Workspace::default(),
            program: None,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Deserialize, Cascade)]
#[cascade(section, strict)]
#[serde(default, deny_unknown_fields)]
pub struct Client {
    pub prefix: String,
    #[serde(rename = "escape_ms")]
    #[cascade(name = "escape_ms")]
    pub escape: u64,
}

impl Default for Client {
    fn default() -> Self {
        Self {
            prefix: "C-g".into(),
            escape: 50,
        }
    }
}

#[derive(Clone, Debug, Default, PartialEq, Deserialize, Cascade)]
#[cascade(section, strict)]
#[serde(default, deny_unknown_fields)]
pub struct Host {
    pub socket: Option<PathBuf>,
}

#[derive(Clone, Debug, PartialEq, Deserialize, Cascade)]
#[cascade(section, strict)]
#[serde(default, deny_unknown_fields)]
pub struct Workspace {
    pub name: String,
    pub cwd: String,
}

impl Default for Workspace {
    fn default() -> Self {
        Self {
            name: "workspace".into(),
            cwd: "foreground".into(),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Deserialize, Cascade)]
#[cascade(section, strict)]
#[serde(default, deny_unknown_fields)]
pub struct History {
    pub lines: usize,
    pub bytes: usize,
    pub total: usize,
}

impl Default for History {
    fn default() -> Self {
        Self {
            lines: terminal::HISTORY,
            bytes: terminal::STORAGE,
            total: 256 * 1024 * 1024,
        }
    }
}

impl History {
    pub fn check(self) -> Result<(), protocol::Failure> {
        terminal::History::new(self.lines, self.bytes).map_err(|_| protocol::Failure::Capacity)?;
        if self.total > 256 * 1024 * 1024 || self.total < self.bytes {
            return Err(protocol::Failure::Capacity);
        }
        Ok(())
    }
}
