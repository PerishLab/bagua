use radicula_config::Cascade;
use serde::Deserialize;

#[derive(Clone, Debug, PartialEq, Deserialize, Cascade)]
#[cascade(section, strict)]
#[serde(default, deny_unknown_fields)]
pub struct Header {
    pub enabled: bool,
    pub format: Option<String>,
    pub position: String,
    pub border: Option<bool>,
    pub foreground: Option<u8>,
    pub highlight: Option<u8>,
    pub components: Option<Vec<String>>,
    pub color: bool,
    pub active: Option<u8>,
    pub inactive: Option<u8>,
}

impl Default for Header {
    fn default() -> Self {
        Self {
            enabled: true,
            format: None,
            position: "top".into(),
            border: None,
            foreground: None,
            highlight: None,
            components: None,
            color: true,
            active: None,
            inactive: None,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Deserialize, Cascade)]
#[cascade(section, strict)]
#[serde(default, deny_unknown_fields)]
pub struct Status {
    pub position: String,
    pub lines: Option<Vec<Line>>,
    pub foreground: Option<u8>,
    pub highlight: Option<u8>,
    pub components: Option<Vec<String>>,
    pub color: bool,
    pub active: Option<u8>,
    pub inactive: Option<u8>,
    pub separator: bool,
    pub timezone: String,
    #[serde(rename = "refresh_seconds")]
    #[cascade(name = "refresh_seconds")]
    pub refresh: u64,
}

impl Default for Status {
    fn default() -> Self {
        Self {
            position: "bottom".into(),
            lines: None,
            foreground: None,
            highlight: None,
            components: None,
            color: true,
            active: None,
            inactive: None,
            separator: false,
            timezone: "UTC".into(),
            refresh: 15,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Line {
    pub left: String,
    pub center: String,
    pub right: String,
    pub fill: bool,
}

impl Default for Line {
    fn default() -> Self {
        Self {
            left: String::new(),
            center: String::new(),
            right: String::new(),
            fill: true,
        }
    }
}
