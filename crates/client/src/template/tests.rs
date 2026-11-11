use super::*;

#[derive(Clone)]
struct Data {
    value: String,
    index: usize,
    items: usize,
    width: usize,
    selected: usize,
}

impl Context for Data {
    fn field(&self, field: Field) -> String {
        match field {
            Field::Window => self.index.to_string(),
            Field::Columns => self.width.to_string(),
            Field::Selected => if self.index == self.selected { "1" } else { "" }.into(),
            _ => self.value.clone(),
        }
    }
    fn width(&self) -> usize {
        self.width
    }
    fn windows(&self) -> Vec<Self> {
        (1..=self.items)
            .map(|index| Self {
                index,
                ..self.clone()
            })
            .collect()
    }
}

fn data(value: &str) -> Data {
    Data {
        value: value.into(),
        index: 0,
        items: 2,
        width: 8192,
        selected: 2,
    }
}

fn text(spans: &[Span]) -> String {
    spans.iter().map(|span| span.text.as_str()).collect()
}

#[test]
fn windows() {
    let template = Template::compile("{{#windows}}{{#if window.active}}{{style active}}[{{window.index}}]{{else}}{{style inactive}}{{window.index}}{{/if}} {{/windows}}").unwrap();
    let spans = template.render(&data("shell"));
    assert_eq!(text(&spans), "1 [2] ");
    assert_eq!(spans[0].tone, Tone::Inactive);
    assert_eq!(spans[1].tone, Tone::Active);
    assert!(
        Template::compile("{{#if pane.command}}yes{{else}}none{{/if}}")
            .unwrap()
            .metadata()
    );
    assert_eq!(
        text(
            &Template::compile("{{#if pane.command}}yes{{else}}none{{/if}}")
                .unwrap()
                .render(&data(""))
        ),
        "none"
    );
}

#[test]
fn invalid() {
    for source in [
        "{{unknown}}",
        "{{pane.command",
        "{{/if}}",
        "{{else}}",
        "{{#if pane.active}}",
        "{{#windows}}{{/if}}",
        "{{#windows}}{{#windows}}{{/windows}}{{/windows}}",
        "{{style shell}}",
        "\x1b[31m",
        "one\ntwo",
    ] {
        assert!(Template::compile(source).is_err(), "{source:?}");
    }
    assert!(Template::compile(&"x".repeat(4097)).is_err());
    assert!(Template::compile(&"{{pane.command}}".repeat(129)).is_err());
    let deep = "{{#if pane.active}}".repeat(9) + &"{{/if}}".repeat(9);
    assert!(Template::compile(&deep).is_err());
}

#[test]
fn inert() {
    let template = Template::compile("{{pane.command}}").unwrap();
    let spans = template.render(&data("{{style active}}\x1b[31m$(id)"));
    assert_eq!(text(&spans), "{{style active}}?[31m$(id)");
    assert!(spans.iter().all(|span| span.tone == Tone::Base));
    assert_eq!(
        text(
            &Template::compile("{{{{session.name}}")
                .unwrap()
                .render(&data("ignored"))
        ),
        "{{session.name}}"
    );
}

#[test]
fn bounded() {
    let template = Template::compile("{{pane.command}}").unwrap();
    let spans = template.render(&Data {
        value: "界".repeat(8192),
        index: 0,
        items: 1024,
        width: 8192,
        selected: 2,
    });
    let value = text(&spans);
    assert_eq!(value.len(), 8190);
    assert!(value.chars().all(|c| c == '界'));
    assert_eq!(
        text(
            &Template::compile("{{#windows}}x{{/windows}}")
                .unwrap()
                .render(&Data {
                    items: 40,
                    ..data("")
                })
        ),
        "x".repeat(32)
    );
}

#[test]
fn selected() {
    use unicode_width::UnicodeWidthStr;
    let template = Template::compile("{{#windows}}[{{window.index}}]{{/windows}}").unwrap();
    let value = text(&template.render(&Data {
        width: 12,
        selected: 17,
        items: 32,
        ..data("")
    }));
    assert!(value.contains("[17]"));
    assert!(value.width() <= 12);
    assert!(value.starts_with("<"));
}

#[test]
fn widths() {
    let template = Template::compile(
        "{{pane.command:4}}!{{#if client.columns >= 64}}wide{{else}}compact{{/if}}",
    )
    .unwrap();
    assert_eq!(
        text(&template.render(&Data {
            width: 12,
            ..data("甲乙丙")
        })),
        "甲乙!compact"
    );
    assert_eq!(
        text(&template.render(&Data {
            width: 80,
            ..data("甲乙丙")
        })),
        "甲乙!wide"
    );
    for source in [
        "{{pane.command:513}}",
        "{{pane.command:-1}}",
        "{{pane.command:x}}",
        "{{#if session.name >= 64}}x{{/if}}",
        "{{#if client.columns > 64}}x{{/if}}",
        "{{#if client.columns >= 64 && pane.active}}x{{/if}}",
    ] {
        assert!(Template::compile(source).is_err());
    }
}
