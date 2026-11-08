use radicula_config::Cascade;
use serde::Deserialize;

#[derive(Clone, Debug, PartialEq, Deserialize, Cascade)]
#[cascade(section, strict)]
#[serde(default, deny_unknown_fields)]
pub struct Header {
    pub enabled: bool,
    pub components: Vec<String>,
    pub color: bool,
    pub active: u8,
    pub inactive: u8,
}

impl Default for Header {
    fn default() -> Self {
        Self {
            enabled: false,
            components: vec!["pane".into(), "command".into(), "cwd".into()],
            color: true,
            active: 4,
            inactive: 8,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Deserialize, Cascade)]
#[cascade(section, strict)]
#[serde(default, deny_unknown_fields)]
pub struct Status {
    pub components: Vec<String>,
    pub color: bool,
    pub active: u8,
    pub inactive: u8,
    pub separator: bool,
    pub timezone: String,
    #[serde(rename = "refresh_seconds")]
    #[cascade(name = "refresh_seconds")]
    pub refresh: u64,
}

impl Default for Status {
    fn default() -> Self {
        Self {
            components: vec![
                "session".into(),
                "windows".into(),
                "prefix".into(),
                "notice".into(),
                "clock".into(),
            ],
            color: true,
            active: 4,
            inactive: 8,
            separator: false,
            timezone: "UTC".into(),
            refresh: 15,
        }
    }
}
