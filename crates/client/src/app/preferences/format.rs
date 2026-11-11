use super::{Header, Status};
use crate::template::Template;
use protocol::Failure;
use workspace::Edge;

pub const HEADER: &str = " W{{window.index}}:{{window.title:24}} · P{{pane.index}}{{#if pane.command}} {{pane.command}}{{/if}}{{#if pane.cwd_name}} · {{pane.cwd_name}}{{/if}} ";
pub const WINDOWS: &str = "{{#if client.columns >= 64}} {{session.name:18}} {{/if}}{{#windows}}{{#if window.active}}{{style active}}{{else}}{{style inactive}}{{/if}} {{window.index}}:{{window.title:24}} {{style base}}{{/windows}}";

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Layout {
    pub lines: Vec<Line>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Line {
    pub left: Template,
    pub center: Template,
    pub right: Template,
    pub fill: bool,
}

impl Line {
    pub fn new(left: &str, center: &str, right: &str) -> Result<Self, Failure> {
        Ok(Self {
            left: Template::compile(left)?,
            center: Template::compile(center)?,
            right: Template::compile(right)?,
            fill: true,
        })
    }
    pub fn metadata(&self) -> bool {
        self.left.metadata() || self.center.metadata() || self.right.metadata()
    }
}

impl Header {
    pub fn format(source: Option<&str>) -> Result<Self, Failure> {
        let mut header = Self::new(&[])?;
        header.template = Some(Template::compile(source.unwrap_or(HEADER))?);
        header.enabled = true;
        header.border = true;
        header.active = 25;
        header.inactive = 237;
        header.foreground = Some(248);
        header.highlight = Some(231);
        Ok(header)
    }
}

impl Status {
    pub fn format(lines: Option<Vec<Line>>, timezone: &str, refresh: u64) -> Result<Self, Failure> {
        let lines = match lines {
            Some(lines) => lines,
            None => {
                let mut blank = Line::new("", "", "")?;
                blank.fill = false;
                vec![
                    blank,
                    Line::new(
                        WINDOWS,
                        "",
                        "{{#if client.columns >= 64}}{{clock.time}}  {{clock.date}} {{/if}}",
                    )?,
                ]
            }
        };
        if !(1..=4).contains(&lines.len()) {
            return Err(Failure::Capacity);
        }
        let mut status = Self::new(&[], timezone, refresh)?;
        status.layout = Some(Layout { lines });
        status.active = 39;
        status.inactive = 234;
        status.foreground = Some(245);
        status.highlight = Some(16);
        Ok(status)
    }

    pub fn rows(&self) -> u16 {
        self.layout
            .as_ref()
            .map_or(1, |layout| layout.lines.len() as u16)
            + u16::from(self.separator)
    }

    pub fn inset(&self) -> u16 {
        if self.edge == Edge::Top {
            self.rows()
        } else {
            0
        }
    }
}
