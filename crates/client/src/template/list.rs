use super::{Context, Field, Node, Span, Tone, render};
use unicode_width::UnicodeWidthStr;

pub(super) fn windows<C: Context>(
    nodes: &[Node],
    context: &C,
    columns: usize,
    tone: Tone,
) -> Vec<Span> {
    let items: Vec<_> = context
        .windows()
        .into_iter()
        .take(32)
        .map(|window| {
            let active = !window.field(Field::Selected).is_empty();
            (render::segment(nodes, &window, tone), active)
        })
        .collect();
    let widths: Vec<_> = items
        .iter()
        .map(|(spans, _)| spans.iter().map(|s| s.text.width()).sum::<usize>())
        .collect();
    let selected = items.iter().position(|(_, active)| *active).unwrap_or(0);
    let budget = if columns >= 4 { columns - 4 } else { columns };
    let mut start = selected;
    let mut end = (selected + 1).min(items.len());
    let mut used = widths.get(selected).copied().unwrap_or(0).min(budget);
    while start > 0 && used + widths[start - 1] <= budget {
        start -= 1;
        used += widths[start];
    }
    while end < items.len() && used + widths[end] <= budget {
        used += widths[end];
        end += 1;
    }
    let mut output = Vec::new();
    if start > 0 && columns >= 4 {
        output.push(Span {
            text: "< ".into(),
            tone,
        });
    }
    for (spans, _) in &items[start..end] {
        output.extend(spans.iter().cloned());
    }
    if end < items.len() && columns >= 4 {
        output.push(Span {
            text: " >".into(),
            tone,
        });
    }
    output
}
