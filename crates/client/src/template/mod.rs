mod condition;
mod field;
use condition::Condition;
mod list;
mod parse;
mod render;
#[cfg(test)]
mod tests;

pub use field::Field;
use protocol::Failure;
pub use render::{Context, Span, Tone};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Template {
    nodes: Vec<Node>,
    source: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
enum Node {
    Text(String),
    Field(Field, Option<usize>),
    Style(Tone),
    If(Condition, Vec<Node>, Vec<Node>),
    Windows(Vec<Node>),
}

impl Template {
    pub fn compile(source: &str) -> Result<Self, Failure> {
        parse::compile(source).map(|nodes| Self {
            nodes,
            source: source.into(),
        })
    }

    pub fn render<C: Context>(&self, context: &C) -> Vec<Span> {
        render::render(&self.nodes, context)
    }

    pub fn source(&self) -> &str {
        &self.source
    }

    pub fn metadata(&self) -> bool {
        metadata(&self.nodes)
    }
}

fn metadata(nodes: &[Node]) -> bool {
    nodes.iter().any(|node| match node {
        Node::Field(field, _) => field.metadata(),
        Node::If(condition, yes, no) => condition.field.metadata() || metadata(yes) || metadata(no),
        Node::Windows(nodes) => metadata(nodes),
        _ => false,
    })
}
