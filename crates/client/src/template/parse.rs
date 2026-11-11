use super::{Condition, Field, Node, Tone};
use protocol::Failure;

pub(super) fn compile(source: &str) -> Result<Vec<Node>, Failure> {
    if source.len() > 4096 || source.chars().any(char::is_control) {
        return Err(Failure::Capacity);
    }
    let mut parser = Parser {
        rest: source,
        count: 0,
    };
    let (nodes, stop) = parser.nodes(0, false)?;
    if stop != Stop::Eof {
        return Err(Failure::Payload);
    }
    Ok(nodes)
}

#[derive(PartialEq)]
enum Stop {
    Eof,
    Else,
    If,
    Windows,
}

struct Parser<'a> {
    rest: &'a str,
    count: usize,
}

impl Parser<'_> {
    fn nodes(&mut self, depth: usize, listed: bool) -> Result<(Vec<Node>, Stop), Failure> {
        if depth > 8 {
            return Err(Failure::Capacity);
        }
        let mut nodes = Vec::new();
        while !self.rest.is_empty() {
            self.count += 1;
            if self.count > 128 {
                return Err(Failure::Capacity);
            }
            if let Some(text) = self.text() {
                nodes.push(Node::Text(text));
                continue;
            }
            let tag = self.tag()?.to_owned();
            match tag.as_str() {
                "else" => return Ok((nodes, Stop::Else)),
                "/if" => return Ok((nodes, Stop::If)),
                "/windows" => return Ok((nodes, Stop::Windows)),
                _ => nodes.push(self.node(&tag, depth, listed)?),
            }
        }
        Ok((nodes, Stop::Eof))
    }

    fn text(&mut self) -> Option<String> {
        if self.rest.starts_with("{{{{") {
            self.rest = &self.rest[4..];
            return Some("{{".into());
        }
        let end = self.rest.find("{{").unwrap_or(self.rest.len());
        if end == 0 {
            return None;
        }
        let text = self.rest[..end].to_owned();
        self.rest = &self.rest[end..];
        Some(text)
    }

    fn tag(&mut self) -> Result<&str, Failure> {
        let end = self.rest.find("}}").ok_or(Failure::Payload)?;
        let text = self.rest[2..end].trim();
        self.rest = &self.rest[end + 2..];
        Ok(text)
    }

    fn node(&mut self, tag: &str, depth: usize, listed: bool) -> Result<Node, Failure> {
        if let Some(field) = tag.strip_prefix("#if ") {
            return self.branch(Condition::parse(field)?, depth, listed);
        }
        if tag == "#windows" {
            if listed {
                return Err(Failure::Payload);
            }
            let (nodes, stop) = self.nodes(depth + 1, true)?;
            return if stop == Stop::Windows {
                Ok(Node::Windows(nodes))
            } else {
                Err(Failure::Payload)
            };
        }
        if let Some(style) = tag.strip_prefix("style ") {
            return Ok(Node::Style(match style {
                "base" => Tone::Base,
                "active" => Tone::Active,
                "inactive" => Tone::Inactive,
                _ => return Err(Failure::Payload),
            }));
        }
        let (field, width) = match tag.split_once(':') {
            Some((field, width)) => {
                let width = width.parse::<usize>().map_err(|_| Failure::Payload)?;
                if width > 512 {
                    return Err(Failure::Capacity);
                }
                (field, Some(width))
            }
            None => (tag, None),
        };
        Ok(Node::Field(Field::parse(field)?, width))
    }

    fn branch(&mut self, field: Condition, depth: usize, listed: bool) -> Result<Node, Failure> {
        let (yes, stop) = self.nodes(depth + 1, listed)?;
        let (no, end) = if stop == Stop::Else {
            self.nodes(depth + 1, listed)?
        } else {
            (Vec::new(), stop)
        };
        if end != Stop::If {
            return Err(Failure::Payload);
        }
        Ok(Node::If(field, yes, no))
    }
}
