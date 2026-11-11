use super::{Field, Node};
use unicode_width::UnicodeWidthChar;

pub trait Context: Sized {
    fn field(&self, field: Field) -> String;
    fn windows(&self) -> Vec<Self>;
    fn width(&self) -> usize {
        8192
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Tone {
    Base,
    Active,
    Inactive,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Span {
    pub text: String,
    pub tone: Tone,
}

pub(super) fn render<C: Context>(nodes: &[Node], context: &C) -> Vec<Span> {
    segment(nodes, context, Tone::Base)
}

pub(super) fn segment<C: Context>(nodes: &[Node], context: &C, tone: Tone) -> Vec<Span> {
    let mut sink = Sink {
        spans: Vec::new(),
        tone,
        remaining: 8192,
        columns: context.width().min(8192),
    };
    sink.nodes(nodes, context);
    sink.spans
}

struct Sink {
    spans: Vec<Span>,
    tone: Tone,
    remaining: usize,
    columns: usize,
}

impl Sink {
    fn nodes<C: Context>(&mut self, nodes: &[Node], context: &C) {
        for node in nodes {
            if self.remaining == 0 || self.columns == 0 {
                break;
            }
            self.node(node, context);
        }
    }

    fn node<C: Context>(&mut self, node: &Node, context: &C) {
        match node {
            Node::Text(text) => self.text(text),
            Node::Field(field, limit) => self.field(&context.field(*field), *limit),
            Node::Style(tone) => self.tone = *tone,
            Node::If(condition, yes, no) => {
                let nodes = if condition.test(context) { yes } else { no };
                self.nodes(nodes, context);
            }
            Node::Windows(nodes) => {
                for span in super::list::windows(nodes, context, self.columns, self.tone) {
                    self.tone = span.tone;
                    self.text(&span.text);
                }
            }
        }
    }

    fn field(&mut self, text: &str, limit: Option<usize>) {
        let available = self.columns;
        self.columns = available.min(limit.unwrap_or(available));
        let budget = self.columns;
        self.text(text);
        self.columns = available - (budget - self.columns);
    }

    fn text(&mut self, text: &str) {
        let mut value = String::new();
        for c in text.chars().map(|c| if c.is_control() { '?' } else { c }) {
            let width = c.width().unwrap_or(0);
            if c.len_utf8() > self.remaining || width > self.columns {
                break;
            }
            value.push(c);
            self.remaining -= c.len_utf8();
            self.columns -= width;
        }
        if value.is_empty() {
            return;
        }
        if let Some(last) = self.spans.last_mut()
            && last.tone == self.tone
        {
            last.text.push_str(&value);
        } else {
            self.spans.push(Span {
                text: value,
                tone: self.tone,
            });
        }
    }
}
