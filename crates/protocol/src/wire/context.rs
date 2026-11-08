use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Context {
    pub pid: u32,
    pub session: u32,
    pub started: u64,
    pub command: String,
    pub cwd: String,
}
