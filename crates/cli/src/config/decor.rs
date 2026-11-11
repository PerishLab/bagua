use crate::error::Error;
use workspace::Edge;

pub(super) fn decor(config: &bagua::config::Config) -> Result<client::Decor, Error> {
    Ok(client::Decor {
        header: header(&config.header)?,
        status: status(&config.status)?,
    })
}

fn edge(value: &str) -> Result<Edge, Error> {
    match value {
        "top" => Ok(Edge::Top),
        "bottom" => Ok(Edge::Bottom),
        _ => Err(Error::config(
            "header.position/status.position: expected top or bottom",
        )),
    }
}

fn header(config: &bagua::config::Header) -> Result<client::Header, Error> {
    if config.components.is_some() && config.format.is_some() {
        return Err(Error::config(
            "header: components and format cannot be combined",
        ));
    }
    let mut value = match &config.components {
        Some(parts) => client::Header::new(parts),
        None => client::Header::format(config.format.as_deref()),
    }
    .map_err(|_| Error::config("header: invalid components or bounded template"))?;
    value.enabled = config.enabled;
    value.edge = edge(&config.position)?;
    value.border = config.border.unwrap_or(value.border);
    value.color = config.color;
    value.active = config.active.unwrap_or(value.active);
    value.inactive = config.inactive.unwrap_or(value.inactive);
    value.foreground = config.foreground.or(value.foreground);
    value.highlight = config.highlight.or(value.highlight);
    Ok(value)
}

fn status(config: &bagua::config::Status) -> Result<client::Status, Error> {
    if config.components.is_some() && config.lines.is_some() {
        return Err(Error::config(
            "status: components and lines cannot be combined",
        ));
    }
    let mut value = match &config.components {
        Some(parts) => client::Status::new(parts, &config.timezone, config.refresh),
        None => {
            let lines = lines(config)?;
            client::Status::format(lines, &config.timezone, config.refresh)
        }
    }.map_err(|_| Error::config("status: invalid components, bounded templates, line count, timezone or refresh_seconds"))?;
    value.edge = edge(&config.position)?;
    value.color = config.color;
    value.active = config.active.unwrap_or(value.active);
    value.inactive = config.inactive.unwrap_or(value.inactive);
    value.foreground = config.foreground.or(value.foreground);
    value.highlight = config.highlight.or(value.highlight);
    value.separator = config.separator;
    Ok(value)
}

fn lines(config: &bagua::config::Status) -> Result<Option<Vec<client::Line>>, Error> {
    let Some(lines) = &config.lines else {
        return Ok(None);
    };
    if !(1..=4).contains(&lines.len()) {
        return Err(Error::config("status.lines: expected one to four lines"));
    }
    lines
        .iter()
        .map(|source| {
            let mut line = client::Line::new(&source.left, &source.center, &source.right)
                .map_err(|_| Error::config("status.lines: invalid or oversized template"))?;
            line.fill = source.fill;
            Ok(line)
        })
        .collect::<Result<Vec<_>, Error>>()
        .map(Some)
}
