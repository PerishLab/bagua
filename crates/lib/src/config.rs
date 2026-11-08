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
    pub program: Option<Program>,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            version: 1,
            client: Client::default(),
            host: Host::default(),
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
