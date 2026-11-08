use protocol::Failure;
use std::time::Duration;

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum Component {
    Pane,
    Command,
    Cwd,
    Session,
    Windows,
    Prefix,
    Notice,
    Clock,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Decor {
    pub header: Header,
    pub status: Status,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Header {
    pub enabled: bool,
    pub color: bool,
    pub active: u8,
    pub inactive: u8,
    pub(crate) components: Vec<Component>,
}

impl Header {
    pub fn new(values: &[String]) -> Result<Self, Failure> {
        Ok(Self {
            components: components(
                values,
                &[Component::Pane, Component::Command, Component::Cwd],
            )?,
            enabled: false,
            color: true,
            active: 4,
            inactive: 8,
        })
    }
}

impl Default for Header {
    fn default() -> Self {
        Self::new(&["pane".into(), "command".into(), "cwd".into()]).unwrap()
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Status {
    pub color: bool,
    pub active: u8,
    pub inactive: u8,
    pub separator: bool,
    pub(crate) components: Vec<Component>,
    pub(crate) offset: u64,
    pub(crate) refresh: Duration,
}

impl Status {
    pub fn new(values: &[String], timezone: &str, refresh: u64) -> Result<Self, Failure> {
        if !(1..=3600).contains(&refresh) {
            return Err(Failure::Capacity);
        }
        let offset = match timezone {
            "UTC" => 0,
            "Asia/Shanghai" => 480,
            _ => return Err(Failure::Payload),
        };
        let components = components(
            values,
            &[
                Component::Session,
                Component::Windows,
                Component::Prefix,
                Component::Notice,
                Component::Clock,
            ],
        )?;
        Ok(Self {
            color: true,
            active: 4,
            inactive: 8,
            separator: false,
            components,
            offset,
            refresh: Duration::from_secs(refresh),
        })
    }
    pub fn refresh(&self) -> Duration {
        self.refresh
    }
}

impl Default for Status {
    fn default() -> Self {
        Self::new(
            &[
                "session".into(),
                "windows".into(),
                "prefix".into(),
                "notice".into(),
                "clock".into(),
            ],
            "UTC",
            15,
        )
        .unwrap()
    }
}

fn components(values: &[String], allowed: &[Component]) -> Result<Vec<Component>, Failure> {
    if values.len() > 8 {
        return Err(Failure::Capacity);
    }
    let mut result = Vec::new();
    for value in values {
        let component = match value.as_str() {
            "pane" => Component::Pane,
            "command" => Component::Command,
            "cwd" => Component::Cwd,
            "session" => Component::Session,
            "windows" => Component::Windows,
            "prefix" => Component::Prefix,
            "notice" => Component::Notice,
            "clock" => Component::Clock,
            _ => return Err(Failure::Payload),
        };
        if !allowed.contains(&component) || result.contains(&component) {
            return Err(Failure::Payload);
        }
        result.push(component);
    }
    Ok(result)
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Mouse {
    pub enabled: bool,
    pub resize: bool,
    pub release: bool,
}

impl Default for Mouse {
    fn default() -> Self {
        Self {
            enabled: true,
            resize: true,
            release: true,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Clipboard {
    pub(crate) outer: bool,
    pub(crate) limit: usize,
}

impl Clipboard {
    pub fn new(target: &str, limit: usize) -> Result<Self, Failure> {
        if !(1..=1_048_576).contains(&limit) {
            return Err(Failure::Capacity);
        }
        let outer = match target {
            "local" => false,
            "osc52" => true,
            _ => return Err(Failure::Payload),
        };
        Ok(Self { outer, limit })
    }
}

impl Default for Clipboard {
    fn default() -> Self {
        Self {
            outer: false,
            limit: 262_144,
        }
    }
}
